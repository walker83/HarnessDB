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
use fe_sql_parser::ast::{AlterTableStmt, CreateDatabaseStmt, CreateTableStmt, DropDatabaseStmt, DropTableStmt};
use types::{DataType, Block, ScalarValue};
use fe_catalog::table::{Table, TableColumn, KeysType};

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
}

impl RorisQueryHandler {
    fn new(catalog: Arc<StdRwLock<CatalogManager>>, storage: Arc<StorageEngine>) -> Self {
        Self {
            catalog,
            current_database: Arc::new(StdRwLock::new("information_schema".to_string())),
            storage,
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
            view_definition: None,
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
                // ALTER TABLE implementation - currently just marks as parsed
                drop(catalog);
                Err("ALTER TABLE execution not yet implemented".to_string())
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
        let (db, table_name) = match parts.len() {
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
        Err(format!("INSERT execution not yet implemented - table: {}.{}", db, table_name))
    }

    fn update(&self, stmt: &fe_sql_parser::ast::UpdateStmt) -> Result<QueryResult, String> {
        Err(format!("UPDATE execution not yet implemented - table: {}", stmt.table))
    }

    fn delete(&self, stmt: &fe_sql_parser::ast::DeleteStmt) -> Result<QueryResult, String> {
        Err(format!("DELETE execution not yet implemented - table: {}", stmt.table))
    }

    fn execute_query(&self, stmt: &Statement) -> Result<QueryResult, String> {
        use fe_sql_planner::plan_node::PlanNodeType;

        let current_db = self.current_database.read().unwrap().clone();

        // Create planner and plan the statement
        let catalog_for_planner = CatalogManager::with_path("data/fe/doris-meta");
        let planner = Planner::new(Arc::new(catalog_for_planner));
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
