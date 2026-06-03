
use ::types::DataType as RorisDataType;
use fe_sql_parser::Statement;
use mysql_protocol::QueryResult;
use mysql_protocol::server::{ColumnDef, ColumnType};

use crate::handler_struct::RorisQueryHandler;
use crate::utils::like_match;

/// Convert internal DataType to MySQL-compatible type string for DESCRIBE/SHOW CREATE TABLE
fn datatype_to_mysql_type(dt: &RorisDataType) -> String {
    match dt {
        RorisDataType::Null => "NULL".to_string(),
        RorisDataType::Boolean => "TINYINT(1)".to_string(),
        RorisDataType::Int8 => "TINYINT".to_string(),
        RorisDataType::Int16 => "SMALLINT".to_string(),
        RorisDataType::Int32 => "INT".to_string(),
        RorisDataType::Int64 => "BIGINT".to_string(),
        RorisDataType::Int128 => "DECIMAL(38,0)".to_string(),
        RorisDataType::Float32 => "FLOAT".to_string(),
        RorisDataType::Float64 => "DOUBLE".to_string(),
        RorisDataType::Decimal(d) => format!("DECIMAL({},{})", d.precision, d.scale),
        RorisDataType::Date => "DATE".to_string(),
        RorisDataType::DateTime => "DATETIME".to_string(),
        RorisDataType::Varchar(n) => format!("VARCHAR({})", n),
        RorisDataType::Char(n) => format!("CHAR({})", n),
        RorisDataType::String => "TEXT".to_string(),
        RorisDataType::Binary => "BLOB".to_string(),
        RorisDataType::Json => "JSON".to_string(),
        RorisDataType::Array(inner) => format!("ARRAY<{}>", datatype_to_mysql_type(inner)),
        RorisDataType::Map(k, v) => format!(
            "MAP<{},{}>",
            datatype_to_mysql_type(k),
            datatype_to_mysql_type(v)
        ),
        RorisDataType::Struct(_) => "STRUCT".to_string(),
        RorisDataType::Float32Vector(dim) => format!("FLOAT32_VECTOR({})", dim),
    }
}

