use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::DataType as ADT;
use mysql_protocol::server::{ColumnDef, ColumnType};
use mysql_protocol::QueryResult;
use fe_sql_parser::Statement;

use crate::handler_struct::RorisQueryHandler;
use crate::utils::{like_match, parse_data_type};

impl RorisQueryHandler {
    pub(crate) fn execute_statement(&self, stmt: &Statement) -> Result<QueryResult, String> {
        match stmt {
            Statement::ShowDatabases => self.show_databases(),
            Statement::ShowTables(db, like) => self.show_tables(db.clone(), like.clone()),
            Statement::ShowCreateTable(db, table) => self.show_create_table(db.clone(), table.clone()),
            Statement::ShowCreateDatabase(db) => self.show_create_database(db),
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
            Statement::Query(_) => Err("Query statements should be handled by DataFusion path".to_string()),
            Statement::Explain(_) => Err("Explain statements should be handled by DataFusion path".to_string()),
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
            Statement::Union(_) => Err("Union statements should be handled by DataFusion path".to_string()),
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

    // ---- show_* methods ----

    pub(crate) fn show_databases(&self) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
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

    pub(crate) fn show_tables(&self, db: Option<String>, like: Option<String>) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
        let target_db = db.as_deref().unwrap_or(&current_db);

        match catalog.list_tables(target_db) {
            Some(tables) => {
                let rows: Vec<Vec<Option<String>>> = tables
                    .iter()
                    .filter(|t| {
                        match &like {
                            Some(pattern) => like_match(pattern, t),
                            None => true,
                        }
                    })
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

    pub(crate) fn describe(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_columns(&self, db: Option<String>, table: Option<String>) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn use_database(&self, db: &str) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        if catalog.get_database(db).is_some() {
            let mut current_db = self.current_database.write();
            *current_db = db.to_string();
            Ok(QueryResult::ok())
        } else {
            Err(format!("Unknown database '{}'", db))
        }
    }

    pub(crate) fn show_create_table(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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
                // Add UNIQUE KEY definitions
                for uk in &tbl.unique_keys {
                    if let Some(ref name) = uk.name {
                        create_sql.push_str(&format!("  UNIQUE KEY `{}` ({})\n", name, uk.columns.join(", ")));
                    } else {
                        create_sql.push_str(&format!("  UNIQUE ({})\n", uk.columns.join(", ")));
                    }
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

    pub(crate) fn show_create_database(&self, db: &str) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
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

    pub(crate) fn show_create_view(&self, db: String, view: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_partitions(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_table_status(&self, db: Option<String>) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_variables(&self, global: bool, pattern: Option<String>) -> Result<QueryResult, String> {
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

    pub(crate) fn show_processlist(&self, full: bool) -> Result<QueryResult, String> {
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

    pub(crate) fn show_index(&self, db: String, table: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_alter_table(&self, _db: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No ALTER TABLE operations in progress".to_string())]],
        ))
    }

    pub(crate) fn show_backends(&self) -> Result<QueryResult, String> {
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

    pub(crate) fn show_frontends(&self) -> Result<QueryResult, String> {
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

    pub(crate) fn show_alter_table_mv(&self, _db: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No ALTER MATERIALIZED VIEW operations in progress".to_string())]],
        ))
    }

    pub(crate) fn show_table_id(&self) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
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

    pub(crate) fn show_partition_id(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "PartitionId".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_dynamic_partition_tables(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Message".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("No dynamic partition tables".to_string())]],
        ))
    }

    pub(crate) fn show_view(&self, db: String, view: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();
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

    pub(crate) fn show_create_materialized_view(&self, name: String) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.current_database.read();

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

    pub(crate) fn show_repositories(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Name".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_users(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "User".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("root".to_string())]],
        ))
    }

    pub(crate) fn show_catalogs(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Catalog".to_string(), col_type: ColumnType::String }],
            vec![vec![Some("internal".to_string())]],
        ))
    }

    pub(crate) fn show_export(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Export".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_functions(&self, pattern: Option<String>) -> Result<QueryResult, String> {
        let _ = pattern;
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_create_function(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }, ColumnDef { name: "Create Function".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(name.clone()), Some(format!("CREATE FUNCTION {}", name))]],
        ))
    }

    pub(crate) fn describe_function(&self, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Function".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(name)]],
        ))
    }

    pub(crate) fn show_analyze(&self, id: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Analyze".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(id.unwrap_or_default())]],
        ))
    }

    pub(crate) fn show_stats(&self, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Table".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(table)]],
        ))
    }

    pub(crate) fn show_table_stats(&self, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Table".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(table)]],
        ))
    }

    pub(crate) fn show_plugins(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Plugin".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_catalog_recycle_bin(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "RecycleBin".to_string(), col_type: ColumnType::String }],
            vec![],
        ))
    }

    pub(crate) fn show_sql_block_rule(&self, filter: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Rule".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }

    pub(crate) fn show_row_policy(&self, filter: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Policy".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }
}
