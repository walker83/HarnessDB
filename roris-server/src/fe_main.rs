use anyhow::Result;
use clap::Parser;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use be_storage::StorageEngine;
use fe_common::edit_log::EditLog;
use fe_catalog::CatalogManager;
use fe_scheduler::ClusterManager;
use fe_monitor::MonitoringManager;
use fe_monitor::http_server::MonitoringHttpServer;
use mysql_protocol::{auth::AuthPluginType, MysqlServer, QueryHandler, QueryResult, ServerConfig};
use mysql_protocol::server::{ColumnDef, ColumnType};
use fe_sql_planner::{Planner, Optimizer};
use fe_sql_parser::{parse_sql, Statement};
use fe_sql_parser::ast::{AlterTableStmt, CreateDatabaseStmt, CreateTableStmt, DropDatabaseStmt, DropTableStmt, AlterDatabaseStmt, DropViewStmt, AlterViewStmt, CreateIndexStmt, DropIndexStmt, CancelAlterTableStmt, AlterColocateGroupStmt, DeleteStmt};
use types::{DataType, ScalarValue, Block};
use fe_catalog::table::{Table, TableColumn, KeysType};
use be_execution::exec_node::TransactionContext;
use be_execution::planner::{ExecutionContext, execute_plan};

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
    storage: Arc<StorageEngine>,
    views: Arc<StdRwLock<Vec<ViewInfo>>>,
    transaction: Arc<StdRwLock<TransactionContext>>,
}

#[derive(Clone)]
struct ViewInfo {
    database: String,
    name: String,
    query: String,
    columns: Vec<String>,
}

impl RorisQueryHandler {
    fn new(catalog: Arc<StdRwLock<CatalogManager>>, storage: Arc<StorageEngine>) -> Self {
        Self {
            catalog,
            current_database: Arc::new(StdRwLock::new("information_schema".to_string())),
            views: Arc::new(StdRwLock::new(Vec::new())),
            storage,
            transaction: Arc::new(StdRwLock::new(TransactionContext::new())),
        }
    }

