use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::DataType as ADT;
use datafusion::arrow::record_batch::RecordBatch;
use mysql_protocol::QueryResult;
use mysql_protocol::server::{ColumnDef, ColumnType};

use fe_sql_parser::ast::DeleteStmt;

use crate::handler_struct::RorisQueryHandler;
use crate::utils::{build_arrow_array, evaluate_delete_filter_simple, expr_to_string_value, update_column_in_batch};

impl RorisQueryHandler {
    pub(crate) fn insert(&self, stmt: &fe_sql_parser::ast::InsertStmt) -> Result<QueryResult, String> {
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.current_database.read();
                (current_db.clone(), stmt.table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read();
                (current_db.clone(), stmt.table.clone())
            }
        };

        let catalog = &self.catalog;
        let table_meta = catalog.get_table(&database, &table_name)
            .ok_or_else(|| format!("table '{}.{}' not found in catalog", database, table_name))?;

        // Build Arrow schema from table metadata
        let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = table_meta.columns.iter().map(|c| {
            datafusion::arrow::datatypes::Field::new(
                &c.name,
                fe_datafusion::types::to_arrow_data_type(&c.data_type),
                c.nullable,
            )
        }).collect();
        let arrow_schema = Arc::new(datafusion::arrow::datatypes::Schema::new(arrow_fields));

        // Build a map from column name to value index in stmt.values
        let column_value_map: std::collections::HashMap<String, usize> = stmt.columns.iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        let num_cols = table_meta.columns.len();
        let mut arrays: Vec<datafusion::arrow::array::ArrayRef> = Vec::new();

        for col_idx in 0..num_cols {
            let col_meta = &table_meta.columns[col_idx];
            let col_type = &col_meta.data_type;

            let values: Vec<Option<String>> = if let Some(value_idx) = column_value_map.get(&col_meta.name) {
                stmt.values.iter().map(|row| {
                    row.get(*value_idx).and_then(|expr| expr_to_string_value(expr))
                }).collect()
            } else {
                stmt.values.iter().map(|_| None).collect()
            };

            let arr = build_arrow_array(col_type, &values);
            arrays.push(arr);
        }

        let batch = datafusion::arrow::record_batch::RecordBatch::try_new(
            arrow_schema.clone(), arrays,
        ).map_err(|e| format!("Failed to create record batch: {}", e))?;

        // Write to Parquet storage
        self.storage.insert(&database, &table_name, batch)
            .map_err(|e| format!("Insert failed: {}", e))?;

        let affected_rows = stmt.values.len();
        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }

    pub(crate) fn update(&self, stmt: &fe_sql_parser::ast::UpdateStmt) -> Result<QueryResult, String> {
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.current_database.read();
                (current_db.clone(), stmt.table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read();
                (current_db.clone(), stmt.table.clone())
            }
        };

        let set_clauses = stmt.set_clauses.clone();
        let selection = stmt.selection.clone();

        let total_updated = self.storage.update(&database, &table_name, |batch| {
            let mut total_updated = 0usize;
            let update_mask: Vec<bool> = if let Some(ref sel) = selection {
                evaluate_delete_filter_simple(&batch, sel).map_err(|e| fe_storage::StorageError::Other(e))?
            } else {
                vec![true; batch.num_rows()]
            };
            total_updated += update_mask.iter().filter(|&u| *u).count();

            let mut updated_batch = batch;
            for set_clause in &set_clauses {
                let col_idx = updated_batch.schema().index_of(&set_clause.column)
                    .map_err(|e| fe_storage::StorageError::Other(e.to_string()))?;
                let val_str = expr_to_string_value(&set_clause.value)
                    .ok_or_else(|| fe_storage::StorageError::Other(format!("Unsupported assignment value: {:?}", set_clause.value)))?;
                update_column_in_batch(&mut updated_batch, col_idx, &val_str, &update_mask)
                    .map_err(|e| fe_storage::StorageError::Other(e))?;
            }
            Ok((updated_batch, total_updated))
        }).map_err(|e| format!("Update failed: {}", e))?;

        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(total_updated.to_string())]],
        ))
    }

    pub(crate) fn delete(&self, stmt: &DeleteStmt) -> Result<QueryResult, String> {
        let target_tables = if stmt.tables.is_empty() {
            if let Some(ref from) = stmt.from {
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
                let current_db = self.current_database.read();
                (current_db.clone(), primary_table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.current_database.read();
                (current_db.clone(), primary_table.clone())
            }
        };

        let selection = stmt.selection.clone();

        let total_deleted = self.storage.delete(&database, &table_name, |batch| {
            if let Some(ref sel) = selection {
                let keep_mask = evaluate_delete_filter_simple(&batch, sel).map_err(|e| fe_storage::StorageError::Other(e))?;
                let deleted_count = keep_mask.iter().filter(|&&k| !k).count();
                let filtered = datafusion::arrow::compute::filter_record_batch(
                    &batch,
                    &datafusion::arrow::array::BooleanArray::from(keep_mask),
                ).map_err(|e| fe_storage::StorageError::Arrow(e.to_string()))?;
                Ok((filtered, deleted_count))
            } else {
                let count = batch.num_rows();
                let schema = batch.schema();
                let empty = datafusion::arrow::record_batch::RecordBatch::new_empty(schema);
                Ok((empty, count))
            }
        }).map_err(|e| format!("Delete failed: {}", e))?;

        Ok(QueryResult::with_rows(
            vec![ColumnDef { name: "affected_rows".to_string(), col_type: ColumnType::Int }],
            vec![vec![Some(total_deleted.to_string())]],
        ))
    }

    pub(crate) fn start_transaction(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        if tx.in_transaction {
            // Nested BEGIN is a no-op in non-savepoint mode (matches MySQL behavior)
            return Ok(QueryResult::ok());
        }
        tx.begin();
        Ok(QueryResult::ok())
    }

    pub(crate) fn commit_tx(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        if !tx.in_transaction {
            return Err("No transaction to commit".to_string());
        }
        tx.in_transaction = false;
        Ok(QueryResult::ok())
    }

    pub(crate) fn rollback_tx(&self) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        if !tx.in_transaction {
            return Err("No transaction to rollback".to_string());
        }
        tx.rollback();
        tx.in_transaction = false;
        Ok(QueryResult::ok())
    }

    pub(crate) fn savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        tx.savepoint(name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn rollback_to_savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        tx.rollback_to_savepoint(&name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn release_savepoint(&self, name: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        tx.release_savepoint(&name).map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn set_transaction_isolation(&self, level: String) -> Result<QueryResult, String> {
        let mut tx = self.transaction.write();
        tx.set_isolation_level(level.clone());
        tracing::info!("Setting transaction isolation level to: {}", level);
        Ok(QueryResult::ok())
    }
}
