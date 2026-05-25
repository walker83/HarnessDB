use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::DataType as ADT;
use mysql_protocol::server::{ColumnDef, ColumnType};
use mysql_protocol::QueryResult;
use fe_sql_parser::Statement;
use ::types::DataType as RorisDataType;

use crate::handler_struct::RorisQueryHandler;
use crate::utils::{like_match, parse_data_type};

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
        RorisDataType::Map(k, v) => format!("MAP<{},{}>", datatype_to_mysql_type(k), datatype_to_mysql_type(v)),
        RorisDataType::Struct(_) => "STRUCT".to_string(),
        RorisDataType::Float32Vector(dim) => format!("FLOAT32_VECTOR({})", dim),
    }
}

impl RorisQueryHandler {
    pub(crate) fn execute_statement(&self, stmt: &Statement) -> Result<QueryResult, String> {
        match stmt {
            Statement::ShowDatabases => self.show_databases(),
            Statement::ShowTables(db, like) => self.show_tables(db.clone(), like.clone()),
            Statement::ShowCreateTable(db, table) => self.show_create_table(db.clone(), table.clone()),
            Statement::ShowCreateDatabase(db) => self.show_create_database(db),
            Statement::ShowCreateView(db, view) => self.show_create_view(db.clone(), view.clone()),
            Statement::Describe(db, table) => self.describe(db.clone(), table.clone()),
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
            // New admin/operations statements
            Statement::ShowStatus { global, pattern } => self.show_status(*global, pattern.clone()),
            Statement::KillQuery(id) => self.kill_query(*id),
            Statement::KillConnection(id) => self.kill_connection(*id),
            Statement::AdminCheckTable(table) => self.admin_check_table(table.clone()),
            Statement::AdminShowReplica => self.admin_show_replica(),
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
                        Some(datatype_to_mysql_type(&col.data_type)),
                        Some(if col.nullable { "YES" } else { "NO" }.to_string()),
                        match &col.default_value {
                            Some(v) => Some(format!("{:?}", v)),
                            None => None,
                        },
                        Some("".to_string()),  // Key (empty for now)
                        Some("".to_string()),  // Extra
                        Some(col.comment.clone()),
                    ]
                }).collect();
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Field".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Type".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Null".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Default".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Key".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Extra".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Comment".to_string(), col_type: ColumnType::String },
                    ],
                    rows,
                ))
            }
            None => Err(format!("Table '{}.{}' not found", target_db, table)),
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
                    let mysql_type = datatype_to_mysql_type(&col.data_type);
                    let nullable = if col.nullable { "" } else { " NOT NULL" };
                    let default_val = col.default_value.as_ref()
                        .map(|v| format!(" DEFAULT {:?}", v))
                        .unwrap_or_default();
                    let comment = if col.comment.is_empty() {
                        String::new()
                    } else {
                        format!(" COMMENT '{}'", col.comment.replace('\'', "\\'"))
                    };
                    let comma = if i < tbl.columns.len() - 1 { "," } else { "" };
                    create_sql.push_str(&format!("  `{}` {}{}{}{}{}\n", col.name, mysql_type, nullable, default_val, comment, comma));
                }
                // Add UNIQUE KEY definitions
                for uk in &tbl.unique_keys {
                    let comma = if create_sql.ends_with('\n') { "," } else { "" };
                    if let Some(ref name) = uk.name {
                        create_sql.push_str(&format!("{}  UNIQUE KEY `{}` ({})\n", comma, name, uk.columns.join(", ")));
                    } else {
                        create_sql.push_str(&format!("{}  UNIQUE ({})\n", comma, uk.columns.join(", ")));
                    }
                }
                create_sql.push_str(") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");
                if let Some(dist) = &tbl.distribution_info {
                    create_sql.push_str(&format!(" DISTRIBUTED BY HASH({}) BUCKETS {}",
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
                        // Try to get actual row count and data size from storage
                        let (row_count, data_size) = self.get_table_stats(target_db, &table_name)
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
        let builder = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
        let row_count = builder.metadata().file_metadata().num_rows();

        Some((row_count as u64, data_size))
    }

    pub(crate) fn show_variables(&self, global: bool, pattern: Option<String>) -> Result<QueryResult, String> {
        let session = if global { None } else { Some(self.session_vars.read()) };
        let session_ref = session.as_deref();
        let vars = self.sys_vars.match_like(pattern.as_deref(), session_ref);
        let rows: Vec<Vec<Option<String>>> = vars.iter()
            .map(|(name, value)| vec![Some(name.clone()), Some(value.clone())])
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Variable_name".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Value".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    pub(crate) fn show_processlist(&self, _full: bool) -> Result<QueryResult, String> {
        let conns = self.connection_tracker.list();
        let rows: Vec<Vec<Option<String>>> = conns.iter().map(|c| {
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
        }).collect();

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

    pub(crate) fn show_status(&self, _global: bool, pattern: Option<String>) -> Result<QueryResult, String> {
        let ct = &self.connection_tracker;
        let db_count = self.catalog.list_databases().len();
        let table_count: usize = self.catalog.list_databases().iter()
            .filter_map(|db| self.catalog.list_tables(db))
            .map(|tables| tables.len())
            .sum();

        let mut status_vars = vec![
            ("Uptime".to_string(), ct.uptime_seconds().to_string()),
            ("Queries".to_string(), ct.total_queries().to_string()),
            ("Threads_connected".to_string(), ct.active_connections().to_string()),
            ("Threads_running".to_string(), ct.active_queries().to_string()),
            ("Connections".to_string(), ct.total_connections().to_string()),
            ("Max_used_connections".to_string(), ct.peak_connections().to_string()),
            ("Slow_queries".to_string(), ct.slow_queries().to_string()),
            ("Database_count".to_string(), db_count.to_string()),
            ("Table_count".to_string(), table_count.to_string()),
            ("Version".to_string(), self.sys_vars.get("version", None).unwrap_or_default()),
            ("Version_comment".to_string(), self.sys_vars.get("version_comment", None).unwrap_or_default()),
        ];

        // Filter by LIKE pattern
        if let Some(ref pat) = pattern {
            let pat_lower = pat.to_lowercase();
            status_vars.retain(|(name, _)| like_match(&pat_lower, &name.to_lowercase()));
        }

        let rows: Vec<Vec<Option<String>>> = status_vars.iter()
            .map(|(name, value)| vec![Some(name.clone()), Some(value.clone())])
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Variable_name".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Value".to_string(), col_type: ColumnType::String },
            ],
            rows,
        ))
    }

    pub(crate) fn kill_query(&self, id: u64) -> Result<QueryResult, String> {
        if self.connection_tracker.kill(id as u32) {
            Ok(QueryResult::with_rows(
                vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
                vec![vec![Some(format!("Query {} killed", id))]],
            ))
        } else {
            Err(format!("Unknown connection ID: {}", id))
        }
    }

    pub(crate) fn kill_connection(&self, id: u64) -> Result<QueryResult, String> {
        if self.connection_tracker.kill(id as u32) {
            Ok(QueryResult::with_rows(
                vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
                vec![vec![Some(format!("Connection {} killed", id))]],
            ))
        } else {
            Err(format!("Unknown connection ID: {}", id))
        }
    }

    pub(crate) fn admin_check_table(&self, table_ref: String) -> Result<QueryResult, String> {
        let (db, tbl) = if table_ref.contains('.') {
            let parts: Vec<&str> = table_ref.splitn(2, '.').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (self.current_database.read().clone(), table_ref)
        };

        match self.storage.read(&db, &tbl) {
            Ok(batch) => {
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Op".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Msg_type".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Msg_text".to_string(), col_type: ColumnType::String },
                    ],
                    vec![vec![
                        Some(format!("{}.{}", db, tbl)),
                        Some("check".to_string()),
                        Some("status".to_string()),
                        Some(format!("OK ({} rows, {} columns)", batch.num_rows(), batch.num_columns())),
                    ]],
                ))
            }
            Err(e) => {
                Ok(QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Table".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Op".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Msg_type".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Msg_text".to_string(), col_type: ColumnType::String },
                    ],
                    vec![vec![
                        Some(format!("{}.{}", db, tbl)),
                        Some("check".to_string()),
                        Some("error".to_string()),
                        Some(format!("FAILED: {}", e)),
                    ]],
                ))
            }
        }
    }

    pub(crate) fn admin_show_replica(&self) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Mode".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Replicas".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Status".to_string(), col_type: ColumnType::String },
            ],
            vec![vec![
                Some("Single Node".to_string()),
                Some("1".to_string()),
                Some("No replicas configured (single-node mode)".to_string()),
            ]],
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
            ("fe1".to_string(), "127.0.0.1".to_string(), "9030".to_string(), "true".to_string(), "false".to_string(), "0".to_string()),
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
        let repos = self.backup_manager.list_repositories();
        let rows: Vec<Vec<Option<String>>> = repos.iter()
            .map(|name| {
                let path = self.backup_manager.get_repo_path(name)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                vec![Some(name.clone()), Some(path)]
            })
            .collect();

        Ok(QueryResult::with_rows(
            vec![
                ColumnDef { name: "Name".to_string(), col_type: ColumnType::String },
                ColumnDef { name: "Path".to_string(), col_type: ColumnType::String },
            ],
            rows,
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