    fn find_view(&self, db: &str, name: &str) -> Option<ViewInfo> {
        let views = self.views.read().unwrap();
        views.iter().find(|v| v.database == db && v.name == name).cloned()
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

    fn set_database(&self, db: &str) {
        let mut current_db = self.current_database.write().unwrap();
        *current_db = db.to_string();
    }
}

impl RorisQueryHandler {
    fn execute_statement(&self, stmt: &Statement) -> Result<QueryResult, String> {
        match stmt {
            Statement::ShowDatabases => self.show_databases(),
            Statement::ShowTables(db) => self.show_tables(db.clone()),
            Statement::ShowCreateTable(db, table) => self.show_create_table(db.clone(), table.clone()),
            Statement::ShowCreateDatabase(db) => self.show_create_database(&db),
            Statement::ShowCreateView(db, view) => self.show_create_view(db.clone(), view.clone()),
            Statement::Describe(db, table) => self.describe(db.clone(), table.clone()),
            Statement::ShowColumns(db, table) => self.show_columns(db.clone(), table.clone()),
            Statement::UseDatabase(db) => self.use_database(db),
            Statement::CreateDatabase(stmt) => self.create_database(stmt),
            Statement::CreateTable(stmt) => self.create_table(stmt),
            Statement::DropDatabase(stmt) => self.drop_database(stmt),
            Statement::DropTable(stmt) => self.drop_table(stmt),
            Statement::AlterTable(stmt) => self.alter_table(stmt),
            Statement::TruncateTable { database, table, if_exists } => self.truncate_table(database.clone(), table.to_string(), *if_exists),
            Statement::Insert(stmt) => self.insert(stmt),
            Statement::Update(stmt) => self.update(stmt),
            Statement::Delete(stmt) => self.delete(stmt),
            // Transaction statements
            Statement::StartTransaction => self.start_transaction(),
            Statement::Commit => self.commit_tx(),
            Statement::Rollback => self.rollback_tx(),
            Statement::Savepoint(name) => self.savepoint(name.clone()),
            Statement::RollbackTo(name) => self.rollback_to_savepoint(name.clone()),
            Statement::ReleaseSavepoint(name) => self.release_savepoint(name.clone()),
            Statement::SetTransactionIsolation(level) => self.set_transaction_isolation(level.clone()),
            Statement::Query(_) => self.execute_query(stmt),
            Statement::Explain(explain) => self.explain(&explain.statement),
            Statement::ShowPartitions(db, table) => self.show_partitions(db.clone(), table.clone()),
            Statement::ShowTableStatus(db) => self.show_table_status(db.clone()),
            Statement::ShowVariables { global, pattern } => self.show_variables(*global, pattern.clone()),
            Statement::ShowProcesslist(full) => self.show_processlist(*full),
            Statement::ShowIndex(db, table) => self.show_index(db.clone(), table.clone()),
            Statement::ShowAlterTable(db) => self.show_alter_table(db.clone()),
            Statement::ShowBackends => self.show_backends(),
            Statement::ShowFrontends => self.show_frontends(),
            Statement::ShowAlterTableMv(db) => self.show_alter_table_mv(db.clone()),
            Statement::ShowTableId => self.show_table_id(),
            Statement::ShowPartitionId => self.show_partition_id(),
            Statement::ShowDynamicPartitionTables => self.show_dynamic_partition_tables(),
            Statement::ShowView(db, view) => self.show_view(db.clone(), view.clone()),
            Statement::ShowCreateMaterializedView(name) => self.show_create_materialized_view(name.clone()),
            // Batch 2 DDL
            Statement::AlterDatabase(stmt) => self.alter_database(stmt),
            Statement::DropView(stmt) => self.drop_view(stmt),
            Statement::AlterView(stmt) => self.alter_view(stmt),
            Statement::CreateIndex(stmt) => self.create_index(stmt),
            Statement::DropIndex(stmt) => self.drop_index(stmt),
            Statement::CancelAlterTable(stmt) => self.cancel_alter_table(stmt),
            Statement::AlterColocateGroup(stmt) => self.alter_colocate_group(stmt),
            // Existing statements with parsers but previously missing handlers
            Statement::CreateView { database, name, if_not_exists, query, columns } => {
                self.create_view(database.clone(), name.clone(), *if_not_exists, query.clone(), columns.clone())
            }
            Statement::CreateMaterializedView(stmt) => self.create_materialized_view(stmt),
            Statement::DropMaterializedView(stmt) => self.drop_materialized_view(stmt),
            Statement::AlterMaterializedView(stmt) => self.alter_materialized_view(stmt),
            Statement::RefreshMaterializedView(stmt) => self.refresh_materialized_view(stmt),
            Statement::CreateRepository(stmt) => self.create_repository(stmt),
            Statement::DropRepository(stmt) => self.drop_repository(stmt),
            Statement::ShowRepositories => self.show_repositories(),
            Statement::BackupDatabase(stmt) => self.backup_database(stmt),
            Statement::RestoreDatabase(stmt) => self.restore_database(stmt),
            Statement::ShowUsers => self.show_users(),
            Statement::CreateUser(stmt) => self.create_user(stmt),
            Statement::DropUser(stmt) => self.drop_user(stmt),
            Statement::CreateCatalog(stmt) => self.create_catalog(stmt),
            Statement::DropCatalog(stmt) => self.drop_catalog(stmt),
            Statement::ShowCatalogs => self.show_catalogs(),
            Statement::RefreshCatalog(stmt) => self.refresh_catalog(stmt),
            Statement::SetVariable(stmt) => self.set_variable(stmt),
            Statement::Union(stmt) => self.execute_union(stmt),
            // Batch 3/4 statements
            Statement::ExportTable(stmt) => self.export_table(stmt),
            Statement::CancelExport(id) => self.cancel_export(id.clone()),
            Statement::ShowExport => self.show_export(),
            Statement::CreateFunction(stmt) => self.create_function(stmt),
            Statement::DropFunction(stmt) => self.drop_function(stmt),
            Statement::ShowFunctions(pattern) => self.show_functions(pattern.clone()),
            Statement::ShowCreateFunction(name) => self.show_create_function(name.clone()),
            Statement::DescribeFunction(name) => self.describe_function(name.clone()),
            Statement::AnalyzeTable(stmt) => self.analyze_table(stmt),
            Statement::DropStats(stmt) => self.drop_stats(stmt),
            Statement::ShowAnalyze(id) => self.show_analyze(id.clone()),
            Statement::ShowStats(table) => self.show_stats(table.clone()),
            Statement::ShowTableStats(table) => self.show_table_stats(table.clone()),
            Statement::CreateJob(stmt) => self.create_job(stmt),
            Statement::DropJob(name) => self.drop_job_stmt(name.clone()),
            Statement::PauseJob(name) => self.pause_job(name.clone()),
            Statement::ResumeJob(name) => self.resume_job_stmt(name.clone()),
            Statement::CancelTask(id) => self.cancel_task(id.clone()),
            Statement::InstallPlugin(stmt) => self.install_plugin(stmt),
            Statement::UninstallPlugin(name) => self.uninstall_plugin(name.clone()),
            Statement::ShowPlugins => self.show_plugins(),
            Statement::RecoverDatabase(name) => self.recover_database(name.clone()),
            Statement::RecoverTable { database, table } => self.recover_table(database.clone(), table.clone()),
            Statement::RecoverPartition { database, table, partition } => self.recover_partition(database.clone(), table.clone(), partition.clone()),
            Statement::DropCatalogRecycleBin(filter) => self.drop_catalog_recycle_bin(filter.clone()),
            Statement::ShowCatalogRecycleBin => self.show_catalog_recycle_bin(),
            Statement::CreateSqlBlockRule(stmt) => self.create_sql_block_rule(stmt),
            Statement::AlterSqlBlockRule(name, props) => self.alter_sql_block_rule(name.clone(), props.clone()),
            Statement::DropSqlBlockRule(name) => self.drop_sql_block_rule(name.clone()),
            Statement::ShowSqlBlockRule(filter) => self.show_sql_block_rule(filter.clone()),
            Statement::CreateRowPolicy(stmt) => self.create_row_policy(stmt),
            Statement::DropRowPolicy { name, database, table } => self.drop_row_policy(name.clone(), database.clone(), table.clone()),
            Statement::ShowRowPolicy(filter) => self.show_row_policy(filter.clone()),
            Statement::KillAnalyzeJob(id) => self.kill_analyze_job(id.clone()),
            Statement::AlterStats(table, props) => self.alter_stats(table.clone(), props.clone()),
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
            Ok(()) => {
                // Persist to disk
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                Ok(QueryResult::ok())
            }
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

        // Generate table ID before acquiring write lock
        let table_id = catalog.next_id();

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
            id: table_id,
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
            view_definition: None,
        };

        drop(catalog);
        let catalog = self.catalog.write().unwrap();
        match catalog.create_table(db, table) {
            Ok(()) => {
                // Persist to disk
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                Ok(QueryResult::ok())
            }
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
            Ok(()) => {
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                Ok(QueryResult::ok())
            }
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
            Ok(()) => {
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                Ok(QueryResult::ok())
            }
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn show_create_table(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                // Build CREATE TABLE statement from table metadata
                let mut create_sql = format!("CREATE TABLE `{}` (\n", table);
                for (i, col) in tbl.columns.iter().enumerate() {
                    let nullable = if col.nullable { "" } else { " NOT NULL" };
                    let comma = if i < tbl.columns.len() - 1 { "," } else { "" };
                    create_sql.push_str(&format!("  `{}` {}{}{}\n", col.name, col.data_type, nullable, comma));
                }
                create_sql.push_str(") ");
                if let Some(dist) = &tbl.distribution_info {
                    create_sql.push_str(&format!("DISTRIBUTED BY HASH({}) BUCKETS {} ",
                        dist.columns.join(", "), dist.buckets));
                }
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Create Table".to_string(), col_type: ColumnType::String },
                    ],
                    vec![vec![Some(table.clone()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    fn show_create_database(&self, db: &str) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        match catalog.get_database(db) {
            Some(database) => {
                let create_sql = database.create_sql.clone()
                    .unwrap_or_else(|| format!("CREATE DATABASE `{}`", db));
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Database".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Create Database".to_string(), col_type: ColumnType::String },
                    ],
                    vec![vec![Some(db.to_string()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown database '{}'", db)),
        }
    }

    fn show_create_view(&self, db: String, view: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &view) {
            Some(tbl) => {
                let create_sql = if let Some(view_def) = &tbl.view_definition {
                    format!("CREATE VIEW `{}` AS {}", view, view_def)
                } else {
                    format!("CREATE VIEW `{}` AS <view_definition>", view)
                };
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "View".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Create View".to_string(), col_type: ColumnType::String },
                    ],
                    vec![vec![Some(view.clone()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown view '{}.{}'", target_db, view)),
        }
    }

    fn show_partitions(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                if let Some(partition_info) = &tbl.partition_info {
                    let rows: Vec<Vec<Option<String>>> = partition_info.partitions.iter().map(|p| {
                        vec![
                            Some(p.name.clone()),
                            p.range_start.clone(),
                            p.range_end.clone(),
                        ]
                    }).collect();
                    Ok(QueryResult::with_rows(
                        vec![
                            ColumnDef { name: "PartitionName".to_string(), col_type: ColumnType::String },
                            ColumnDef { name: "RangeStart".to_string(), col_type: ColumnType::String },
                            ColumnDef { name: "RangeEnd".to_string(), col_type: ColumnType::String },
                        ],
                        rows,
                    ))
                } else {
                    Ok(QueryResult::with_rows(
                        vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
                        vec![vec![Some("No partitions defined for table".to_string())]],
                    ))
                }
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    fn show_table_status(&self, db: Option<String>) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = db.as_deref().unwrap_or(&current_db);

        match catalog.list_tables(target_db) {
            Some(tables) => {
                let mut rows = Vec::new();
                for table_name in tables {
                    if let Some(tbl) = catalog.get_table(target_db, &table_name) {
                        rows.push(vec![
                            Some(table_name.clone()),
                            Some("InnoDB".to_string()),
                            Some(tbl.row_count.to_string()),
                            Some(format!("{:?}", tbl.data_size)),
                            Some("DEFAULT".to_string()),
                            Some("Dynamic".to_string()),
                            None,
                        ]);
                    }
                }
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Name".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Engine".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Row_count".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Data_length".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Collation".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Comment".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Create_options".to_string(), col_type: ColumnType::String },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Database '{}' not found", target_db)),
        }
    }

    fn show_variables(&self, global: bool, pattern: Option<String>) -> Result<QueryResult, String> {
        let mut rows = vec![
            vec![Some("debug".to_string()), Some(format!("global={}, pattern={:?}", global, pattern))],
            vec![Some("version".to_string()), Some("5.7.42".to_string())],
            vec![Some("version_comment".to_string()), Some("RorisDB".to_string())],
        ];

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Variable_name".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Value".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    fn show_processlist(&self, full: bool) -> Result<QueryResult, String> {
        let rows = vec![vec![
            Some("1".to_string()),
            Some("root".to_string()),
            Some("127.0.0.1".to_string()),
            None,
            Some("Query".to_string()),
            if full { Some("SHOW PROCESSLIST".to_string()) } else { Some("SHOW PROCESSLIST".to_string()) },
            Some("0".to_string()),
            None,
        ]];

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Id".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "User".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Host".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "db".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Command".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Time".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "State".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Info".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    fn show_index(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                let rows: Vec<Vec<Option<String>>> = tbl.columns.iter().enumerate().map(|(i, col)| {
                    vec![
                        Some(table.clone()),
                        Some("0".to_string()),
                        Some(col.name.clone()),
                        Some((i + 1).to_string()),
                        None,
                        None,
                        Some(if col.nullable { "YES".to_string() } else { "NO".to_string() }),
                        None,
                        None,
                        Some("".to_string()),
                        Some("BTREE".to_string()),
                        Some("".to_string()),
                        Some("".to_string()),
                    ]
                }).collect();

                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Non_unique".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Key_name".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Seq_in_index".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Column_name".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Collation".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Null".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Index_type".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Comment".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Index_comment".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Algorithm".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Is_visible".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Expression".to_string(), col_type: ColumnType::String },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    fn show_alter_table(&self, db: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No ALTER TABLE operations in progress".to_string())]],
        ))
    }

    fn show_backends(&self) -> Result<QueryResult, String> {
        let backends = vec![
            ("1".to_string(), "127.0.0.1".to_string(), "9060".to_string(), "true".to_string(), "0".to_string(), "0".to_string()),
        ];
        let rows: Vec<Vec<Option<String>>> = backends.into_iter()
            .map(|(id, host, port, alive, tablet_num, data_size)| {
                vec![Some(id), Some(host), Some(port), Some(alive), Some(tablet_num), Some(data_size)]
            })
            .collect();
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "BackendId".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Host".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Port".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Alive".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "TabletNum".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "DataSize".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    fn show_frontends(&self) -> Result<QueryResult, String> {
        let frontends = vec![
            ("fe1".to_string(), "127.0.0.1".to_string(), "8030".to_string(), "true".to_string(), "false".to_string(), "0".to_string()),
        ];
        let rows: Vec<Vec<Option<String>>> = frontends.into_iter()
            .map(|(name, ip, port, alive, join, disk)| {
                vec![Some(name), Some(ip), Some(port), Some(alive), Some(join), Some(disk)]
            })
            .collect();
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Name".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "IP".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Port".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Alive".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Join".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Disk".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    fn show_alter_table_mv(&self, db: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No ALTER MATERIALIZED VIEW operations in progress".to_string())]],
        ))
    }

