mod handler_struct;
mod query_executor;
mod ddl_handler;
mod dml_handler;
mod utils;
mod connection_tracker;
mod metrics;
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
use maxcompute_protocol::{start_mc_server, McServerConfig, McServerState};
use pg_protocol::{PgServer, PgServerConfig};
use fe_sql_parser::{parse_sql, is_dml_sql};
use fe_storage::{ParquetCatalogProvider, InformationSchemaProvider};
use fe_datafusion::{register_doris_udfs, register_misc_udfs};
use datafusion::prelude::{SessionConfig, SessionContext};

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

    #[arg(long, default_value = "9031")]
    maxcompute_port: u16,

    #[arg(long, default_value = "5432")]
    hologres_port: u16,

    #[arg(long, default_value = "data/fe/storage")]
    data_dir: String,

    #[arg(long, default_value = "roris.toml")]
    config_file: String,
}

impl QueryHandler for RorisQueryHandler {
    fn handle_query(&self, conn_id: u32, sql: &str) -> QueryResult {
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
                self.dispatch_parsed(conn_id, trimmed)
            } else if upper.starts_with("DELETE") || upper.starts_with("UPDATE") {
                // DELETE and UPDATE also fall through to parse_sql path
                self.dispatch_parsed(conn_id, trimmed)
            } else {
                // Other DML (like SELECT) goes to DataFusion
                // Create a per-query context with the correct default schema for this connection
                let current_db = self.get_session(conn_id);
                let result = self.run_datafusion({
                    let catalog = self.catalog.clone();
                    let storage = self.storage.clone();
                    let sql = trimmed.to_string();
                    move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            let df_catalog = Arc::new(ParquetCatalogProvider::new(catalog, storage));
                            let df_config = SessionConfig::new()
                                .with_default_catalog_and_schema("roris", &current_db)
                                .with_create_default_catalog_and_schema(false)
                                .with_information_schema(false); // Use custom information_schema from ParquetCatalogProvider
                            let mut ctx = SessionContext::new_with_config(df_config);
                            ctx.register_catalog("roris", df_catalog);
                            register_doris_udfs(&mut ctx);
                            register_misc_udfs(&mut ctx);
                            fe_datafusion::register_date_udfs(&mut ctx);
                            let df = ctx.sql(&sql).await.map_err(|e| e.to_string())?;
                            let schema = df.schema().clone();
                            let batches = df.collect().await.map_err(|e| e.to_string())?;
                            Ok::<_, String>((batches, schema))
                        })
                    }
                });

                match result {
                    Ok((batches, df_schema)) => {
                        record_batches_to_query_result_with_df_schema(&batches, &df_schema)
                    }
                    Err(e) => {
                        tracing::error!("DataFusion error: {}", e);
                        QueryResult::with_rows(
                            vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                            vec![vec![Some(format!("ERROR: {}", e))]],
                        )
                    }
                }
            }
        } else {
            self.dispatch_parsed(conn_id, trimmed)
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
        let is_slow = duration_ms >= slow_threshold;
        if is_slow {
            self.connection_tracker.record_slow_query();
        }

        // Record Prometheus metrics
        crate::metrics::record_query(
            trimmed,
            duration_ms as f64,
            is_slow,
            has_error,
        );

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
                    database: Some(self.get_session(conn_id)),
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

    fn set_database(&self, conn_id: u32, db: &str) {
        self.set_current_database(conn_id, db.to_string());
        // No longer modify shared session_ctx - per-query contexts use get_session(conn_id)
    }

    fn on_connect(&self, conn_id: u32, user: &str, host: &str) {
        let db = self.get_session(conn_id);
        self.connection_tracker.register(conn_id, user, host, db);
    }

    fn on_disconnect(&self, conn_id: u32) {
        self.connection_tracker.unregister(conn_id);
        self.remove_session(conn_id);
    }
}

impl RorisQueryHandler {
    fn dispatch_parsed(&self, conn_id: u32, trimmed: &str) -> QueryResult {
        match parse_sql(trimmed) {
            Ok(statements) => {
                if statements.is_empty() {
                    return QueryResult::ok();
                }
                // Execute all statements, return the last non-OK result or the final result
                let mut last_result = QueryResult::ok();
                for stmt in &statements {
                    match self.execute_statement(conn_id, stmt) {
                        Ok(result) => {
                            // If this statement produced a result set (rows), return it immediately
                            // This handles multi-statement queries where a SELECT is the last statement
                            if !result.columns.is_empty() {
                                return result;
                            }
                            last_result = result;
                        }
                        Err(e) => {
                            tracing::error!("Query error: {}", e);
                            return QueryResult::with_rows(
                                vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                                vec![vec![Some(format!("ERROR: {}", e))]],
                            );
                        }
                    }
                }
                last_result
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
        max_connections: config.server.max_connections,
    };
    let mysql_server = MysqlServer::new(mysql_config, query_handler.clone());

    tokio::spawn(async move {
        match mysql_server.run().await {
            Ok(()) => tracing::info!("MySQL server stopped"),
            Err(e) => tracing::error!("MySQL server failed: {}", e),
        }
    });

    // Start MaxCompute protocol server (HTTP/REST on port 9031)
    let mc_config = McServerConfig {
        bind_addr: config.server.bind_addr.clone(),
        port: args.maxcompute_port,
        access_key_id: "roris".to_string(),
        access_key_secret: "roris-secret".to_string(),
        default_project: "default".to_string(),
        region: None,
    };
    let mc_handler = query_handler.clone();
    tokio::spawn(async move {
        if let Err(e) = start_mc_server(mc_handler, mc_config).await {
            tracing::error!("MaxCompute server failed: {}", e);
        }
    });

    // Start Hologres protocol server (PostgreSQL wire protocol on port 5432)
    let pg_config = PgServerConfig {
        bind_addr: config.server.bind_addr.clone(),
        port: args.hologres_port,
        max_connections: config.server.max_connections,
        username: "roris".to_string(),
        password: "roris-secret".to_string(),
        accept_any_password: false,
    };
    let pg_server = PgServer::new(pg_config, query_handler.clone());
    tokio::spawn(async move {
        if let Err(e) = pg_server.run().await {
            tracing::error!("Hologres (PG) server failed: {}", e);
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

    tracing::info!("RorisDB servers started: MySQL={}, MaxCompute={}, Hologres={}",
        config.server.mysql_port, args.maxcompute_port, args.hologres_port);

    // Initialize Prometheus server info metric
    crate::metrics::RORIS_SERVER_INFO.set(1.0);

    // Periodic memory usage collection (every 15 seconds)
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            crate::metrics::update_process_memory();
        }
    });

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
