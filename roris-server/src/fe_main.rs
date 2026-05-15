mod handler_struct;
mod query_executor;
mod ddl_handler;
mod dml_handler;
mod utils;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use fe_common::edit_log::EditLog;
use fe_catalog::CatalogManager;
use fe_monitor::MonitoringManager;
use fe_monitor::http_server::MonitoringHttpServer;
use mysql_protocol::{auth::AuthPluginType, MysqlServer, QueryHandler, QueryResult, ServerConfig};
use mysql_protocol::server::{ColumnDef, ColumnType};
use fe_sql_parser::{parse_sql, is_dml_sql};

use handler_struct::RorisQueryHandler;
use utils::{record_batches_to_query_result_with_df_schema};

#[derive(Parser)]
#[command(name = "roris-fe", about = "Roris Frontend Server")]
struct Args {
    #[arg(long, default_value = "conf/fe.conf")]
    config: String,

    #[arg(long, default_value = "8030")]
    http_port: u16,

    #[arg(long, default_value = "9020")]
    rpc_port: u16,

    #[arg(long, default_value = "data/fe/doris-meta")]
    meta_dir: String,

    #[arg(long, default_value = "8040")]
    metrics_port: u16,

    #[arg(long, default_value = "9030")]
    mysql_port: u16,

    #[arg(long, default_value = "data/fe/storage")]
    data_dir: String,
}

impl QueryHandler for RorisQueryHandler {
    fn handle_query(&self, sql: &str) -> QueryResult {
        tracing::info!("handle_query received SQL: {:?}", sql);
        let trimmed = sql.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            return QueryResult::ok();
        }

        if is_dml_sql(trimmed) {
            let upper = trimmed.to_uppercase();
            if upper.starts_with("INSERT") {
                // Fall through to parse_sql path
            } else if upper.starts_with("DELETE") || upper.starts_with("UPDATE") {
                // DELETE and UPDATE also fall through to parse_sql path
            } else {
                // Other DML (like SELECT with INTO) goes to DataFusion
                let result = std::thread::spawn({
                    let ctx = self.session_ctx.clone();
                    let sql = trimmed.to_string();
                    move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            let df = ctx.sql(&sql).await?;
                            let schema = df.schema().clone();
                            let batches = df.collect().await?;
                            Ok::<_, datafusion::error::DataFusionError>((batches, schema))
                        })
                    }
                })
                .join();

                match result {
                    Ok(Ok((batches, df_schema))) => {
                        return record_batches_to_query_result_with_df_schema(&batches, &df_schema);
                    }
                    Ok(Err(df_err)) => {
                        tracing::error!("DataFusion error: {}", df_err);
                        return QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: {}", df_err))]],
                        );
                    }
                    Err(e) => {
                        tracing::error!("Thread error: {:?}", e);
                        return QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: thread panicked"))]],
                        );
                    }
                }
            }
        }

        match parse_sql(trimmed) {
            Ok(statements) => {
                if statements.is_empty() {
                    return QueryResult::ok();
                }
                let stmt = &statements[0];
                match self.execute_statement(stmt) {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("Query error: {}", e);
                        QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: {}", e))]],
                        )
                    }
                }
            }
            Err(e) => {
                tracing::error!("Parse error: {}", e);
                QueryResult::with_rows(
                    vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                    vec![vec![Some(format!("PARSE ERROR: {}", e))]],
                )
            }
        }
    }

    fn set_database(&self, db: &str) {
        {
            let mut current_db = self.current_database.write();
            *current_db = db.to_string();
        }
        self.session_ctx.state_ref().write().config_mut().options_mut().catalog.default_schema = db.to_string();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Roris FE starting...");
    tracing::info!("Config file: {}", args.config);
    tracing::info!("HTTP port: {}, RPC port: {}", args.http_port, args.rpc_port);
    tracing::info!("Meta directory: {}", args.meta_dir);
    tracing::info!("Metrics port: {}", args.metrics_port);
    tracing::info!("MySQL port: {}", args.mysql_port);

    let catalog = Arc::new(CatalogManager::with_path(&args.meta_dir));
    {
        catalog.load()?;
        tracing::info!("Catalog loaded from disk");
    }

    let edit_log = Arc::new(RwLock::new(EditLog::new(&args.meta_dir)));
    {
        let mut log = edit_log.write().await;
        log.replay().await?;
        tracing::info!("Edit log replayed, last_applied_index: {}", log.last_applied_index());
    }

    {
        let log = edit_log.read().await;
        catalog.replay_edit_log(&log)?;
        tracing::info!("Edit log applied to catalog");
    }

    let monitoring = Arc::new(MonitoringManager::new(catalog.clone()));
    tracing::info!("Monitoring manager initialized");

    let http_server = MonitoringHttpServer::new(args.metrics_port, monitoring.clone());
    tokio::spawn(async move {
        if let Err(e) = http_server.start().await {
            tracing::error!("Monitoring HTTP server failed: {}", e);
        }
    });
    tracing::info!("Monitoring HTTP server started on port {}", args.metrics_port);

    let query_handler = RorisQueryHandler::new(catalog.clone(), args.data_dir.clone());
    let mysql_config = ServerConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: args.mysql_port,
        default_auth_plugin: AuthPluginType::NativePassword,
        auth_timeout_secs: 30,
    };
    let mysql_server = MysqlServer::new(mysql_config, Arc::new(query_handler));
    tracing::info!("MySQL server starting on port {}", args.mysql_port);

    tokio::spawn(async move {
        tracing::info!("MySQL server task started");
        match mysql_server.run().await {
            Ok(()) => tracing::info!("MySQL server stopped"),
            Err(e) => tracing::error!("MySQL server failed: {}", e),
        }
    });
    tracing::info!("MySQL server spawn completed on port {}", args.mysql_port);

    let edit_log_clone = edit_log.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let mut log = edit_log_clone.write().await;
            if let Err(e) = log.flush().await {
                tracing::error!("EditLog flush failed: {}", e);
            } else {
                tracing::debug!("EditLog flushed");
            }
        }
    });

    tracing::info!("Roris FE started successfully");

    let catalog_clone = catalog.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if let Err(e) = catalog_clone.save() {
                tracing::error!("Catalog save failed: {}", e);
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris FE shutting down...");

    monitoring.audit_log.flush().await;
    tracing::info!("Audit logs flushed");

    catalog.save()?;
    tracing::info!("Catalog saved on shutdown");

    Ok(())
}