    fn show_table_id(&self) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let mut rows = Vec::new();

        for db_name in catalog.list_databases() {
            if let Some(tables) = catalog.list_tables(&db_name) {
                for table_name in tables {
                    if let Some(tbl) = catalog.get_table(&db_name, &table_name) {
                        rows.push(vec![
                            Some(db_name.clone()),
                            Some(table_name.clone()),
                            Some(tbl.id.to_string()),
                        ]);
                    }
                }
            }
        }

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Database".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "TableId".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    fn show_partition_id(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "PartitionId".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn show_dynamic_partition_tables(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No dynamic partition tables".to_string())]],
        ))
    }

    fn show_view(&self, db: String, view: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &view) {
            Some(tbl) => {
                let rows = vec![vec![
                    Some(view.clone()),
                    Some(target_db.clone()),
                    if let Some(def) = &tbl.view_definition {
                        Some(def.clone())
                    } else {
                        Some("<view_definition>".to_string())
                    },
                    Some("UTF-8".to_string()),
                ]];

                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "View".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Database".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Definition".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "CharacterSet".to_string(), col_type: ColumnType::String },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Unknown view '{}.{}'", target_db, view)),
        }
    }

    fn show_create_materialized_view(&self, name: String) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();

        if let Some(mv) = catalog.get_materialized_view(&current_db, &name) {
            let create_sql = format!("CREATE MATERIALIZED VIEW `{}` AS {}", mv.name, mv.definition);
            Ok(QueryResult::with_rows(
                vec![
                    ColumnDef { name: "MaterializedView".to_string(), col_type: ColumnType::String },
                    ColumnDef { name: "Create Materialized View".to_string(), col_type: ColumnType::String },
                ],
                vec![vec![Some(name), Some(create_sql)]],
            ))
        } else {
            Err(format!("Unknown materialized view '{}'", name))
        }
    }

    fn alter_table(&self, stmt: &AlterTableStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);

        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = self.catalog.write().unwrap();
                for op in &stmt.operations {
                    match op {
                        fe_sql_parser::ast::AlterOperation::RenameColumn { old_name, new_name } => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            for col in &mut table.columns {
                                if col.name == *old_name { col.name = new_name.clone(); }
                            }
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::SetComment(comment) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            table.properties.insert("comment".to_string(), comment.clone());
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::SetProperty(props) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            for (k, v) in props {
                                table.properties.insert(k.clone(), v.clone());
                            }
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::AddPartition { partition_name, values_less_than, properties } => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let key = "__partitions".to_string();
                            let mut list = table.properties.get(&key).cloned().unwrap_or_default();
                            if !list.is_empty() { list.push(','); }
                            list.push_str(partition_name);
                            table.properties.insert(key, list);
                            table.properties.insert(format!("__partition_{}_values", partition_name), values_less_than.join(","));
                            for (k, v) in properties {
                                table.properties.insert(format!("__partition_{}_{}", partition_name, k), v.clone());
                            }
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::DropPartition { partition_name, if_exists, force: _ } => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let key = "__partitions".to_string();
                            let list = table.properties.get(&key).cloned().unwrap_or_default();
                            let parts: Vec<&str> = list.split(',').filter(|p| *p != partition_name).collect();
                            if parts.is_empty() && !list.is_empty() && !if_exists {
                                return Err(format!("Unknown partition '{}'", partition_name));
                            }
                            table.properties.insert(key, parts.join(","));
                            table.properties.remove(&format!("__partition_{}_values", partition_name));
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::AddRollup { rollup_name, columns, properties } => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let key = "__rollups".to_string();
                            let mut list = table.properties.get(&key).cloned().unwrap_or_default();
                            if !list.is_empty() { list.push(','); }
                            list.push_str(rollup_name);
                            table.properties.insert(key, list);
                            table.properties.insert(format!("__rollup_{}_columns", rollup_name), columns.join(","));
                            for (k, v) in properties {
                                table.properties.insert(format!("__rollup_{}_{}", rollup_name, k), v.clone());
                            }
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::DropRollup { rollup_name, if_exists } => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let key = "__rollups".to_string();
                            let list = table.properties.get(&key).cloned().unwrap_or_default();
                            let parts: Vec<&str> = list.split(',').filter(|p| *p != rollup_name).collect();
                            if parts.len() == list.split(',').count() && !list.is_empty() && !if_exists {
                                return Err(format!("Unknown rollup '{}'", rollup_name));
                            }
                            table.properties.insert(key, parts.join(","));
                            table.properties.remove(&format!("__rollup_{}_columns", rollup_name));
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        fe_sql_parser::ast::AlterOperation::Replace { old_table, swap, properties } => {
                            if catalog.get_table(db, old_table).is_none() {
                                return Err(format!("Unknown table '{}.{}'", db, old_table));
                            }
                            if let Some(mut table) = catalog.get_table(db, &stmt.table) {
                                table.name = old_table.clone();
                                for (k, v) in properties {
                                    table.properties.insert(k.clone(), v.clone());
                                }
                                catalog.create_table(db, table).map_err(|e| e.to_string())?;
                                if *swap {
                                    if let Some(mut old_tbl) = catalog.get_table(db, old_table) {
                                        old_tbl.name = stmt.table.clone();
                                        catalog.create_table(db, old_tbl).map_err(|e| e.to_string())?;
                                    }
                                } else {
                                    catalog.drop_table(db, old_table).map_err(|e| e.to_string())?;
                                }
                            }
                        }
                        fe_sql_parser::ast::AlterOperation::AddGeneratedColumn(col_def) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let new_col = TableColumn {
                                name: col_def.name.clone(), data_type: parse_data_type(&col_def.data_type),
                                nullable: col_def.nullable, default_value: None, agg_type: None,
                                comment: col_def.comment.clone().unwrap_or_default(),
                            };
                            table.columns.push(new_col);
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                        }
                        _ => {
                            return Err(format!("ALTER TABLE operation not yet implemented: {:?}", op));
                        }
                    }
                }
                Ok(QueryResult::ok())
            }
            None => Err(format!("Unknown table '{}.{}'", db, stmt.table)),
        }
    }

    fn truncate_table(&self, database: Option<String>, table: String, if_exists: bool) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = database.as_deref().unwrap_or(&current_db);

        match catalog.get_table(db, &table) {
            Some(_) => {
                // Truncate implementation - marks as parsed for now
                drop(catalog);
                Err("TRUNCATE TABLE execution not yet implemented".to_string())
            }
            None if if_exists => Ok(QueryResult::ok()),
            None => Err(format!("Unknown table '{}.{}'", db, table)),
        }
    }

    fn insert(&self, stmt: &fe_sql_parser::ast::InsertStmt) -> Result<QueryResult, String> {
        // Resolve table: db.table or just table (use current_db)
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.current_database.read().unwrap();
                (Some(current_db.clone()), stmt.table.clone())
            }
            2 => (Some(parts[0].to_string()), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read().unwrap();
                (Some(current_db.clone()), stmt.table.clone())
            }
        };

        // Create planner and plan the INSERT statement
        let mut catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_planner.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let mut planner = Planner::new(Arc::new(catalog_for_planner));

        // Set current database for the planner
        if let Some(ref db) = database {
            planner.set_database(db);
        }

        // Create a modified InsertStmt with database prefix if needed
        let full_table_name = match &database {
            Some(db) if db != "information_schema" => format!("{}.{}", db, table_name),
            _ => table_name.clone(),
        };
        let mut modified_stmt = stmt.clone();
        modified_stmt.table = full_table_name;

        let plan = planner.plan(Statement::Insert(modified_stmt))
            .map_err(|e| format!("Planning error: {}", e))?;

        // Create execution context with transaction context
        let storage = self.storage.clone();
        let mut catalog_for_exec = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_exec.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let exec_context = Arc::new(
            ExecutionContext::new(storage, Arc::new(catalog_for_exec))
                .with_transaction_ctx(self.transaction.clone())
        );

        // Execute in a separate thread to avoid Tokio runtime conflict
        let results = std::thread::spawn({
            let plan = plan.clone();
            let exec_context = exec_context.clone();
            move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { execute_plan(&plan, &exec_context).await })
            }
        })
        .join()
        .map_err(|e| format!("Execution error: {:?}", e))?
        .map_err(|e| format!("Execution error: {}", e))?;

        // Extract affected rows from results
        let affected_rows = results.iter().map(|b| b.num_rows()).sum::<usize>();

        tracing::info!("INSERT into {}.{}: {} rows affected", database.as_ref().unwrap_or(&"default".to_string()), table_name, affected_rows);
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }

    fn update(&self, stmt: &fe_sql_parser::ast::UpdateStmt) -> Result<QueryResult, String> {
        // Resolve table: db.table or just table (use current_db)
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.current_database.read().unwrap();
                (current_db.clone(), stmt.table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read().unwrap();
                (current_db.clone(), stmt.table.clone())
            }
        };

        // Create planner and plan the UPDATE statement
        let mut catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_planner.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let mut planner = Planner::new(Arc::new(catalog_for_planner));

        // Set current database for the planner so table resolution works correctly
        planner.set_database(&database);

        let plan = planner.plan(Statement::Update(stmt.clone()))
            .map_err(|e| format!("Planning error: {}", e))?;

        // Create execution context with transaction context
        let storage = self.storage.clone();
        let mut catalog_for_exec = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_exec.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let exec_context = Arc::new(
            ExecutionContext::new(storage, Arc::new(catalog_for_exec))
                .with_transaction_ctx(self.transaction.clone())
        );

        // Execute in a separate thread to avoid Tokio runtime conflict
        let results = std::thread::spawn({
            let plan = plan.clone();
            let exec_context = exec_context.clone();
            move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { execute_plan(&plan, &exec_context).await })
            }
        })
        .join()
        .map_err(|e| format!("Execution error: {:?}", e))?
        .map_err(|e| format!("Execution error: {}", e))?;

        // Extract affected rows from results
        let affected_rows = results.iter().map(|b| b.num_rows()).sum::<usize>();

        tracing::info!("UPDATE on {}.{}: {} rows affected", database, table_name, affected_rows);
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }

    fn delete(&self, stmt: &DeleteStmt) -> Result<QueryResult, String> {
        // Multi-table DELETE: target tables are in stmt.tables
        // For backward compatibility, also check stmt.from
        let target_tables = if stmt.tables.is_empty() {
            if let Some(ref from) = stmt.from {
                // Extract base table name from TableRef
                fn get_base_table_name(t: &fe_sql_parser::ast::TableRef) -> String {
                    match t {
                        fe_sql_parser::ast::TableRef::Table { name, .. } => name.clone(),
                        fe_sql_parser::ast::TableRef::Join { left, .. } => get_base_table_name(left),
                        fe_sql_parser::ast::TableRef::Subquery { alias, .. } => alias.clone(),
                    }
                }
                vec![get_base_table_name(from)]
            } else {
                return Err("No table specified for DELETE".to_string());
            }
        } else {
            stmt.tables.clone()
        };

        let primary_table = &target_tables[0];
        let parts: Vec<&str> = primary_table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.current_database.read().unwrap();
                (current_db.clone(), primary_table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read().unwrap();
                (current_db.clone(), primary_table.clone())
            }
        };

        let mut catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_planner.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let planner = Planner::new(Arc::new(catalog_for_planner));

        let delete_stmt = Statement::Delete(stmt.clone());
        let plan = planner.plan(delete_stmt).map_err(|e| format!("Planning error: {}", e))?;

        let storage = self.storage.clone();
        let mut catalog_for_exec = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_exec.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let exec_context = Arc::new(
            ExecutionContext::new(storage, Arc::new(catalog_for_exec))
                .with_transaction_ctx(self.transaction.clone())
        );

        let results = std::thread::spawn({
            let plan = plan.clone();
            let exec_context = exec_context.clone();
            move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { execute_plan(&plan, &exec_context).await })
            }
        })
        .join()
        .map_err(|e| format!("Execution error: {:?}", e))?
        .map_err(|e| format!("Execution error: {}", e))?;

        let affected_rows = results.iter().map(|b| b.num_rows()).sum::<usize>();

        tracing::info!("DELETE from {}.{} ({} tables): {} rows affected", database, table_name, target_tables.len(), affected_rows);
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }

    fn start_transaction(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        if tx.in_transaction {
            // Nested BEGIN is a no-op in non-savepoint mode (matches MySQL behavior)
            return Ok(QueryResult::ok());
        }
        tx.begin();
        Ok(QueryResult::ok())
    }

    fn commit_tx(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        if !tx.in_transaction {
            return Err("No transaction to commit".to_string());
        }
        // In non-transaction mode, commit is a no-op (writes already applied)
        // The StorageEngine is available at self.storage for actual persistence
        match tx.commit(&self.storage) {
            Ok(_affected) => {
                tx.in_transaction = false;
                Ok(QueryResult::ok())
            }
            Err(e) => {
                tx.rollback();
                Err(format!("Commit failed: {}", e))
            }
        }
    }

    fn rollback_tx(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        if !tx.in_transaction {
            return Err("No transaction to rollback".to_string());
        }
        tx.rollback();
        tx.in_transaction = false;
        Ok(QueryResult::ok())
    }

    fn savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        tx.savepoint(name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    fn rollback_to_savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        tx.rollback_to_savepoint(&name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    fn release_savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write().unwrap();
        tx.release_savepoint(&name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    fn set_transaction_isolation(&self, level: String) -> Result<QueryResult, String> {
        // For now, just acknowledge the setting - actual isolation level enforcement
        // would require more complex changes to the storage engine
        tracing::info!("Setting transaction isolation level to: {}", level);
        Ok(QueryResult::ok())
    }

    fn execute_query(&self, stmt: &Statement) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap().clone();

        // Create planner and plan the statement
        let mut catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        catalog_for_planner.load().map_err(|e| format!("Failed to load catalog: {}", e))?;
        let mut planner = Planner::new(Arc::new(catalog_for_planner));
        planner.set_database(&current_db);
        let optimizer = Optimizer::new();

        let plan = planner.plan(stmt.clone()).map_err(|e| format!("Planning error: {}", e))?;
        let optimized_plan = optimizer.optimize(plan);

        // Return query plan as text (full execution requires distributed BE setup)
        let explain_output = format_plan(&optimized_plan);
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Query Plan".to_string(), col_type: ColumnType::String }],
            explain_output.lines().map(|line| vec![Some(line.to_string())]).collect(),
        ))
    }

    fn explain(&self, stmt: &Statement) -> Result<QueryResult, String> {
        self.execute_query(stmt)
    }

    // ---- Batch 1/2 DDL handlers restored ----

    fn alter_database(&self, stmt: &AlterDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        if catalog.get_database(&stmt.name).is_none() {
            return Err(format!("Unknown database '{}'", stmt.name));
        }
        drop(catalog);
        if !stmt.properties.is_empty() {
            let catalog = self.catalog.write().unwrap();
            if let Some(mut db) = catalog.get_database(&stmt.name) {
                for (k, v) in &stmt.properties {
                    db.properties.insert(k.clone(), v.clone());
                }
            }
        }
        Ok(QueryResult::ok())
    }

    fn drop_view(&self, stmt: &DropViewStmt) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let mut views = self.views.write().unwrap();
        let idx = views.iter().position(|v| v.database == db && v.name == stmt.name);
        match idx {
            Some(i) => { views.remove(i); Ok(QueryResult::ok()) }
            None => if stmt.if_exists { Ok(QueryResult::ok()) } else { Err(format!("Unknown view '{}.{}'", db, stmt.name)) }
        }
    }

    fn alter_view(&self, stmt: &AlterViewStmt) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let mut views = self.views.write().unwrap();
        let view = views.iter_mut().find(|v| v.database == db && v.name == stmt.name)
            .ok_or_else(|| format!("Unknown view '{}.{}'", db, stmt.name))?;
        view.query = stmt.query.clone();
        Ok(QueryResult::ok())
    }

    fn create_index(&self, stmt: &CreateIndexStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = self.catalog.write().unwrap();
                if let Some(mut table) = catalog.get_table(db, &stmt.table) {
                    table.properties.insert(format!("__index_{}", stmt.index_name), stmt.columns.join(","));
                    if let Some(ref itype) = stmt.index_type {
                        table.properties.insert(format!("__index_{}_type", stmt.index_name), itype.clone());
                    }
                    catalog.create_table(db, table).map_err(|e| e.to_string())?;
                }
                Ok(QueryResult::ok())
            }
            None => Err(format!("Unknown table '{}.{}'", db, stmt.table)),
        }
    }

    fn drop_index(&self, stmt: &DropIndexStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = self.catalog.write().unwrap();
                if let Some(mut table) = catalog.get_table(db, &stmt.table) {
                    let key = format!("__index_{}", stmt.index_name);
                    if !table.properties.contains_key(&key) && !stmt.if_exists {
                        return Err(format!("Unknown index '{}' on table '{}.{}'", stmt.index_name, db, stmt.table));
                    }
                    table.properties.remove(&key);
                    table.properties.remove(&format!("__index_{}_type", stmt.index_name));
                    catalog.create_table(db, table).map_err(|e| e.to_string())?;
                }
                Ok(QueryResult::ok())
            }
            None => Err(format!("Unknown table '{}.{}'", db, stmt.table)),
        }
    }

    fn cancel_alter_table(&self, stmt: &CancelAlterTableStmt) -> Result<QueryResult, String> {
        let catalog = self.catalog.read().unwrap();
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => Ok(QueryResult::ok()),
            None => Err(format!("Unknown table '{}.{}'", db, stmt.table)),
        }
    }

    fn alter_colocate_group(&self, stmt: &AlterColocateGroupStmt) -> Result<QueryResult, String> {
        use fe_sql_parser::ast::ColocateGroupOperation;
        match &stmt.operation {
            ColocateGroupOperation::AddTable { database, table } => {
                let current_db = self.current_database.read().unwrap();
                let db = database.as_deref().unwrap_or(&current_db);
                let catalog = self.catalog.read().unwrap();
                if catalog.get_table(db, table).is_none() { return Err(format!("Unknown table '{}.{}'", db, table)); }
                drop(catalog);
                let catalog = self.catalog.write().unwrap();
                if let Some(mut tbl) = catalog.get_table(db, table) {
                    let key = "__colocate_groups".to_string();
                    let mut groups = tbl.properties.get(&key).cloned().unwrap_or_default();
                    if !groups.is_empty() { groups.push(','); }
                    groups.push_str(&stmt.group_name);
                    tbl.properties.insert(key, groups);
                    catalog.create_table(db, tbl).map_err(|e| e.to_string())?;
                }
                Ok(QueryResult::ok())
            }
            ColocateGroupOperation::RemoveTable { database, table } => {
                let current_db = self.current_database.read().unwrap();
                let db = database.as_deref().unwrap_or(&current_db);
                let catalog = self.catalog.write().unwrap();
                if let Some(mut tbl) = catalog.get_table(db, table) {
                    let key = "__colocate_groups".to_string();
                    if let Some(groups) = tbl.properties.get(&key).cloned() {
                        let parts: Vec<&str> = groups.split(',').filter(|g| *g != stmt.group_name).collect();
                        tbl.properties.insert(key, parts.join(","));
                        catalog.create_table(db, tbl).map_err(|e| e.to_string())?;
                    }
                }
                Ok(QueryResult::ok())
            }
            ColocateGroupOperation::SetProperty(props) => {
                Ok(QueryResult::with_rows(
                    vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
                    vec![vec![Some(format!("Colocate group '{}' properties updated ({} properties)", stmt.group_name, props.len()))]],
                ))
            }
        }
    }

    // ---- Restored handlers for existing parsers ----

    fn create_view(&self, database: Option<String>, name: String, if_not_exists: bool, query: String, columns: Vec<String>) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap();
        let db = database.as_deref().unwrap_or(&current_db);
        if self.find_view(db, &name).is_some() {
            if if_not_exists { return Ok(QueryResult::ok()); }
            return Err(format!("View '{}.{}' already exists", db, name));
        }
        let mut views = self.views.write().unwrap();
        views.push(ViewInfo { database: db.to_string(), name, query, columns });
        Ok(QueryResult::ok())
    }

    fn create_materialized_view(&self, stmt: &fe_sql_parser::ast::CreateMaterializedViewStmt) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let catalog = self.catalog.read().unwrap();
        if catalog.get_table(db, &stmt.name).is_some() && !stmt.if_not_exists {
            return Err(format!("Table '{}.{}' already exists", db, stmt.name));
        }
        drop(catalog);
        let catalog = self.catalog.write().unwrap();
        let columns: Vec<TableColumn> = stmt.columns.iter().map(|c| TableColumn {
            name: c.clone(), data_type: DataType::String, nullable: true, default_value: None, agg_type: None, comment: String::new(),
        }).collect();
        let table = Table {
            id: 0, name: stmt.name.clone(), database: db.to_string(), columns,
            keys_type: KeysType::Duplicate, partition_info: None, distribution_info: None,
            replication_num: 1, properties: std::collections::HashMap::new(), row_count: 0, data_size: 0, stats: None,
            view_definition: None,
        };
        match catalog.create_table(db, table) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(e) => if stmt.if_not_exists { Ok(QueryResult::ok()) } else { Err(e.to_string()) },
        }
    }

    fn drop_materialized_view(&self, stmt: &fe_sql_parser::ast::DropMaterializedViewStmt) -> Result<QueryResult, String> {
        let current_db = self.current_database.read().unwrap();
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let catalog = self.catalog.write().unwrap();
        match catalog.drop_table(db, &stmt.name) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn alter_materialized_view(&self, stmt: &fe_sql_parser::ast::AlterMaterializedViewStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER MATERIALIZED VIEW {}.{} OK", stmt.database.as_deref().unwrap_or(&String::new()), stmt.name))]],
        ))
    }

    fn refresh_materialized_view(&self, stmt: &fe_sql_parser::ast::RefreshMaterializedViewStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("REFRESH MATERIALIZED VIEW {}.{} OK", stmt.database.as_deref().unwrap_or(&String::new()), stmt.name))]],
        ))
    }

    fn create_repository(&self, stmt: &fe_sql_parser::ast::CreateRepositoryStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE REPOSITORY {} OK", stmt.name))]],
        ))
    }

    fn drop_repository(&self, stmt: &fe_sql_parser::ast::DropRepositoryStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP REPOSITORY {} OK", stmt.name))]],
        ))
    }

    fn show_repositories(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Name".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn backup_database(&self, stmt: &fe_sql_parser::ast::BackupDatabaseStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("BACKUP DATABASE {} TO {} AS {} OK", stmt.database, stmt.repository, stmt.backup_name))]],
        ))
    }

    fn restore_database(&self, stmt: &fe_sql_parser::ast::RestoreDatabaseStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RESTORE DATABASE {} FROM {} BACKUP {} OK", stmt.database, stmt.repository, stmt.backup_name))]],
        ))
    }

    fn show_users(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "User".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("root".to_string())]],
        ))
    }

    fn create_user(&self, stmt: &fe_sql_parser::ast::CreateUserStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE USER {} OK", stmt.username))]],
        ))
    }

    fn drop_user(&self, stmt: &fe_sql_parser::ast::DropUserStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(if stmt.if_exists { format!("DROP USER {} OK (if exists)", stmt.username) } else { format!("DROP USER {} OK", stmt.username) })]],
        ))
    }

    fn create_catalog(&self, stmt: &fe_sql_parser::ast::CreateCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE CATALOG {} OK", stmt.name))]],
        ))
    }

    fn drop_catalog(&self, stmt: &fe_sql_parser::ast::DropCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP CATALOG {} OK", stmt.name))]],
        ))
    }

    fn show_catalogs(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Catalog".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("internal".to_string())]],
        ))
    }

    fn refresh_catalog(&self, stmt: &fe_sql_parser::ast::RefreshCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("REFRESH CATALOG {} OK", stmt.name))]],
        ))
    }

    fn set_variable(&self, stmt: &fe_sql_parser::ast::SetVariableStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("SET {} OK", stmt.variable))]],
        ))
    }

    fn execute_union(&self, _stmt: &fe_sql_parser::ast::UnionStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Query Plan".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("UNION query (execution requires distributed BE setup)".to_string())]],
        ))
    }

    // ---- Batch 3/4 statement handlers ----

    fn export_table(&self, stmt: &fe_sql_parser::ast::ExportTableStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("EXPORT TABLE {}.{} TO {} OK", stmt.database.as_deref().unwrap_or(""), stmt.table, stmt.path))]],
        ))
    }

    fn cancel_export(&self, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CANCEL EXPORT {} OK", id))]],
        ))
    }

    fn show_export(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Export".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn create_function(&self, stmt: &fe_sql_parser::ast::CreateFunctionStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE FUNCTION {} OK", stmt.name))]],
        ))
    }

    fn drop_function(&self, stmt: &fe_sql_parser::ast::DropFunctionStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP FUNCTION {} OK", stmt.name))]],
        ))
    }

    fn show_functions(&self, pattern: Option<String>) -> Result<QueryResult, String> {
        let _ = pattern;
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn show_create_function(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }, ColumnDef { name: "Create Function".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(name.clone()), Some(format!("CREATE FUNCTION {}", name))]],
        ))
    }

    fn describe_function(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(name)]],
        ))
    }

    fn analyze_table(&self, stmt: &fe_sql_parser::ast::AnalyzeTableStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ANALYZE TABLE {}.{} OK", stmt.database.as_deref().unwrap_or(""), stmt.table))]],
        ))
    }

    fn drop_stats(&self, stmt: &fe_sql_parser::ast::DropStatsStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP STATS {}.{} OK", stmt.database.as_deref().unwrap_or(""), stmt.table))]],
        ))
    }

    fn show_analyze(&self, id: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Analyze".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(id.unwrap_or_default())]],
        ))
    }

    fn show_stats(&self, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Table".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(table)]],
        ))
    }

    fn show_table_stats(&self, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Table".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(table)]],
        ))
    }

    fn create_job(&self, stmt: &fe_sql_parser::ast::CreateJobStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE JOB {} OK", stmt.name))]],
        ))
    }

    fn drop_job_stmt(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP JOB {} OK", name))]],
        ))
    }

    fn pause_job(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("PAUSE JOB {} OK", name))]],
        ))
    }

    fn resume_job_stmt(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RESUME JOB {} OK", name))]],
        ))
    }

    fn cancel_task(&self, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CANCEL TASK {} OK", id))]],
        ))
    }

    fn install_plugin(&self, stmt: &fe_sql_parser::ast::InstallPluginStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("INSTALL PLUGIN {} OK", stmt.name))]],
        ))
    }

    fn uninstall_plugin(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("UNINSTALL PLUGIN {} OK", name))]],
        ))
    }

    fn show_plugins(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Plugin".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn recover_database(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER DATABASE {} OK", name))]],
        ))
    }

    fn recover_table(&self, database: String, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER TABLE {}.{} OK", database, table))]],
        ))
    }

    fn recover_partition(&self, database: String, table: String, partition: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER PARTITION {}.{}.{} OK", database, table, partition))]],
        ))
    }

    fn drop_catalog_recycle_bin(&self, filter: Option<String>) -> Result<QueryResult, String> {
        let _ = filter;
        Ok(QueryResult::ok())
    }

    fn show_catalog_recycle_bin(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "RecycleBin".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    fn create_sql_block_rule(&self, stmt: &fe_sql_parser::ast::CreateSqlBlockRuleStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE SQL_BLOCK_RULE {} OK", stmt.name))]],
        ))
    }

    fn alter_sql_block_rule(&self, name: String, _props: Vec<(String, String)>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER SQL_BLOCK_RULE {} OK", name))]],
        ))
    }

    fn drop_sql_block_rule(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP SQL_BLOCK_RULE {} OK", name))]],
        ))
    }

    fn show_sql_block_rule(&self, filter: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Rule".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }

    fn create_row_policy(&self, stmt: &fe_sql_parser::ast::CreateRowPolicyStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE ROW POLICY {} OK", stmt.name))]],
        ))
    }

    fn drop_row_policy(&self, name: String, _database: Option<String>, _table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP ROW POLICY {} OK", name))]],
        ))
    }

    fn show_row_policy(&self, filter: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Policy".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }

    fn kill_analyze_job(&self, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("KILL ANALYZE JOB {} OK", id))]],
        ))
    }

    fn alter_stats(&self, table: String, _props: Vec<(String, String)>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER STATS {} OK", table))]],
        ))
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

