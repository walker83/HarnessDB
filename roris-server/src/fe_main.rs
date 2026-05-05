use anyhow::Result;
use clap::Parser;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use fe_common::edit_log::EditLog;
use fe_catalog::CatalogManager;
use fe_scheduler::ClusterManager;
use fe_monitor::MonitoringManager;
use fe_monitor::http_server::MonitoringHttpServer;
use mysql_protocol::{auth::AuthPluginType, MysqlServer, QueryHandler, QueryResult, ServerConfig};
use mysql_protocol::server::{ColumnDef, ColumnType};
use fe_sql_planner::{Planner, Optimizer};
use fe_sql_parser::{parse_sql, Statement};
use fe_sql_parser::ast::{CreateDatabaseStmt, CreateTableStmt, DropDatabaseStmt, DropTableStmt};
use types::DataType;

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
}

struct RorisQueryHandler {
    catalog: Arc<StdRwLock<CatalogManager>>,
    current_database: Arc<StdRwLock<String>>,
}

impl RorisQueryHandler {
    fn new(catalog: Arc<StdRwLock<CatalogManager>>) -> Self {
        Self {
            catalog,
            current_database: Arc::new(StdRwLock::new("information_schema".to_string())),
        }
    }
}

impl QueryHandler for RorisQueryHandler {
    fn handle_query(&self, sql: &str) -> QueryResult {
        let trimmed = sql.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            return QueryResult::ok();
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
}

impl RorisQueryHandler {
    fn execute_statement(&self, stmt: &Statement) -> Result<QueryResult, String> {
        match stmt {
            Statement::ShowDatabases => self.show_databases(),
            Statement::ShowTables(db) => self.show_tables(db.clone()),
            Statement::Describe(db, table) => self.describe(db.clone(), table.clone()),
            Statement::ShowColumns(db, table) => self.show_columns(db.clone(), table.clone()),
            Statement::UseDatabase(db) => self.use_database(db),
            Statement::CreateDatabase(stmt) => self.create_database(stmt),
            Statement::CreateTable(stmt) => self.create_table(stmt),
            Statement::DropDatabase(stmt) => self.drop_database(stmt),
            Statement::DropTable(stmt) => self.drop_table(stmt),
            Statement::Query(_) => self.execute_query(stmt),
            Statement::Explain(explain) => self.explain(&explain.statement),
            _ => Ok(QueryResult::with_rows(
                vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
                vec![vec![Some(format!("Statement parsed successfully (execution not fully implemented)"))]],
            )),
        }
    }