impl RorisQueryHandler {
    pub(crate) fn execute_statement(
        &self,
        conn_id: u32,
        stmt: &Statement,
    ) -> Result<QueryResult, String> {
        match stmt {
            Statement::ShowDatabases => self.show_databases(conn_id),
            Statement::ShowTables(db, like, is_full) => {
                self.show_tables(conn_id, db.clone(), like.clone(), *is_full)
            }
            Statement::ShowCreateTable(db, table) => {
                self.show_create_table(conn_id, db.clone(), table.clone())
            }
            Statement::ShowCreateDatabase(db) => self.show_create_database(conn_id, db),
            Statement::ShowCreateView(db, view) => {
                self.show_create_view(conn_id, db.clone(), view.clone())
            }
            Statement::Describe(db, table) => self.describe(conn_id, db.clone(), table.clone()),
            Statement::UseDatabase(db) => self.use_database(conn_id, db),
            Statement::CreateDatabase(stmt) => self.create_database(conn_id, stmt),
            Statement::CreateTable(stmt) => self.create_table(conn_id, stmt),
            Statement::DropDatabase(stmt) => self.drop_database(conn_id, stmt),
            Statement::DropTable(stmt) => self.drop_table(conn_id, stmt),
            Statement::AlterTable(stmt) => self.alter_table(conn_id, stmt),
            Statement::TruncateTable {
                database,
                table,
                if_exists,
            } => self.truncate_table(conn_id, database.clone(), table.to_string(), *if_exists),
            Statement::Insert(stmt) => self.insert(conn_id, stmt),
            Statement::Update(stmt) => self.update(conn_id, stmt),
            Statement::Delete(stmt) => self.delete(conn_id, stmt),
            // Transaction statements
            Statement::StartTransaction => self.start_transaction(conn_id),
            Statement::Commit => self.commit_tx(conn_id),
            Statement::Rollback => self.rollback_tx(conn_id),
            Statement::Savepoint(name) => self.savepoint_cmd(conn_id, name.clone()),
            Statement::RollbackTo(name) => self.rollback_to_savepoint_cmd(conn_id, name.clone()),
            Statement::ReleaseSavepoint(name) => self.release_savepoint_cmd(conn_id, name.clone()),
            Statement::SetTransactionIsolation(level) => {
                self.set_transaction_isolation(conn_id, level.clone())
            }
            Statement::Query(_) => {
                Err("Query statements should be handled by DataFusion path".to_string())
            }
            Statement::Explain(_) => {
                Err("Explain statements should be handled by DataFusion path".to_string())
            }
            Statement::ShowPartitions(db, table) => {
                self.show_partitions(conn_id, db.clone(), table.clone())
            }
            Statement::ShowTableStatus(db) => self.show_table_status(conn_id, db.clone()),
            Statement::ShowVariables { global, pattern } => {
                self.show_variables(conn_id, *global, pattern.clone())
            }
            Statement::ShowProcesslist(full) => self.show_processlist(conn_id, *full),
            Statement::ShowIndex(db, table) => self.show_index(conn_id, db.clone(), table.clone()),
            Statement::ShowAlterTable(db) => self.show_alter_table(conn_id, db.clone()),
            Statement::ShowBackends => self.show_backends(conn_id),
            Statement::ShowFrontends => self.show_frontends(conn_id),
            Statement::ShowTableId => self.show_table_id(conn_id),
            Statement::ShowPartitionId => self.show_partition_id(conn_id),
            Statement::ShowDynamicPartitionTables => self.show_dynamic_partition_tables(conn_id),
            Statement::ShowView(db, view) => self.show_view(conn_id, db.clone(), view.clone()),
            Statement::ShowCreateMaterializedView(name) => {
                self.show_create_materialized_view(conn_id, name.clone())
            }
            // Batch 2 DDL
            Statement::AlterDatabase(stmt) => self.alter_database(conn_id, stmt),
            Statement::DropView(stmt) => self.drop_view(conn_id, stmt),
            Statement::AlterView(stmt) => self.alter_view(conn_id, stmt),
            Statement::CreateIndex(stmt) => self.create_index(conn_id, stmt),
            Statement::DropIndex(stmt) => self.drop_index(conn_id, stmt),
            Statement::CancelAlterTable(stmt) => self.cancel_alter_table(conn_id, stmt),
            Statement::AlterColocateGroup(stmt) => self.alter_colocate_group(conn_id, stmt),
            // Existing statements with parsers but previously missing handlers
            Statement::CreateView {
                database,
                name,
                if_not_exists,
                query,
                columns,
            } => self.create_view(
                conn_id,
                database.clone(),
                name.clone(),
                *if_not_exists,
                query.clone(),
                columns.clone(),
            ),
            Statement::CreateMaterializedView(stmt) => self.create_materialized_view(conn_id, stmt),
            Statement::DropMaterializedView(stmt) => self.drop_materialized_view(conn_id, stmt),
            Statement::AlterMaterializedView(stmt) => self.alter_materialized_view(conn_id, stmt),
            Statement::RefreshMaterializedView(stmt) => {
                self.refresh_materialized_view(conn_id, stmt)
            }
            Statement::CreateRepository(stmt) => self.create_repository(conn_id, stmt),
            Statement::DropRepository(stmt) => self.drop_repository(conn_id, stmt),
            Statement::ShowRepositories => self.show_repositories(conn_id),
            Statement::BackupDatabase(stmt) => self.backup_database(conn_id, stmt),
            Statement::RestoreDatabase(stmt) => self.restore_database(conn_id, stmt),
            Statement::ShowUsers => self.show_users(conn_id),
            Statement::CreateUser(stmt) => self.create_user(conn_id, stmt),
            Statement::DropUser(stmt) => self.drop_user(conn_id, stmt),
            Statement::CreateCatalog(stmt) => self.create_catalog(conn_id, stmt),
            Statement::DropCatalog(stmt) => self.drop_catalog(conn_id, stmt),
            Statement::ShowCatalogs => self.show_catalogs(conn_id),
            Statement::RefreshCatalog(stmt) => self.refresh_catalog(conn_id, stmt),
            Statement::SetVariable(stmt) => self.set_variable(conn_id, stmt),
            Statement::Union(_) => {
                Err("Union statements should be handled by DataFusion path".to_string())
            }
            // Batch 3/4 statements
            Statement::ExportTable(stmt) => self.export_table(conn_id, stmt),
            Statement::CancelExport(id) => self.cancel_export(conn_id, id.clone()),
            Statement::ShowExport => self.show_export(conn_id),
            Statement::CreateFunction(stmt) => self.create_function(conn_id, stmt),
            Statement::DropFunction(stmt) => self.drop_function(conn_id, stmt),
            Statement::ShowFunctions(pattern) => self.show_functions(conn_id, pattern.clone()),
            Statement::ShowCreateFunction(name) => self.show_create_function(conn_id, name.clone()),
            Statement::DescribeFunction(name) => self.describe_function(conn_id, name.clone()),
            Statement::AnalyzeTable(stmt) => self.analyze_table(conn_id, stmt),
            Statement::DropStats(stmt) => self.drop_stats(conn_id, stmt),
            Statement::ShowAnalyze(id) => self.show_analyze(conn_id, id.clone()),
            Statement::ShowStats(table) => self.show_stats(conn_id, table.clone()),
            Statement::ShowTableStats(table) => self.show_table_stats(conn_id, table.clone()),
            Statement::CreateJob(stmt) => self.create_job(conn_id, stmt),
            Statement::DropJob(name) => self.drop_job_stmt(conn_id, name.clone()),
            Statement::PauseJob(name) => self.pause_job(conn_id, name.clone()),
            Statement::ResumeJob(name) => self.resume_job_stmt(conn_id, name.clone()),
            Statement::CancelTask(id) => self.cancel_task(conn_id, id.clone()),
            Statement::InstallPlugin(stmt) => self.install_plugin(conn_id, stmt),
            Statement::UninstallPlugin(name) => self.uninstall_plugin(conn_id, name.clone()),
            Statement::ShowPlugins => self.show_plugins(conn_id),
            Statement::RecoverDatabase(name) => self.recover_database(conn_id, name.clone()),
            Statement::RecoverTable { database, table } => {
                self.recover_table(conn_id, database.clone(), table.clone())
            }
            Statement::RecoverPartition {
                database,
                table,
                partition,
            } => {
                self.recover_partition(conn_id, database.clone(), table.clone(), partition.clone())
            }
            Statement::DropCatalogRecycleBin(filter) => {
                self.drop_catalog_recycle_bin(conn_id, filter.clone())
            }
            Statement::ShowCatalogRecycleBin => self.show_catalog_recycle_bin(conn_id),
            Statement::CreateSqlBlockRule(stmt) => self.create_sql_block_rule(conn_id, stmt),
            Statement::AlterSqlBlockRule(name, props) => {
                self.alter_sql_block_rule(conn_id, name.clone(), props.clone())
            }
            Statement::DropSqlBlockRule(name) => self.drop_sql_block_rule(conn_id, name.clone()),
            Statement::ShowSqlBlockRule(filter) => self.show_sql_block_rule(filter.clone()),
            Statement::CreateRowPolicy(stmt) => self.create_row_policy(conn_id, stmt),
            Statement::DropRowPolicy {
                name,
                database,
                table,
            } => self.drop_row_policy(conn_id, name.clone(), database.clone(), table.clone()),
            Statement::ShowRowPolicy(filter) => self.show_row_policy(filter.clone()),
            Statement::KillAnalyzeJob(id) => self.kill_analyze_job(conn_id, id.clone()),
            Statement::AlterStats(table, props) => {
                self.alter_stats(conn_id, table.clone(), props.clone())
            }
            // New admin/operations statements
            Statement::ShowStatus { global, pattern } => self.show_status(*global, pattern.clone()),
            Statement::ShowEngines => self.show_engines(),
            Statement::ShowCharset => self.show_charset(),
            Statement::KillQuery(id) => self.kill_query(*id),
            Statement::KillConnection(id) => self.kill_connection(*id),
            Statement::AdminCheckTable(table) => self.admin_check_table(conn_id, table.clone()),
            Statement::AdminShowReplica => self.admin_show_replica(conn_id),
        }
    }