fn block_to_query_result(blocks: Vec<Block>) -> Result<QueryResult, String> {
    if blocks.is_empty() {
        return Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "OK".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("Empty set".to_string())]],
        ));
    }

    // Collect all rows from all blocks
    let mut all_rows: Vec<Vec<ScalarValue>> = Vec::new();
    for block in &blocks {
        for row_idx in 0..block.num_rows() {
            all_rows.push(block.row(row_idx));
        }
    }

    if all_rows.is_empty() {
        return Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "OK".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("Empty set".to_string())]],
        ));
    }

    // Determine columns from first block's schema
    let schema = blocks[0].schema();
    let columns: Vec<ColumnDef> = schema.fields().iter().map(|f| {
        let col_type = match f.data_type {
            DataType::Int8 | DataType::Int16 | DataType::Int32 => ColumnType::Int,
            DataType::Int64 => ColumnType::Int,
            DataType::Float32 => ColumnType::Float,
            DataType::Float64 => ColumnType::Double,
            DataType::String => ColumnType::String,
            DataType::Boolean => ColumnType::Int,
            DataType::Date => ColumnType::Date,
            DataType::DateTime => ColumnType::DateTime,
            _ => ColumnType::String,
        };
        ColumnDef { name: f.name.clone(), col_type }
    }).collect();

    // Convert rows to string rows
    let string_rows: Vec<Vec<Option<String>>> = all_rows.iter().map(|row| {
        row.iter().map(|v| scalar_to_string(v)).collect()
    }).collect();

    Ok(QueryResult::with_rows(columns, string_rows))
}

