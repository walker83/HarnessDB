mod handler_struct;
mod query_executor;
mod ddl_handler;
mod dml_handler;
mod utils;
mod connection_tracker;
mod web;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use fe_common::edit_log::EditLog;
use fe_catalog::CatalogManager;
use fe_monitor::MonitoringManager;
use fe_monitor::audit_log::{AuditLogger, AuditLogConfig, AuditLogEntry, QueryType, QueryStatus};
use fe_config::RorisConfig;
use fe_config::SystemVariableManager;
use fe_backup::BackupManager;
use mysql_protocol::{auth::AuthPluginType, MysqlServer, QueryHandler, QueryResult, ServerConfig};
use mysql_protocol::server::{ColumnDef, ColumnType};
use fe_sql_parser::{parse_sql, is_dml_sql};

use handler_struct::RorisQueryHandler;
use connection_tracker::ConnectionTracker;
use utils::record_batches_to_query_result_with_df_schema;
use web::{WebState, start_web_server};

#[derive(Parser)]
#[command(name = "roris-fe", about = "Roris Frontend Server")]
struct Args {
    #[arg(long, default_value = "data/fe/doris-meta")]
    meta_dir: String,

    #[arg(long, default_value = "9030")]
    mysql_port: u16,

    #[arg(long, default_value = "data/fe/storage")]
    data_dir: String,

    #[arg(long, default_value = "roris.toml")]
    config_file: String,
}

impl QueryHandler for RorisQueryHandler {
    fn handle_query(&self, sql: &str) -> QueryResult {
        tracing::info!("handle_query received SQL: {:?}", sql);
        let trimmed = sql.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            return QueryResult::ok();
        }

        let start = Instant::now();

        // Track query
        self.connection_tracker.record_query();
        self.connection_tracker.query_start();

        let result = if is_dml_sql(trimmed) {
            let upper = trimmed.to_uppercase();
            if upper.starts_with("INSERT") {
                // Fall through to parse_sql path
                self.dispatch_parsed(trimmed)
            } else if upper.starts_with("DELETE") || upper.starts_with("UPDATE") {
                // DELETE and UPDATE also fall through to parse_sql path
                self.dispatch_parsed(trimmed)
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
                        record_batches_to_query_result_with_df_schema(&batches, &df_schema)
                    }
                    Ok(Err(df_err)) => {
                        tracing::error!("DataFusion error: {}", df_err);
                        QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: {}", df_err))]],
                        )
                    }
                    Err(e) => {
                        tracing::error!("Thread error: {:?}", e);
                        QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: thread panicked"))]],
                        )
                    }
                }
            }
        } else {
            self.dispatch_parsed(trimmed)
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        self.connection_tracker.query_end();

        // Audit logging (fire and forget)
        let has_error = result.columns.iter().any(|c| c.name == "Error");
        let query_type = QueryType::from_sql(trimmed);
        let status = if has_error { QueryStatus::Failed } else { QueryStatus::Success };
        let error_msg = if has_error {
            result.rows.first().and_then(|r| r.first()).and_then(|v| v.clone())
        } else {
            None
        };

        // Check slow query
        let slow_threshold = self.sys_vars.get("slow_query_threshold", None)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1000);
        if duration_ms >= slow_threshold {
            self.connection_tracker.record_slow_query();
        }

        let audit_enabled = self.sys_vars.get("enable_audit_log", None)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        if audit_enabled {
            let slow_only = self.sys_vars.get("audit_log_slow_only", None)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);

            if !slow_only || duration_ms >= slow_threshold {
                let entry = AuditLogEntry {
                    timestamp: chrono::Utc::now(),
                    user: "root".to_string(),
                    host: "127.0.0.1".to_string(),
                    database: Some(self.current_database.read().clone()),
                    query: trimmed.to_string(),
                    query_type,
                    status,
                    duration_ms,
                    rows_affected: None,
                    bytes_scanned: None,
                    error_message: error_msg,
                };
                let audit = self.audit_logger.clone();
                tokio::spawn(async move {
                    audit.log_entry(entry).await;
                });
            }
        }

        result
    }

    fn set_database(&self, db: &str) {
        {
            let mut current_db = self.current_database.write();
            *current_db = db.to_string();
        }
        self.session_ctx.state_ref().write().config_mut().options_mut().catalog.default_schema = db.to_string();
    }

    fn on_connect(&self, conn_id: u32, user: &str, host: &str) {
        let db = self.current_database.read().clone();
        self.connection_tracker.register(conn_id, user, host, db);
    }

    fn on_disconnect(&self, conn_id: u32) {
        self.connection_tracker.unregister(conn_id);
    }
}