    fn show_databases(&self) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let databases = catalog.list_databases();
        let rows: Vec<Vec<Option<String>>> = databases
            .iter()
            .map(|db| vec![Some(db.clone())])
            .collect();
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Database".to_string(), col_type: ColumnType::String }],
            rows,
        ))
    }

    fn show_tables(&self, db: Option<String>) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = db.as_deref().unwrap_or(&current_db);

        match catalog.list_tables(target_db) {
            Some(tables) => {
                let rows: Vec<Vec<Option<String>>> = tables
                    .iter()
                    .map(|t| vec![Some(t.clone())])
                    .collect();
                Ok(QueryResult::with_rows(
                    vec![ColumnDef { name: "Tables".to_string(), col_type: ColumnType::String }],
                    rows,
                ))
            }
            None => Err(format!("Database '{}' not found", target_db)),
        }
    }

    fn describe(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                let rows: Vec<Vec<Option<String>>> = tbl.columns.iter().map(|col| {
                    vec![
                        Some(col.name.clone()),
                        Some(format!("{:?}", col.data_type)),
                        Some(if col.nullable { "YES" } else { "NO" }.to_string()),
                        col.default_value.as_ref().map(|v| format!("{:?}", v)),
                        Some(col.comment.clone()),
                    ]
                }).collect();
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Field".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Type".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Null".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Default".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Comment".to_string(), col_type: ColumnType::String },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Table '{}.{}' not found", target_db, table)),
        }
    }

    fn show_columns(&self, db: Option<String>, table: Option<String>) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = db.as_deref().unwrap_or(&current_db);

        match table {
            Some(tbl) => self.describe(target_db.to_string(), tbl),
            None => {
                match catalog.list_tables(target_db) {
                    Some(tables) => {
                        let mut rows = Vec::new();
                        for table_name in &tables {
                            if let Some(tbl) = catalog.get_table(target_db, table_name) {
                                for col in &tbl.columns {
                                    rows.push(vec![
                                        Some(table_name.clone()),
                                        Some(col.name.clone()),
                                        Some(format!("{:?}", col.data_type)),
                                    ]);
                                }
                            }
                        }
                        Ok(QueryResult::with_rows(
                            vec![
                                ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                                ColumnDef { name: "Field".to_string(), col_type: ColumnType::String },
                                ColumnDef { name: "Type".to_string(), col_type: ColumnType::String },
                            ],
                            rows,
                        ))
                    }
                    None => Err(format!("Database '{}' not found", target_db)),
                }
            }
        }
    }

    fn use_database(&self, db: &str) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        if catalog.get_database(db).is_some() {
            let mut current_db = self.current_database.write().unwrap();
            *current_db = db.to_string();
            Ok(QueryResult::ok())
        } else {
            Err(format!("Unknown database '{}'", db))
        }
    }

    fn create_database(&self, stmt: &CreateDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.write().unwrap();
        match catalog.create_database(&stmt.name) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(e) => {
                if stmt.if_not_exists {
                    Ok(QueryResult::ok())
                } else {
                    Err(format!("{}", e))
                }
            }
        }
    }

    fn create_table(&self, stmt: &CreateTableStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);

        use fe_catalog::table::{Table, TableColumn, KeysType};

        let columns: Vec<TableColumn> = stmt.columns.iter().map(|c| {
            TableColumn {
                name: c.name.clone(),
                data_type: parse_data_type(&c.data_type),
                nullable: c.nullable,
                default_value: None,
                agg_type: c.agg_type.clone(),
                comment: c.comment.clone().unwrap_or_default(),
            }
        }).collect();

        let table = Table {
            id: 0,
            name: stmt.name.clone(),
            database: db.to_string(),
            columns,
            keys_type: KeysType::Duplicate,
            partition_info: None,
            distribution_info: None,
            replication_num: 1,
            properties: std::collections::HashMap::new(),
            row_count: 0,
            data_size: 0,
            stats: None,
        };

        drop(catalog);
        let catalog = self.catalog.write().unwrap();
        match catalog.create_table(db, table) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(e) => {
                if stmt.if_not_exists {
                    Ok(QueryResult::ok())
                } else {
                    Err(format!("{}", e))
                }
            }
        }
    }

    fn drop_database(&self, stmt: &DropDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.write().unwrap();
        match catalog.drop_database(&stmt.name) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn drop_table(&self, stmt: &DropTableStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let table = stmt.name.clone();

        drop(catalog);
        let catalog = self.catalog.write().unwrap();
        match catalog.drop_table(db, &table) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn execute_query(&self, stmt: &Statement) -> Result<QueryResult, String> {
        let _catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let _db = current_db.clone();

        // Create a new CatalogManager instance for the planner
        // In production, this would share state properly
        let catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        let planner = Planner::new(Arc::new(catalog_for_planner));
        let optimizer = Optimizer::new();

        let plan = planner.plan(stmt.clone()).map_err(|e| format!("Planning error: {}", e))?;
        let optimized_plan = optimizer.optimize(plan);

        let explain_output = format_plan(&optimized_plan);

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Query Plan".to_string(), col_type: ColumnType::String },
            ],
            explain_output.lines().map(|line| vec![Some(line.to_string())]).collect(),
        ))
    }

    fn explain(&self, stmt: &Statement) -> Result<QueryResult, String> {
        self.execute_query(stmt)
    }
}

fn format_plan(node: &fe_sql_planner::PlanNode) -> String {
    let mut output = String::new();
    format_plan_recursive(node, &mut output, 0);
    output
}