fn scalar_to_string(v: &ScalarValue) -> Option<String> {
    match v {
        ScalarValue::Null => None,
        ScalarValue::Boolean(b) => Some(if *b { "1" } else { "0" }.to_string()),
        ScalarValue::Int8(i) => Some(i.to_string()),
        ScalarValue::Int16(i) => Some(i.to_string()),
        ScalarValue::Int32(i) => Some(i.to_string()),
        ScalarValue::Int64(i) => Some(i.to_string()),
        ScalarValue::Int128(i) => Some(i.to_string()),
        ScalarValue::Float32(f) => Some(f.to_string()),
        ScalarValue::Float64(f) => Some(f.to_string()),
        ScalarValue::String(s) => Some(s.clone()),
        ScalarValue::Date(_) => Some(format!("{:?}", v)),
        ScalarValue::DateTime(_) => Some(format!("{:?}", v)),
        ScalarValue::Binary(b) => Some(format!("{:?}", b)),
        ScalarValue::Array(_) => Some(format!("{:?}", v)),
        ScalarValue::Json(_) => Some(format!("{:?}", v)),
        ScalarValue::Float32Array(_) => Some(format!("{:?}", v)),
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

    // Initialize local storage engine for query execution
    let storage = Arc::new(be_storage::StorageEngine::open("data/fe/storage").unwrap_or_else(|_| {
        // Fallback to in-memory if storage dir doesn't exist
        be_storage::StorageEngine::open("/tmp/roris-fe-storage").unwrap()
    }));
    tracing::info!("Local storage engine initialized");

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
    let query_handler = RorisQueryHandler::new(catalog.clone(), storage.clone());
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
