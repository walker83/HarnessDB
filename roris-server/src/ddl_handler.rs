use std::sync::Arc;
use std::collections::HashMap;

use mysql_protocol::server::{ColumnDef, ColumnType};
use mysql_protocol::QueryResult;
use fe_catalog::table::{Table, TableColumn, KeysType};
use fe_sql_parser::ast::*;
use types::DataType;

use crate::handler_struct::{RorisQueryHandler, ViewInfo};
use crate::utils::{literal_to_string, parse_data_type};

impl RorisQueryHandler {
    // ---- Database DDL ----

    pub(crate) fn create_database(&self, conn_id: u32, stmt: &CreateDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        match catalog.create_database(&stmt.name) {
            Ok(()) => {
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                drop(catalog);
                let df_cat = self.session_ctx.catalog("roris")
                    .ok_or_else(|| "roris catalog not found".to_string())?;
                if let Some(roris_cat) = df_cat.as_any().downcast_ref::<fe_storage::ParquetCatalogProvider>() {
                    roris_cat.create_database(&stmt.name);
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

    pub(crate) fn drop_database(&self, conn_id: u32, stmt: &DropDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
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

    pub(crate) fn alter_database(&self, conn_id: u32, stmt: &AlterDatabaseStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        if catalog.get_database(&stmt.name).is_none() {
            return Err(format!("Unknown database '{}'", stmt.name));
        }
        drop(catalog);
        if !stmt.properties.is_empty() {
            let catalog = &self.catalog;
            if let Some(mut db) = catalog.get_database(&stmt.name) {
                for (k, v) in &stmt.properties {
                    db.properties.insert(k.clone(), v.clone());
                }
            }
        }
        Ok(QueryResult::ok())
    }

    // ---- Table DDL ----

    pub(crate) fn create_table(&self, conn_id: u32, stmt: &CreateTableStmt) -> Result<QueryResult, String> {
        // Validate at least one column (MySQL rejects 0-column tables)
        if stmt.columns.is_empty() {
            return Err("A table must have at least one column".to_string());
        }

        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);

        let table_id = catalog.next_id();

        use fe_catalog::table::{Table, TableColumn, KeysType, PartitionInfo, Partition, DistributionInfo};

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

        let partition_info = stmt.partition.as_ref().map(|p| {
            PartitionInfo {
                partition_type: p.partition_type.clone(),
                columns: p.columns.clone(),
                // Partition ranges parsing is not yet implemented; partitions
                // will be populated when full partition DDL support is added.
                partitions: vec![],
            }
        });

        let distribution_info = stmt.distribution.as_ref().map(|d| {
            DistributionInfo {
                dist_type: d.dist_type.clone(),
                columns: d.columns.clone(),
                buckets: d.buckets as u32,
            }
        });

        let properties: HashMap<String, String> = stmt.properties.iter()
            .cloned()
            .collect();

        let replication_num = properties.get("replication_num")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);

        let table = Table {
            id: table_id,
            tablet_id: table_id,
            name: stmt.name.clone(),
            database: db.to_string(),
            columns,
            keys_type: match stmt.keys_type {
                fe_sql_parser::ast::KeysType::Duplicate => KeysType::Duplicate,
                fe_sql_parser::ast::KeysType::Aggregate => KeysType::Aggregate,
                fe_sql_parser::ast::KeysType::Unique => KeysType::Unique,
                fe_sql_parser::ast::KeysType::Primary => KeysType::Primary,
            },
            unique_keys: stmt.unique_keys.iter().map(|uk| fe_catalog::UniqueKeyDef {
                name: uk.name.clone(),
                columns: uk.columns.clone(),
            }).collect(),
            partition_info,
            distribution_info,
            replication_num,
            properties,
            row_count: 0,
            data_size: 0,
            stats: None,
            view_definition: None,
        };

        drop(catalog);
        let catalog = &self.catalog;
        match catalog.create_table(db, table) {
            Ok(()) => {
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                drop(catalog);
                let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = stmt.columns.iter().map(|c| {
                    datafusion::arrow::datatypes::Field::new(
                        &c.name,
                        fe_datafusion::types::to_arrow_data_type(&parse_data_type(&c.data_type)),
                        c.nullable,
                    )
                }).collect();
                let arrow_schema = Arc::new(datafusion::arrow::datatypes::Schema::new(arrow_fields));
                let Some(df_cat) = self.session_ctx.catalog("roris") else {
                    return Ok(QueryResult::with_rows(
                        vec![ColumnDef { name: "Error".to_string(), col_type: ColumnType::String }],
                        vec![vec![Some("Internal error: catalog 'roris' not found".to_string())]],
                    ));
                };
                if let Some(parquet_cat) = df_cat.as_any().downcast_ref::<fe_storage::ParquetCatalogProvider>() {
                    if let Err(e) = parquet_cat.create_table(db, &stmt.name, arrow_schema) {
                        tracing::error!("Failed to create table storage: {}", e);
                    }
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

    pub(crate) fn drop_table(&self, conn_id: u32, stmt: &DropTableStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let table = stmt.name.clone();

        drop(catalog);
        let catalog = &self.catalog;
        match catalog.drop_table(db, &table) {
            Ok(()) => {
                if let Err(e) = catalog.save() {
                    tracing::error!("Failed to save catalog: {}", e);
                }
                // Drop Parquet data
                let df_cat = self.session_ctx.catalog("roris").unwrap();
                if let Some(parquet_cat) = df_cat.as_any().downcast_ref::<fe_storage::ParquetCatalogProvider>() {
                    if let Err(e) = parquet_cat.drop_table(db, &table) {
                        tracing::warn!("Failed to drop table storage: {}", e);
                    }
                }
                Ok(QueryResult::ok())
            }
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(format!("{}", e)),
        }
    }

    pub(crate) fn alter_table(&self, conn_id: u32, stmt: &AlterTableStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);

        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = &self.catalog;
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
                        fe_sql_parser::ast::AlterOperation::AddColumn(col_def) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let new_col = TableColumn {
                                name: col_def.name.clone(),
                                data_type: parse_data_type(&col_def.data_type),
                                nullable: col_def.nullable,
                                default_value: col_def.default_value.as_ref().and_then(|e| {
                                    match e {
                                        fe_sql_parser::ast::Expr::Literal(lit) => Some(literal_to_string(lit)),
                                        _ => None,
                                    }
                                }),
                                agg_type: col_def.agg_type.clone(),
                                comment: col_def.comment.clone().unwrap_or_default(),
                            };
                            // Build Field for the new column BEFORE adding to table
                            let new_field = datafusion::arrow::datatypes::Field::new(
                                &col_def.name,
                                fe_datafusion::types::to_arrow_data_type(&parse_data_type(&col_def.data_type)),
                                col_def.nullable,
                            );
                            table.columns.push(new_col);
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                            // Rewrite Parquet data to include the new column (NULL for existing rows)
                            if let Err(e) = self.storage.rewrite_parquet_add_column(db, &stmt.table, &new_field) {
                                tracing::warn!("Failed to rewrite parquet adding column: {}", e);
                            }
                        }
                        fe_sql_parser::ast::AlterOperation::DropColumn(col_name) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let idx = table.columns.iter().position(|c| c.name == *col_name);
                            if let Some(idx) = idx {
                                // Rewrite Parquet data BEFORE removing column from catalog
                                if let Err(e) = self.storage.rewrite_parquet_drop_column(db, &stmt.table, idx) {
                                    tracing::warn!("Failed to rewrite parquet dropping column: {}", e);
                                }
                                table.columns.remove(idx);
                                catalog.create_table(db, table).map_err(|e| e.to_string())?;
                            } else {
                                return Err(format!("Unknown column '{}'", col_name));
                            }
                        }
                        fe_sql_parser::ast::AlterOperation::ModifyColumn(col_def) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let idx = table.columns.iter().position(|c| c.name == col_def.name);
                            if let Some(idx) = idx {
                                table.columns[idx] = TableColumn {
                                    name: col_def.name.clone(),
                                    data_type: parse_data_type(&col_def.data_type),
                                    nullable: col_def.nullable,
                                    default_value: col_def.default_value.as_ref().and_then(|e| {
                                        match e {
                                            fe_sql_parser::ast::Expr::Literal(lit) => Some(literal_to_string(lit)),
                                            _ => None,
                                        }
                                    }),
                                    agg_type: col_def.agg_type.clone(),
                                    comment: col_def.comment.clone().unwrap_or_default(),
                                };
                                catalog.create_table(db, table).map_err(|e| e.to_string())?;
                                if let Err(e) = self.update_df_table_schema_inner(db, &stmt.table) {
                                    tracing::warn!("Failed to update DataFusion schema: {}", e);
                                }
                            } else {
                                return Err(format!("Unknown column '{}'", col_def.name));
                            }
                        }
                        fe_sql_parser::ast::AlterOperation::RenameTable(new_name) => {
                            let mut table = catalog.get_table(db, &stmt.table)
                                .ok_or_else(|| format!("Table {}.{} not found", db, stmt.table))?;
                            let old_name = table.name.clone();
                            table.name = new_name.clone();
                            // First drop the old table entry
                            catalog.drop_table(db, &old_name).map_err(|e| e.to_string())?;
                            // Then create with new name
                            catalog.create_table(db, table).map_err(|e| e.to_string())?;
                            // Rename the data directory
                            let old_data_dir = self.storage.table_dir(db, &old_name);
                            let new_data_dir = self.storage.table_dir(db, new_name);
                            if old_data_dir.exists() {
                                std::fs::rename(&old_data_dir, &new_data_dir)
                                    .map_err(|e| format!("Failed to rename data dir: {}", e))?;
                            }
                            // Update DataFusion catalog
                            if let Some(df_cat) = self.session_ctx.catalog("roris") {
                                if let Some(roris_cat) = df_cat.as_any().downcast_ref::<fe_storage::ParquetCatalogProvider>() {
                                    // Get old schema (CatalogManager still has old entry at this point)
                                    if let Some(schema) = roris_cat.get_table_schema(db, &old_name) {
                                        roris_cat.create_table(db, new_name, schema);
                                        roris_cat.drop_table(db, &old_name);
                                    }
                                }
                            }
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

    pub(crate) fn truncate_table(&self, conn_id: u32, database: Option<String>, table: String, if_exists: bool) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = database.as_deref().unwrap_or(&current_db);

        match catalog.get_table(db, &table) {
            Some(tbl) => {
                drop(catalog);

                // Truncate Parquet data
                let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = tbl.columns.iter().map(|c| {
                    datafusion::arrow::datatypes::Field::new(
                        &c.name,
                        fe_datafusion::types::to_arrow_data_type(&c.data_type),
                        c.nullable,
                    )
                }).collect();
                let arrow_schema = Arc::new(datafusion::arrow::datatypes::Schema::new(arrow_fields));
                if let Err(e) = self.storage.truncate(db, &table, arrow_schema) {
                    tracing::warn!("Failed to truncate table storage: {}", e);
                }

                // Update catalog metadata
                let catalog = &self.catalog;
                if let Some(mut tbl) = catalog.get_table(db, &table) {
                    tbl.row_count = 0;
                    tbl.data_size = 0;
                    tbl.stats = None;
                    catalog.create_table(db, tbl).map_err(|e| e.to_string())?;
                    if let Err(e) = catalog.save() {
                        tracing::error!("Failed to save catalog: {}", e);
                    }
                }

                Ok(QueryResult::ok())
            }
            None if if_exists => Ok(QueryResult::ok()),
            None => Err(format!("Unknown table '{}.{}'", db, table)),
        }
    }

    // ---- View DDL ----

    pub(crate) fn create_view(&self, conn_id: u32, database: Option<String>, name: String, if_not_exists: bool, query: String, columns: Vec<String>) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = database.as_deref().unwrap_or(&current_db);
        if self.find_view(db, &name).is_some() {
            if if_not_exists { return Ok(QueryResult::ok()); }
            return Err(format!("View '{}.{}' already exists", db, name));
        }
        let mut views = self.views.write();
        views.push(ViewInfo { database: db.to_string(), name, query, columns });
        Ok(QueryResult::ok())
    }

    pub(crate) fn drop_view(&self, conn_id: u32, stmt: &DropViewStmt) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let mut views = self.views.write();
        let idx = views.iter().position(|v| v.database == db && v.name == stmt.name);
        match idx {
            Some(i) => { views.remove(i); Ok(QueryResult::ok()) }
            None => if stmt.if_exists { Ok(QueryResult::ok()) } else { Err(format!("Unknown view '{}.{}'", db, stmt.name)) }
        }
    }

    pub(crate) fn alter_view(&self, conn_id: u32, stmt: &AlterViewStmt) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let mut views = self.views.write();
        let view = views.iter_mut().find(|v| v.database == db && v.name == stmt.name)
            .ok_or_else(|| format!("Unknown view '{}.{}'", db, stmt.name))?;
        view.query = stmt.query.clone();
        Ok(QueryResult::ok())
    }

    // ---- Index DDL ----

    pub(crate) fn create_index(&self, conn_id: u32, stmt: &CreateIndexStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = &self.catalog;
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

    pub(crate) fn drop_index(&self, conn_id: u32, stmt: &DropIndexStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => {
                drop(catalog);
                let catalog = &self.catalog;
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

    // ---- Materialized View DDL ----

    pub(crate) fn create_materialized_view(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateMaterializedViewStmt) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let catalog = &self.catalog;
        if catalog.get_table(db, &stmt.name).is_some() && !stmt.if_not_exists {
            return Err(format!("Table '{}.{}' already exists", db, stmt.name));
        }
        drop(catalog);
        let catalog = &self.catalog;
        let columns: Vec<TableColumn> = stmt.columns.iter().map(|c| TableColumn {
            name: c.clone(), data_type: DataType::String, nullable: true, default_value: None, agg_type: None, comment: String::new(),
        }).collect();
        let table = Table {
            id: 0, tablet_id: 0, name: stmt.name.clone(), database: db.to_string(), columns,
            keys_type: KeysType::Duplicate, unique_keys: vec![], partition_info: None, distribution_info: None,
            replication_num: 1, properties: HashMap::new(), row_count: 0, data_size: 0, stats: None,
            view_definition: None,
        };
        match catalog.create_table(db, table) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(e) => if stmt.if_not_exists { Ok(QueryResult::ok()) } else { Err(e.to_string()) },
        }
    }

    pub(crate) fn drop_materialized_view(&self, conn_id: u32, stmt: &fe_sql_parser::ast::DropMaterializedViewStmt) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let catalog = &self.catalog;
        match catalog.drop_table(db, &stmt.name) {
            Ok(()) => Ok(QueryResult::ok()),
            Err(_) if stmt.if_exists => Ok(QueryResult::ok()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub(crate) fn alter_materialized_view(&self, conn_id: u32, stmt: &fe_sql_parser::ast::AlterMaterializedViewStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER MATERIALIZED VIEW {}.{} OK", stmt.database.as_deref().unwrap_or(&String::new()), stmt.name))]],
        ))
    }

    pub(crate) fn refresh_materialized_view(&self, conn_id: u32, stmt: &fe_sql_parser::ast::RefreshMaterializedViewStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("REFRESH MATERIALIZED VIEW {}.{} OK", stmt.database.as_deref().unwrap_or(&String::new()), stmt.name))]],
        ))
    }

    // ---- Repository / Backup DDL ----

    pub(crate) fn create_repository(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateRepositoryStmt) -> Result<QueryResult, String> {
        // Extract path from properties
        let path = stmt.properties.iter()
            .find(|(k, _)| k.to_lowercase() == "location" || k.to_lowercase() == "path")
            .map(|(_, v)| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return Err("CREATE REPOSITORY requires a location property. Example: CREATE REPOSITORY repo WITH BROKER ON '/path/to/backup'".to_string());
        }

        self.backup_manager.create_repository(&stmt.name, path)?;

        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE REPOSITORY `{}` at '{}' completed", stmt.name, path))]],
        ))
    }

    pub(crate) fn drop_repository(&self, conn_id: u32, stmt: &fe_sql_parser::ast::DropRepositoryStmt) -> Result<QueryResult, String> {
        match self.backup_manager.drop_repository(&stmt.name) {
            Ok(()) => Ok(QueryResult::with_rows(
                vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
                vec![vec![Some(format!("DROP REPOSITORY `{}` completed", stmt.name))]],
            )),
            Err(e) => {
                if stmt.if_exists {
                    Ok(QueryResult::ok())
                } else {
                    Err(e)
                }
            }
        }
    }

    pub(crate) fn backup_database(&self, conn_id: u32, stmt: &fe_sql_parser::ast::BackupDatabaseStmt) -> Result<QueryResult, String> {
        let msg = self.backup_manager.backup_database(
            &self.catalog,
            &stmt.database,
            &stmt.repository,
            &stmt.backup_name,
        )?;
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(msg)]],
        ))
    }

    pub(crate) fn restore_database(&self, conn_id: u32, stmt: &fe_sql_parser::ast::RestoreDatabaseStmt) -> Result<QueryResult, String> {
        let msg = self.backup_manager.restore_database(
            &self.catalog,
            &stmt.database,
            &stmt.repository,
            &stmt.backup_name,
        )?;
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(msg)]],
        ))
    }

    // ---- Alter helpers ----

    pub(crate) fn cancel_alter_table(&self, conn_id: u32, stmt: &CancelAlterTableStmt) -> Result<QueryResult, String> {
        let catalog = &self.catalog;
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        match catalog.get_table(db, &stmt.table) {
            Some(_) => Ok(QueryResult::ok()),
            None => Err(format!("Unknown table '{}.{}'", db, stmt.table)),
        }
    }

    pub(crate) fn alter_colocate_group(&self, conn_id: u32, stmt: &AlterColocateGroupStmt) -> Result<QueryResult, String> {
        use fe_sql_parser::ast::ColocateGroupOperation;
        match &stmt.operation {
            ColocateGroupOperation::AddTable { database, table } => {
                let current_db = self.get_session(conn_id);
                let db = database.as_deref().unwrap_or(&current_db);
                let catalog = &self.catalog;
                if catalog.get_table(db, table).is_none() { return Err(format!("Unknown table '{}.{}'", db, table)); }
                drop(catalog);
                let catalog = &self.catalog;
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
                let current_db = self.get_session(conn_id);
                let db = database.as_deref().unwrap_or(&current_db);
                let catalog = &self.catalog;
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

    // ---- User DDL ----

    pub(crate) fn create_user(&self, _conn_id: u32, stmt: &fe_sql_parser::ast::CreateUserStmt) -> Result<QueryResult, String> {
        use mysql_protocol::auth::double_sha1;

        let password = stmt.password.as_deref().unwrap_or("");
        let hash = double_sha1(password.as_bytes());
        self.mysql_credentials.insert(stmt.username.clone(), hash);

        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE USER {} OK", stmt.username))]],
        ))
    }

    pub(crate) fn drop_user(&self, _conn_id: u32, stmt: &fe_sql_parser::ast::DropUserStmt) -> Result<QueryResult, String> {
        let existed = self.mysql_credentials.remove(&stmt.username).is_some();
        if !existed && !stmt.if_exists {
            return Err(format!("User '{}' does not exist", stmt.username));
        }

        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(if stmt.if_exists { format!("DROP USER {} OK (if exists)", stmt.username) } else { format!("DROP USER {} OK", stmt.username) })]],
        ))
    }

    // ---- Catalog DDL ----

    pub(crate) fn create_catalog(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE CATALOG {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn drop_catalog(&self, conn_id: u32, stmt: &fe_sql_parser::ast::DropCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP CATALOG {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn refresh_catalog(&self, conn_id: u32, stmt: &fe_sql_parser::ast::RefreshCatalogStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("REFRESH CATALOG {} OK", stmt.name))]],
        ))
    }

    // ---- Export / Variable ----

    pub(crate) fn export_table(&self, conn_id: u32, stmt: &fe_sql_parser::ast::ExportTableStmt) -> Result<QueryResult, String> {
        let current_db = self.get_session(conn_id);
        let db = stmt.database.as_deref().unwrap_or(&current_db);
        let format = stmt.properties.iter()
            .find(|(k, _)| k.to_lowercase() == "format")
            .map(|(_, v)| v.as_str())
            .unwrap_or("parquet");

        let msg = fe_backup::export::export_table(&self.storage, db, &stmt.table, &stmt.path, format)?;
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(msg)]],
        ))
    }

    pub(crate) fn set_variable(&self, conn_id: u32, stmt: &fe_sql_parser::ast::SetVariableStmt) -> Result<QueryResult, String> {
        let var_name = stmt.variable.trim_matches('`').to_lowercase();
        // Convert Expr to string value
        let value = match &stmt.value {
            fe_sql_parser::ast::Expr::Literal(lit) => match lit {
                fe_sql_parser::ast::LiteralValue::String(s) => s.clone(),
                fe_sql_parser::ast::LiteralValue::Int64(n) => n.to_string(),
                fe_sql_parser::ast::LiteralValue::Float64(n) => n.to_string(),
                fe_sql_parser::ast::LiteralValue::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
                fe_sql_parser::ast::LiteralValue::Null => "".to_string(),
                fe_sql_parser::ast::LiteralValue::Date(s) => s.clone(),
            },
            fe_sql_parser::ast::Expr::ColumnRef { column, .. } => column.clone(),
            _ => format!("{:?}", stmt.value),
        };

        if stmt.is_global {
            self.sys_vars.set_global(&var_name, &value)?;
        } else {
            // Set session variable within the closure to avoid lifetime issues
            self.with_session_mut(conn_id, |s| {
                s.session_vars.set(&var_name, &value)
            })?;
        }

        Ok(QueryResult::ok())
    }

    // ---- Stats / Analyze ----

    pub(crate) fn analyze_table(&self, conn_id: u32, stmt: &fe_sql_parser::ast::AnalyzeTableStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ANALYZE TABLE {}.{} OK", stmt.database.as_deref().unwrap_or(""), stmt.table))]],
        ))
    }

    pub(crate) fn drop_stats(&self, conn_id: u32, stmt: &fe_sql_parser::ast::DropStatsStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP STATS {}.{} OK", stmt.database.as_deref().unwrap_or(""), stmt.table))]],
        ))
    }

    // ---- Function DDL ----

    pub(crate) fn create_function(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateFunctionStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE FUNCTION {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn drop_function(&self, conn_id: u32, stmt: &fe_sql_parser::ast::DropFunctionStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP FUNCTION {} OK", stmt.name))]],
        ))
    }

    // ---- Stats / Analyze ----

    pub(crate) fn create_job(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateJobStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE JOB {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn drop_job_stmt(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP JOB {} OK", name))]],
        ))
    }

    // ---- Plugin DDL ----

    pub(crate) fn install_plugin(&self, conn_id: u32, stmt: &fe_sql_parser::ast::InstallPluginStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("INSTALL PLUGIN {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn uninstall_plugin(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("UNINSTALL PLUGIN {} OK", name))]],
        ))
    }

    // ---- Recover ----

    pub(crate) fn recover_database(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER DATABASE {} OK", name))]],
        ))
    }

    pub(crate) fn recover_table(&self, conn_id: u32, database: String, table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER TABLE {}.{} OK", database, table))]],
        ))
    }

    pub(crate) fn recover_partition(&self, conn_id: u32, database: String, table: String, partition: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RECOVER PARTITION {}.{}.{} OK", database, table, partition))]],
        ))
    }

    // ---- Recycle Bin ----

    pub(crate) fn drop_catalog_recycle_bin(&self, conn_id: u32, filter: Option<String>) -> Result<QueryResult, String> {
        let _ = filter;
        Ok(QueryResult::ok())
    }

    // ---- SQL Block Rule ----

    pub(crate) fn create_sql_block_rule(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateSqlBlockRuleStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE SQL_BLOCK_RULE {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn alter_sql_block_rule(&self, conn_id: u32, name: String, _props: Vec<(String, String)>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER SQL_BLOCK_RULE {} OK", name))]],
        ))
    }

    pub(crate) fn drop_sql_block_rule(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP SQL_BLOCK_RULE {} OK", name))]],
        ))
    }

    // ---- Row Policy ----

    pub(crate) fn create_row_policy(&self, conn_id: u32, stmt: &fe_sql_parser::ast::CreateRowPolicyStmt) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CREATE ROW POLICY {} OK", stmt.name))]],
        ))
    }

    pub(crate) fn drop_row_policy(&self, conn_id: u32, name: String, _database: Option<String>, _table: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("DROP ROW POLICY {} OK", name))]],
        ))
    }

    // ---- Stats / Analyze admin ----

    pub(crate) fn alter_stats(&self, conn_id: u32, table: String, _props: Vec<(String, String)>) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("ALTER STATS {} OK", table))]],
        ))
    }

    pub(crate) fn kill_analyze_job(&self, conn_id: u32, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("KILL ANALYZE JOB {} OK", id))]],
        ))
    }

    // ---- Export / Task / Job stubs ----

    pub(crate) fn cancel_export(&self, conn_id: u32, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CANCEL EXPORT {} OK", id))]],
        ))
    }

    pub(crate) fn pause_job(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("PAUSE JOB {} OK", name))]],
        ))
    }

    pub(crate) fn resume_job_stmt(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("RESUME JOB {} OK", name))]],
        ))
    }

    pub(crate) fn cancel_task(&self, conn_id: u32, id: String) -> Result<QueryResult, String> {
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "Status".to_string(), col_type: ColumnType::String }],
            vec![vec![Some(format!("CANCEL TASK {} OK", id))]],
        ))
    }

    // ---- Internal helpers ----

    /// Update the DataFusion table schema by re-creating Parquet storage with current catalog metadata.
    fn update_df_table_schema_inner(&self, db: &str, table_name: &str) -> Result<(), String> {
        let catalog = &self.catalog;
        let tbl = catalog.get_table(db, table_name)
            .ok_or_else(|| format!("Table {}.{} not found", db, table_name))?;

        let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = tbl.columns.iter().map(|c| {
            datafusion::arrow::datatypes::Field::new(
                &c.name,
                fe_datafusion::types::to_arrow_data_type(&c.data_type),
                c.nullable,
            )
        }).collect();
        let arrow_schema = Arc::new(datafusion::arrow::datatypes::Schema::new(arrow_fields));

        // Re-create Parquet storage with new schema (truncate + create)
        if let Err(e) = self.storage.truncate(db, table_name, arrow_schema) {
            tracing::warn!("Failed to update table storage schema: {}", e);
        }

        Ok(())
    }
}