fn format_plan_recursive(node: &fe_sql_planner::PlanNode, output: &mut String, indent: usize) {
    let prefix = "  ".repeat(indent);
    output.push_str(&format!("{}{:?}\n", prefix, node.node_type));
    for child in &node.children {
        format_plan_recursive(child, output, indent + 1);
    }
}

fn parse_data_type(s: &str) -> DataType {
    match s.to_uppercase().as_str() {
        "INT8" | "TINYINT" => DataType::Int8,
        "INT16" | "SMALLINT" => DataType::Int16,
        "INT32" | "INT" => DataType::Int32,
        "INT64" | "BIGINT" => DataType::Int64,
        "FLOAT32" | "FLOAT" => DataType::Float32,
        "FLOAT64" | "DOUBLE" => DataType::Float64,
        "STRING" | "VARCHAR" | "TEXT" => DataType::String,
        "BOOLEAN" | "BOOL" => DataType::Boolean,
        "DATE" => DataType::Date,
        "DATETIME" | "TIMESTAMP" => DataType::DateTime,
        _ => DataType::String,
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

    // Initialize catalog manager with persistence path
    let catalog = Arc::new(StdRwLock::new(CatalogManager::with_path(&args.meta_dir)));

    // Load catalog from disk if it exists
    {
        let mut catalog_guard = catalog.write().unwrap();
        catalog_guard.load()?;
        tracing::info!("Catalog loaded from disk");
    }

    // Initialize edit log
    let edit_log = Arc::new(RwLock::new(EditLog::new(&args.meta_dir)));

    // Replay edit log on startup
    {
        let mut log = edit_log.write().await;
        log.replay().await?;
        tracing::info!("Edit log replayed, last_applied_index: {}", log.last_applied_index());
    }

    // Apply edit log entries to catalog
    {
        let mut catalog_guard = catalog.write().unwrap();
        let log = edit_log.read().await;
        catalog_guard.replay_edit_log(&log)?;
        tracing::info!("Edit log applied to catalog");
    }

    // Initialize cluster manager for BE health monitoring
    let _cluster = Arc::new(RwLock::new(ClusterManager::new(fe_scheduler::cluster::ClusterConfig::default())));

    // Initialize monitoring manager
    let monitoring = Arc::new(MonitoringManager::new(catalog.clone()));
    tracing::info!("Monitoring manager initialized");

    // Start monitoring HTTP server
    let http_server = MonitoringHttpServer::new(args.metrics_port, monitoring.clone());
    tokio::spawn(async move {
        if let Err(e) = http_server.start().await {
            tracing::error!("Monitoring HTTP server failed: {}", e);
        }
    });
    tracing::info!("Monitoring HTTP server started on port {}", args.metrics_port);

    // Start MySQL protocol server
    let query_handler = RorisQueryHandler::new(catalog.clone());
    let mysql_config = ServerConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: args.mysql_port,
        default_auth_plugin: AuthPluginType::NativePassword,
    };
    let mysql_server = MysqlServer::new(mysql_config, Arc::new(query_handler));
    tracing::info!("MySQL server starting on port {}", args.mysql_port);

    // Spawn MySQL server in background
    tokio::spawn(async move {
        tracing::info!("MySQL server task started");
        match mysql_server.run().await {
            Ok(()) => tracing::info!("MySQL server stopped"),
            Err(e) => tracing::error!("MySQL server failed: {}", e),
        }
    });
    tracing::info!("MySQL server spawn completed on port {}", args.mysql_port);

    // Background task to periodically flush the EditLog
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

    // Periodically save catalog
    let catalog_clone = catalog.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if let Err(e) = catalog_clone.read().unwrap().save() {
                tracing::error!("Catalog save failed: {}", e);
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris FE shutting down...");

    // Flush audit logs before shutdown
    monitoring.audit_log.flush().await;
    tracing::info!("Audit logs flushed");

    // Final save on shutdown
    catalog.read().unwrap().save()?;
    tracing::info!("Catalog saved on shutdown");

    Ok(())
}