impl RorisQueryHandler {
    fn dispatch_parsed(&self, trimmed: &str) -> QueryResult {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("RorisDB starting...");

    // Load configuration
    let mut config = RorisConfig::load_or_default(&args.config_file);
    config.apply_cli_overrides(
        Some(args.mysql_port),
        Some(args.data_dir.clone()),
        Some(args.meta_dir.clone()),
    );
    tracing::info!("Config loaded: mysql_port={}, data_dir={}, http_port={}",
        config.server.mysql_port, config.storage.data_dir, config.server.http_port);

    // Initialize system variables
    let sys_vars = Arc::new(SystemVariableManager::new());
    // Apply config values to system variables
    let _ = sys_vars.set_global("max_connections", &config.server.max_connections.to_string());
    let _ = sys_vars.set_global("wait_timeout", &config.server.wait_timeout.to_string());
    let _ = sys_vars.set_global("http_port", &config.server.http_port.to_string());
    let _ = sys_vars.set_global("storage_compression", &config.storage.compression);
    let _ = sys_vars.set_global("enable_audit_log", &config.logging.enable_audit_log.to_string());
    let _ = sys_vars.set_global("slow_query_threshold", &config.logging.slow_query_threshold_ms.to_string());
    let _ = sys_vars.set_global("audit_log_slow_only", &config.logging.audit_log_slow_only.to_string());
    let _ = sys_vars.set_global("query_timeout", &config.query.query_timeout.to_string());
    let _ = sys_vars.set_global("max_allowed_packet", &config.query.max_allowed_packet.to_string());
    let _ = sys_vars.set_global("sql_mode", &config.query.sql_mode);
    let _ = sys_vars.set_global("time_zone", &config.query.time_zone);

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

    let monitoring = Arc::new(MonitoringManager::new());

    // Create audit logger with config
    let audit_config = AuditLogConfig {
        enabled: config.logging.enable_audit_log,
        log_dir: std::path::PathBuf::from(&config.logging.audit_log_dir),
        max_file_size_mb: config.logging.audit_log_max_size_mb as usize,
        max_files: config.logging.audit_log_max_files as usize,
        log_queries: true,
        log_slow_queries_only: config.logging.audit_log_slow_only,
        slow_query_threshold_ms: config.logging.slow_query_threshold_ms,
    };
    let audit_logger = Arc::new(AuditLogger::with_config(audit_config));

    // Create connection tracker
    let connection_tracker = Arc::new(ConnectionTracker::new());

    // Create backup manager
    let backup_manager = Arc::new(BackupManager::new(&args.meta_dir, &config.storage.data_dir));

    let query_handler = Arc::new(RorisQueryHandler::new(
        catalog.clone(),
        config.clone(),
        sys_vars,
        audit_logger.clone(),
        connection_tracker.clone(),
        backup_manager,
    ));

    let mysql_config = ServerConfig {
        bind_addr: config.server.bind_addr.clone(),
        port: config.server.mysql_port,
        default_auth_plugin: AuthPluginType::NativePassword,
        auth_timeout_secs: 30,
    };
    let mysql_server = MysqlServer::new(mysql_config, query_handler.clone());

    tokio::spawn(async move {
        match mysql_server.run().await {
            Ok(()) => tracing::info!("MySQL server stopped"),
            Err(e) => tracing::error!("MySQL server failed: {}", e),
        }
    });

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

    tracing::info!("RorisDB MySQL server started on port {}", config.server.mysql_port);

    // Start Web SQL Editor if enabled
    if config.server.http_port > 0 {
        let web_state = Arc::new(WebState::new(
            query_handler.clone(),
            connection_tracker.clone(),
        ));
        let http_port = config.server.http_port;
        tokio::spawn(async move {
            if let Err(e) = start_web_server(web_state, http_port).await {
                tracing::error!("Web server failed: {}", e);
            }
        });
        tracing::info!("SQL Editor available at http://127.0.0.1:{}", config.server.http_port);
    }

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

    audit_logger.flush().await;
    tracing::info!("Audit logs flushed");

    catalog.save()?;
    tracing::info!("Catalog saved on shutdown");

    Ok(())
}