    // ---- show_* methods ----

    pub(crate) fn show_databases(&self, _conn_id: u32) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let databases = catalog.list_databases();
        let rows: Vec<Vec<Option<String>>> =
            databases.iter().map(|db| vec![Some(db.clone())]).collect();
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Database".to_string(),
                col_type: ColumnType::String,
            }],
            rows,
        ))
    }

    pub(crate) fn show_tables(
        &self,
        conn_id: u32,
        db: Option<String>,
        like: Option<String>,
        is_full: bool,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = db.as_deref().unwrap_or(&current_db);

        match catalog.list_tables(target_db) {
            Some(tables) => {
                let rows: Vec<Vec<Option<String>>> = tables
                    .iter()
                    .filter(|t| match &like {
                        Some(pattern) => like_match(pattern, t),
                        None => true,
                    })
                    .map(|t| {
                        if is_full {
                            vec![Some(t.clone()), Some("BASE TABLE".to_string())]
                        } else {
                            vec![Some(t.clone())]
                        }
                    })
                    .collect();
                let columns = if is_full {
                    vec![
                        ColumnDef {
                            name: format!("Tables_in_{}", target_db),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Table_type".to_string(),
                            col_type: ColumnType::String,
                        },
                    ]
                } else {
                    vec![ColumnDef {
                        name: "Tables".to_string(),
                        col_type: ColumnType::String,
                    }]
                };
                Ok(QueryResult::with_rows(columns, rows))
            }
            None => Err(format!("Database '{}' not found", target_db)),
        }
    }

    pub(crate) fn describe(
        &self,
        conn_id: u32,
        db: String,
        table: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                let rows: Vec<Vec<Option<String>>> = tbl
                    .columns
                    .iter()
                    .map(|col| {
                        vec![
                            Some(col.name.clone()),
                            Some(datatype_to_mysql_type(&col.data_type)),
                            Some(if col.nullable { "YES" } else { "NO" }.to_string()),
                            match &col.default_value {
                                Some(v) => Some(format!("{:?}", v)),
                                None => None,
                            },
                            Some("".to_string()), // Key (empty for now)
                            Some("".to_string()), // Extra
                            Some(col.comment.clone()),
                        ]
                    })
                    .collect();
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef {
                            name: "Field".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Type".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Null".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Default".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Key".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Extra".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Comment".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Table '{}.{}' not found", target_db, table)),
        }
    }

    pub(crate) fn use_database(&self, conn_id: u32, db: &str) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        if catalog.get_database(db).is_some() {
            self.set_current_database(conn_id, db.to_string());
            Ok(QueryResult::ok())
        } else {
            Err(format!("Unknown database '{}'", db))
        }
    }

    pub(crate) fn show_create_table(
        &self,
        conn_id: u32,
        db: String,
        table: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                // Build CREATE TABLE statement from table metadata
                let mut create_sql = format!("CREATE TABLE `{}` (\n", table);
                for (i, col) in tbl.columns.iter().enumerate() {
                    let mysql_type = datatype_to_mysql_type(&col.data_type);
                    let nullable = if col.nullable { "" } else { " NOT NULL" };
                    let default_val = col
                        .default_value
                        .as_ref()
                        .map(|v| format!(" DEFAULT {:?}", v))
                        .unwrap_or_default();
                    let comment = if col.comment.is_empty() {
                        String::new()
                    } else {
                        format!(" COMMENT '{}'", col.comment.replace('\'', "\\'"))
                    };
                    let comma = if i < tbl.columns.len() - 1 { "," } else { "" };
                    create_sql.push_str(&format!(
                        "  `{}` {}{}{}{}{}\n",
                        col.name, mysql_type, nullable, default_val, comment, comma
                    ));
                }
                // Add UNIQUE KEY definitions
                for uk in &tbl.unique_keys {
                    let comma = if create_sql.ends_with('\n') { "," } else { "" };
                    if let Some(ref name) = uk.name {
                        create_sql.push_str(&format!(
                            "{}  UNIQUE KEY `{}` ({})\n",
                            comma,
                            name,
                            uk.columns.join(", ")
                        ));
                    } else {
                        create_sql.push_str(&format!(
                            "{}  UNIQUE ({})\n",
                            comma,
                            uk.columns.join(", ")
                        ));
                    }
                }
                create_sql.push_str(") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");
                if let Some(dist) = &tbl.distribution_info {
                    create_sql.push_str(&format!(
                        " DISTRIBUTED BY HASH({}) BUCKETS {}",
                        dist.columns.join(", "),
                        dist.buckets
                    ));
                }
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef {
                            name: "Table".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Create Table".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    vec![vec![Some(table.clone()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    pub(crate) fn show_create_database(
        &self,
        _conn_id: u32,
        db: &str,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        match catalog.get_database(db) {
            Some(database) => {
                let create_sql = database
                    .create_sql
                    .clone()
                    .unwrap_or_else(|| format!("CREATE DATABASE `{}`", db));
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef {
                            name: "Database".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Create Database".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    vec![vec![Some(db.to_string()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown database '{}'", db)),
        }
    }

    pub(crate) fn show_create_view(
        &self,
        conn_id: u32,
        db: String,
        view: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
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
                        ColumnDef {
                            name: "View".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Create View".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    vec![vec![Some(view.clone()), Some(create_sql)]],
                ))
            }
            None => Err(format!("Unknown view '{}.{}'", target_db, view)),
        }
    }

    pub(crate) fn show_partitions(
        &self,
        conn_id: u32,
        db: String,
        table: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                if let Some(partition_info) = &tbl.partition_info {
                    let rows: Vec<Vec<Option<String>>> = partition_info
                        .partitions
                        .iter()
                        .map(|p| {
                            vec![
                                Some(p.name.clone()),
                                p.range_start.clone(),
                                p.range_end.clone(),
                            ]
                        })
                        .collect();
                    Ok(QueryResult::with_rows(
                        vec![
                            ColumnDef {
                                name: "PartitionName".to_string(),
                                col_type: ColumnType::String,
                            },
                            ColumnDef {
                                name: "RangeStart".to_string(),
                                col_type: ColumnType::String,
                            },
                            ColumnDef {
                                name: "RangeEnd".to_string(),
                                col_type: ColumnType::String,
                            },
                        ],
                        rows,
                    ))
                } else {
                    Ok(QueryResult::with_rows(
                        vec![ColumnDef {
                            name: "Message".to_string(),
                            col_type: ColumnType::String,
                        }],
                        vec![vec![Some("No partitions defined for table".to_string())]],
                    ))
                }
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    pub(crate) fn show_table_status(
        &self,
        conn_id: u32,
        db: Option<String>,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = db.as_deref().unwrap_or(&current_db);

        match catalog.list_tables(target_db) {
            Some(tables) => {
                let mut rows = Vec::new();
                for table_name in tables {
                    if let Some(tbl) = catalog.get_table(target_db, &table_name) {
                        // Try to get actual row count and data size from storage
                        let (row_count, data_size) = self
                            .get_table_stats(target_db, &table_name)
                            .unwrap_or((tbl.row_count, tbl.data_size));

                        rows.push(vec![
                            Some(table_name.clone()),
                            Some("InnoDB".to_string()),
                            Some(row_count.to_string()),
                            Some(data_size.to_string()),
                            Some("utf8mb4_general_ci".to_string()),
                            None, // Comment
                            None, // Create_options
                        ]);
                    }
                }
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef {
                            name: "Name".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Engine".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Row_count".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Data_length".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Collation".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Comment".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Create_options".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Database '{}' not found", target_db)),
        }
    }

    /// Get actual row count and data size from Parquet file
    fn get_table_stats(&self, db: &str, table: &str) -> Option<(u64, u64)> {
        let table_dir = self.storage.table_dir(db, table);
        let parquet_path = table_dir.join("data.parquet");

        if !parquet_path.exists() {
            return Some((0, 0));
        }

        // Get file size
        let metadata = std::fs::metadata(&parquet_path).ok()?;
        let data_size = metadata.len();

        // Read Parquet metadata to get row count
        let file = std::fs::File::open(&parquet_path).ok()?;
        let builder =
            parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
        let row_count = builder.metadata().file_metadata().num_rows();

        Some((row_count as u64, data_size))
    }

    pub(crate) fn show_variables(
        &self,
        conn_id: u32,
        global: bool,
        pattern: Option<String>,
    ) -> Result<QueryResult, String> {
        let vars = if global {
            self.sys_vars.match_like(pattern.as_deref(), None)
        } else {
            self.with_session_mut(conn_id, |s| {
                self.sys_vars
                    .match_like(pattern.as_deref(), Some(&s.session_vars))
            })
        };
        let rows: Vec<Vec<Option<String>>> = vars
            .iter()
            .map(|(name, value)| vec![Some(name.clone()), Some(value.clone())])
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Variable_name".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Value".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_processlist(
        &self,
        _conn_id: u32,
        _full: bool,
    ) -> Result<QueryResult, String> {
        let conns = self.connection_tracker.list();
        let rows: Vec<Vec<Option<String>>> = conns
            .iter()
            .map(|c| {
                let time = c.connected_at.elapsed().as_secs().to_string();
                let info = c.current_sql.clone();
                vec![
                    Some(c.id.to_string()),
                    Some(c.user.clone()),
                    Some(c.host.clone()),
                    c.db.clone(),
                    Some(c.command.clone()),
                    Some(time),
                    Some(c.state.clone()),
                    info,
                ]
            })
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Id".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "User".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Host".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "db".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Command".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Time".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "State".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Info".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_status(
        &self,
        _global: bool,
        pattern: Option<String>,
    ) -> Result<QueryResult, String> {
        let ct = &self.connection_tracker;
        let db_count = self.catalog.list_databases().len();
        let table_count: usize = self
            .catalog
            .list_databases()
            .iter()
            .filter_map(|db| self.catalog.list_tables(db))
            .map(|tables| tables.len())
            .sum();

        let mut status_vars = vec![
            ("Uptime".to_string(), ct.uptime_seconds().to_string()),
            ("Queries".to_string(), ct.total_queries().to_string()),
            (
                "Threads_connected".to_string(),
                ct.active_connections().to_string(),
            ),
            (
                "Threads_running".to_string(),
                ct.active_queries().to_string(),
            ),
            (
                "Connections".to_string(),
                ct.total_connections().to_string(),
            ),
            (
                "Max_used_connections".to_string(),
                ct.peak_connections().to_string(),
            ),
            ("Slow_queries".to_string(), ct.slow_queries().to_string()),
            ("Database_count".to_string(), db_count.to_string()),
            ("Table_count".to_string(), table_count.to_string()),
            (
                "Version".to_string(),
                self.sys_vars.get("version", None).unwrap_or_default(),
            ),
            (
                "Version_comment".to_string(),
                self.sys_vars
                    .get("version_comment", None)
                    .unwrap_or_default(),
            ),
        ];

        // Filter by LIKE pattern
        if let Some(ref pat) = pattern {
            let pat_lower = pat.to_lowercase();
            status_vars.retain(|(name, _)| like_match(&pat_lower, &name.to_lowercase()));
        }

        let rows: Vec<Vec<Option<String>>> = status_vars
            .iter()
            .map(|(name, value)| vec![Some(name.clone()), Some(value.clone())])
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Variable_name".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Value".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_engines(&self) -> Result<QueryResult, String> {
        // RorisDB uses a single storage engine based on Parquet
        let rows = vec![
            vec![
                Some("InnoDB".to_string()),
                Some("DEFAULT".to_string()),
                Some("RorisDB Parquet storage engine".to_string()),
                Some("NO".to_string()),
                Some("NO".to_string()),
                Some("NO".to_string()),
            ],
            vec![
                Some("MEMORY".to_string()),
                Some("".to_string()),
                Some("In-memory storage (not persistent)".to_string()),
                Some("YES".to_string()),
                Some("NO".to_string()),
                Some("NO".to_string()),
            ],
        ];

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Engine".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Support".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Comment".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Transactions".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "XA".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Savepoints".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_charset(&self) -> Result<QueryResult, String> {
        // Return common MySQL character sets
        let rows = vec![
            vec![
                Some("utf8mb4".to_string()),
                Some("UTF-8 Unicode".to_string()),
                Some("utf8mb4_general_ci".to_string()),
                Some("4".to_string()),
            ],
            vec![
                Some("utf8".to_string()),
                Some("UTF-8 Unicode".to_string()),
                Some("utf8_general_ci".to_string()),
                Some("3".to_string()),
            ],
            vec![
                Some("latin1".to_string()),
                Some("cp1252 West European".to_string()),
                Some("latin1_swedish_ci".to_string()),
                Some("1".to_string()),
            ],
            vec![
                Some("binary".to_string()),
                Some("Binary pseudo charset".to_string()),
                Some("binary".to_string()),
                Some("1".to_string()),
            ],
        ];

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Charset".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Description".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Default collation".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Maxlen".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn kill_query(&self, id: u64) -> Result<QueryResult, String> {
        if self.connection_tracker.kill(id as u32) {
            Ok(QueryResult::with_rows(
                vec![ColumnDef {
                    name: "Status".to_string(),
                    col_type: ColumnType::String,
                }],
                vec![vec![Some(format!("Query {} killed", id))]],
            ))
        } else {
            Err(format!("Unknown connection ID: {}", id))
        }
    }

    pub(crate) fn kill_connection(&self, id: u64) -> Result<QueryResult, String> {
        if self.connection_tracker.kill(id as u32) {
            Ok(QueryResult::with_rows(
                vec![ColumnDef {
                    name: "Status".to_string(),
                    col_type: ColumnType::String,
                }],
                vec![vec![Some(format!("Connection {} killed", id))]],
            ))
        } else {
            Err(format!("Unknown connection ID: {}", id))
        }
    }

    pub(crate) fn admin_check_table(
        &self,
        conn_id: u32,
        table_ref: String,
    ) -> Result<QueryResult, String> {
        let (db, tbl) = if table_ref.contains('.') {
            let parts: Vec<&str> = table_ref.splitn(2, '.').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (self.get_session(conn_id).clone(), table_ref)
        };

        match self.storage.read(&db, &tbl) {
            Ok(batch) => Ok(QueryResult::with_rows(
                vec![
                    ColumnDef {
                        name: "Table".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Op".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Msg_type".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Msg_text".to_string(),
                        col_type: ColumnType::String,
                    },
                ],
                vec![vec![
                    Some(format!("{}.{}", db, tbl)),
                    Some("check".to_string()),
                    Some("status".to_string()),
                    Some(format!(
                        "OK ({} rows, {} columns)",
                        batch.num_rows(),
                        batch.num_columns()
                    )),
                ]],
            )),
            Err(e) => Ok(QueryResult::with_rows(
                vec![
                    ColumnDef {
                        name: "Table".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Op".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Msg_type".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Msg_text".to_string(),
                        col_type: ColumnType::String,
                    },
                ],
                vec![vec![
                    Some(format!("{}.{}", db, tbl)),
                    Some("check".to_string()),
                    Some("error".to_string()),
                    Some(format!("FAILED: {}", e)),
                ]],
            )),
        }
    }

    pub(crate) fn admin_show_replica(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Mode".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Replicas".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Status".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            vec![vec![
                Some("Single Node".to_string()),
                Some("1".to_string()),
                Some("No replicas configured (single-node mode)".to_string()),
            ]],
        ))
    }

    pub(crate) fn show_index(
        &self,
        conn_id: u32,
        db: String,
        table: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let target_db = if db.is_empty() { &current_db } else { &db };

        match catalog.get_table(target_db, &table) {
            Some(tbl) => {
                let rows: Vec<Vec<Option<String>>> = tbl
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, col)| {
                        vec![
                            Some(table.clone()),
                            Some("0".to_string()),
                            Some(col.name.clone()),
                            Some((i + 1).to_string()),
                            None,
                            None,
                            Some(if col.nullable {
                                "YES".to_string()
                            } else {
                                "NO".to_string()
                            }),
                            None,
                            None,
                            Some("".to_string()),
                            Some("BTREE".to_string()),
                            Some("".to_string()),
                            Some("".to_string()),
                        ]
                    })
                    .collect();

                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef {
                            name: "Table".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Non_unique".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Key_name".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Seq_in_index".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Column_name".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Collation".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Null".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Index_type".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Comment".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Index_comment".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Algorithm".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Is_visible".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Expression".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Unknown table '{}.{}'", target_db, table)),
        }
    }

    pub(crate) fn show_alter_table(
        &self,
        _conn_id: u32,
        _db: Option<String>,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Message".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(
                "No ALTER TABLE operations in progress".to_string(),
            )]],
        ))
    }

    pub(crate) fn show_backends(&self, _conn_id: u32) -> Result<QueryResult, String> {
        let backends = vec![(
            "1".to_string(),
            "127.0.0.1".to_string(),
            "9060".to_string(),
            "true".to_string(),
            "0".to_string(),
            "0".to_string(),
        )];
        let rows: Vec<Vec<Option<String>>> = backends
            .into_iter()
            .map(|(id, host, port, alive, tablet_num, data_size)| {
                vec![
                    Some(id),
                    Some(host),
                    Some(port),
                    Some(alive),
                    Some(tablet_num),
                    Some(data_size),
                ]
            })
            .collect();
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "BackendId".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Host".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Port".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Alive".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "TabletNum".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "DataSize".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_frontends(&self, _conn_id: u32) -> Result<QueryResult, String> {
        let frontends = vec![(
            "fe1".to_string(),
            "127.0.0.1".to_string(),
            "9030".to_string(),
            "true".to_string(),
            "false".to_string(),
            "0".to_string(),
        )];
        let rows: Vec<Vec<Option<String>>> = frontends
            .into_iter()
            .map(|(name, ip, port, alive, join, disk)| {
                vec![
                    Some(name),
                    Some(ip),
                    Some(port),
                    Some(alive),
                    Some(join),
                    Some(disk),
                ]
            })
            .collect();
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Name".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "IP".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Port".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Alive".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Join".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Disk".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_table_id(&self, _conn_id: u32) -> Result<QueryResult, String> {
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
                ColumnDef {
                    name: "Database".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Table".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "TableId".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_partition_id(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "PartitionId".to_string(),
                col_type: ColumnType::String,
            }],
            vec![],
        ))
    }

    pub(crate) fn show_dynamic_partition_tables(
        &self,
        _conn_id: u32,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Message".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some("No dynamic partition tables".to_string())]],
        ))
    }

    pub(crate) fn show_view(
        &self,
        conn_id: u32,
        db: String,
        view: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
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
                        ColumnDef {
                            name: "View".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Database".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "Definition".to_string(),
                            col_type: ColumnType::String,
                        },
                        ColumnDef {
                            name: "CharacterSet".to_string(),
                            col_type: ColumnType::String,
                        },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Unknown view '{}.{}'", target_db, view)),
        }
    }

    pub(crate) fn show_create_materialized_view(
        &self,
        conn_id: u32,
        name: String,
    ) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);

        if let Some(mv) = catalog.get_materialized_view(&current_db, &name) {
            let create_sql = format!(
                "CREATE MATERIALIZED VIEW `{}` AS {}",
                mv.name, mv.definition
            );
            Ok(QueryResult::with_rows(
                vec![
                    ColumnDef {
                        name: "MaterializedView".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Create Materialized View".to_string(),
                        col_type: ColumnType::String,
                    },
                ],
                vec![vec![Some(name), Some(create_sql)]],
            ))
        } else {
            Err(format!("Unknown materialized view '{}'", name))
        }
    }

    pub(crate) fn show_repositories(&self, _conn_id: u32) -> Result<QueryResult, String> {
        let repos = self.backup_manager.list_repositories();
        let rows: Vec<Vec<Option<String>>> = repos
            .iter()
            .map(|name| {
                let path = self
                    .backup_manager
                    .get_repo_path(name)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                vec![Some(name.clone()), Some(path)]
            })
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Name".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Path".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            rows,
        ))
    }

    pub(crate) fn show_users(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "User".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some("root".to_string())]],
        ))
    }

    pub(crate) fn show_catalogs(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Catalog".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some("internal".to_string())]],
        ))
    }

    pub(crate) fn show_export(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Export".to_string(),
                col_type: ColumnType::String,
            }],
            vec![],
        ))
    }

    pub(crate) fn show_functions(
        &self,
        _conn_id: u32,
        pattern: Option<String>,
    ) -> Result<QueryResult, String> {
        let _ = pattern;
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Function".to_string(),
                col_type: ColumnType::String,
            }],
            vec![],
        ))
    }

    pub(crate) fn show_create_function(
        &self,
        _conn_id: u32,
        name: String,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef {
                    name: "Function".to_string(),
                    col_type: ColumnType::String,
                },
                ColumnDef {
                    name: "Create Function".to_string(),
                    col_type: ColumnType::String,
                },
            ],
            vec![vec![
                Some(name.clone()),
                Some(format!("CREATE FUNCTION {}", name)),
            ]],
        ))
    }

    pub(crate) fn describe_function(
        &self,
        _conn_id: u32,
        name: String,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Function".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(name)]],
        ))
    }

    pub(crate) fn show_analyze(
        &self,
        _conn_id: u32,
        id: Option<String>,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Analyze".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(id.unwrap_or_default())]],
        ))
    }

    pub(crate) fn show_stats(&self, _conn_id: u32, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Table".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(table)]],
        ))
    }

    pub(crate) fn show_table_stats(
        &self,
        _conn_id: u32,
        table: String,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Table".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(table)]],
        ))
    }

    pub(crate) fn show_plugins(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Plugin".to_string(),
                col_type: ColumnType::String,
            }],
            vec![],
        ))
    }

    pub(crate) fn show_catalog_recycle_bin(&self, _conn_id: u32) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "RecycleBin".to_string(),
                col_type: ColumnType::String,
            }],
            vec![],
        ))
    }

    pub(crate) fn show_sql_block_rule(
        &self,
        filter: Option<String>,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Rule".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }

    pub(crate) fn show_row_policy(&self, filter: Option<String>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "Policy".to_string(),
                col_type: ColumnType::String,
            }],
            vec![vec![Some(filter.unwrap_or_default())]],
        ))
    }
}
