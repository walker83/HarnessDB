use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::compute::concat_batches;
use datafusion::arrow::datatypes::{DataType as ADT, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use datafusion::prelude::{SessionConfig, SessionContext};
use fe_storage::ParquetCatalogProvider;
use mysql_protocol::QueryResult;
use mysql_protocol::server::{ColumnDef, ColumnType};

use fe_sql_parser::ast::{self, DeleteStmt};

use crate::handler_struct::RorisQueryHandler;
use crate::utils::{
    build_arrow_array_from_exprs, expr_to_string_value, merge_columns, update_column_in_batch,
};

impl RorisQueryHandler {
    pub(crate) fn insert(
        &self,
        conn_id: u32,
        stmt: &ast::InsertStmt,
    ) -> Result<QueryResult, String> {
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.get_session(conn_id);
                (current_db.clone(), stmt.table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.get_session(conn_id);
                (current_db.clone(), stmt.table.clone())
            }
        };

        let catalog = &self.catalog;
        let table_meta = catalog
            .get_table(&database, &table_name)
            .ok_or_else(|| format!("table '{}.{}' not found in catalog", database, table_name))?;

        // Build Arrow schema from table metadata
        let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = table_meta
            .columns
            .iter()
            .map(|c| {
                datafusion::arrow::datatypes::Field::new(
                    &c.name,
                    fe_datafusion::types::to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        let arrow_schema = Arc::new(datafusion::arrow::datatypes::Schema::new(arrow_fields));

        // ---------- INSERT INTO ... SELECT path ----------
        if let Some(query) = &stmt.query {
            return self.handle_insert_select(
                query,
                &stmt.columns,
                &database,
                &table_name,
                &table_meta,
                arrow_schema,
            );
        }

        // ---------- INSERT INTO ... VALUES path ----------
        // Map column position to value index in stmt.values.
        // If stmt.columns is empty (INSERT INTO t VALUES ...), values map 1:1 by position.
        // Otherwise, build a name->index map from the explicit column list.
        let positional = stmt.columns.is_empty();
        let column_value_map: std::collections::HashMap<String, usize> = if !positional {
            stmt.columns
                .iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), i))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

        let num_cols = table_meta.columns.len();
        let mut arrays: Vec<datafusion::arrow::array::ArrayRef> = Vec::new();

        for col_idx in 0..num_cols {
            let col_meta = &table_meta.columns[col_idx];
            let arrow_type = fe_datafusion::types::to_arrow_data_type(&col_meta.data_type);

            // Handle columns not in explicit column list — fill with nulls
            if !positional && !column_value_map.contains_key(&col_meta.name) {
                let arr = new_null_array(&arrow_type, stmt.values.len());
                arrays.push(arr);
                continue;
            }

            let exprs: Vec<&ast::Expr> = if positional {
                stmt.values
                    .iter()
                    .filter_map(|row| row.get(col_idx))
                    .collect()
            } else {
                let value_idx = column_value_map[&col_meta.name];
                stmt.values
                    .iter()
                    .filter_map(|row| row.get(value_idx))
                    .collect()
            };

            let arr = build_arrow_array_from_exprs(&arrow_type, &exprs);
            arrays.push(arr);
        }

        let batch =
            datafusion::arrow::record_batch::RecordBatch::try_new(arrow_schema.clone(), arrays)
                .map_err(|e| format!("Failed to create record batch: {}", e))?;

        // Write to Parquet storage
        self.storage
            .insert(&database, &table_name, batch)
            .map_err(|e| format!("Insert failed: {}", e))?;

        let affected_rows = stmt.values.len();
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "affected_rows".to_string(),
                col_type: ColumnType::Int,
            }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }

    /// Execute INSERT INTO ... SELECT via DataFusion.
    fn handle_insert_select(
        &self,
        query: &ast::QueryStmt,
        insert_columns: &[String],
        database: &str,
        table_name: &str,
        table_meta: &fe_catalog::table::Table,
        target_schema: Arc<datafusion::arrow::datatypes::Schema>,
    ) -> Result<QueryResult, String> {
        // Reconstruct SELECT SQL from the parsed QueryStmt AST
        let select_sql = query_stmt_to_sql(query);

        // Execute via DataFusion with concurrency-limited thread
        // Use a per-query context with the correct default schema
        let current_db = database.to_string();
        let result = self.run_datafusion({
            let catalog = self.catalog.clone();
            let storage = self.storage.clone();
            let rt = self.tokio_runtime.clone();
            move || {
                rt.block_on(async {
                    let df_catalog = Arc::new(ParquetCatalogProvider::new(catalog, storage));
                    let df_config = SessionConfig::new()
                        .with_default_catalog_and_schema("roris", &current_db)
                        .with_create_default_catalog_and_schema(false)
                        .with_information_schema(false); // Use custom information_schema from ParquetCatalogProvider
                    let ctx = SessionContext::new_with_config(df_config);
                    ctx.register_catalog("roris", df_catalog);
                    let df = ctx.sql(&select_sql).await.map_err(|e| e.to_string())?;
                    let batches = df.collect().await.map_err(|e| e.to_string())?;
                    Ok::<_, String>(batches)
                })
            }
        });

        let select_batches = match result {
            Ok(batches) => batches,
            Err(e) => return Err(format!("INSERT ... SELECT query failed: {}", e)),
        };

        // Concatenate all batches into one
        let select_batch = if select_batches.is_empty() {
            RecordBatch::new_empty(target_schema.clone())
        } else {
            concat_batches(&target_schema, &select_batches)
                .map_err(|e| format!("Failed to concat SELECT batches: {}", e))?
        };

        let num_select_cols = select_batch.num_columns();
        let num_target_cols = table_meta.columns.len();

        // Build mapping: for each target column index, which SELECT output column provides data.
        let mut target_to_select: Vec<Option<usize>> = vec![None; num_target_cols];

        if !insert_columns.is_empty() {
            // Explicit column list: insert_columns[i] is the target column name for SELECT output i
            for (select_idx, col_name) in insert_columns.iter().enumerate() {
                if let Some(target_idx) =
                    table_meta.columns.iter().position(|c| c.name == *col_name)
                {
                    target_to_select[target_idx] = Some(select_idx);
                }
            }
        } else {
            // Positional: SELECT output column i maps to target column i
            for i in 0..num_select_cols.min(num_target_cols) {
                target_to_select[i] = Some(i);
            }
        }

        // Build target arrays with potential column reordering
        let mut target_arrays: Vec<ArrayRef> = Vec::with_capacity(num_target_cols);
        for target_idx in 0..num_target_cols {
            let arrow_type = target_schema.field(target_idx).data_type();
            match target_to_select[target_idx] {
                Some(sel_idx) if sel_idx < select_batch.num_columns() => {
                    let src_col = select_batch.column(sel_idx);
                    // Cast if the types don't match exactly
                    if src_col.data_type() == arrow_type {
                        target_arrays.push(src_col.clone());
                    } else {
                        // Attempt a cast via Arrow compute
                        let casted =
                            datafusion::arrow::compute::kernels::cast::cast(src_col, arrow_type)
                                .map_err(|e| {
                                    format!("Type cast failed for column {}: {}", target_idx, e)
                                })?;
                        target_arrays.push(casted);
                    }
                }
                _ => {
                    // Column not covered by SELECT — fill with nulls
                    target_arrays.push(new_null_array(arrow_type, select_batch.num_rows()));
                }
            }
        }

        let batch = RecordBatch::try_new(target_schema.clone(), target_arrays)
            .map_err(|e| format!("Failed to create target record batch: {}", e))?;

        self.storage
            .insert(database, table_name, batch)
            .map_err(|e| format!("Insert failed: {}", e))?;

        let affected_rows = select_batch.num_rows();
        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "affected_rows".to_string(),
                col_type: ColumnType::Int,
            }],
            vec![vec![Some(affected_rows.to_string())]],
        ))
    }
}

// ============================================================
// SQL reconstruction helpers for INSERT INTO ... SELECT
// ============================================================

/// Reconstruct a SELECT SQL string from a parsed `QueryStmt` AST.
/// Used by INSERT INTO ... SELECT to execute the SELECT via DataFusion.
fn query_stmt_to_sql(query: &ast::QueryStmt) -> String {
    let mut sql = String::new();

    // WITH clause
    if let Some(ref cte) = query.with {
        sql.push_str("WITH ");
        sql.push_str(&cte.name);
        if !cte.columns.is_empty() {
            sql.push('(');
            sql.push_str(&cte.columns.join(", "));
            sql.push(')');
        }
        sql.push_str(" AS (");
        sql.push_str(&query_stmt_to_sql(&cte.query));
        sql.push_str(") ");
    }

    // SELECT items
    sql.push_str("SELECT ");
    let items: Vec<String> = query
        .select_list
        .iter()
        .map(|item| {
            let mut s = expr_to_sql(&item.expr);
            if let Some(ref alias) = item.alias {
                s.push_str(" AS ");
                s.push_str(alias);
            }
            s
        })
        .collect();
    sql.push_str(&items.join(", "));

    // FROM
    if let Some(ref from) = query.from {
        sql.push_str(" FROM ");
        sql.push_str(&table_ref_to_sql(from));
    }

    // WHERE
    if let Some(ref where_expr) = query.r#where {
        sql.push_str(" WHERE ");
        sql.push_str(&expr_to_sql(where_expr));
    }

    // GROUP BY
    if !query.group_by.is_empty() {
        sql.push_str(" GROUP BY ");
        let group_items: Vec<String> = query.group_by.iter().map(expr_to_sql).collect();
        sql.push_str(&group_items.join(", "));
    }

    // HAVING
    if let Some(ref having) = query.having {
        sql.push_str(" HAVING ");
        sql.push_str(&expr_to_sql(having));
    }

    // ORDER BY
    if !query.order_by.is_empty() {
        sql.push_str(" ORDER BY ");
        let order_items: Vec<String> = query
            .order_by
            .iter()
            .map(|item| {
                let mut s = expr_to_sql(&item.expr);
                if !item.ascending {
                    s.push_str(" DESC");
                }
                if item.nulls_first {
                    s.push_str(" NULLS FIRST");
                }
                s
            })
            .collect();
        sql.push_str(&order_items.join(", "));
    }

    // LIMIT
    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }

    // OFFSET
    if let Some(offset) = query.offset {
        sql.push_str(&format!(" OFFSET {}", offset));
    }

    // UNION / EXCEPT / INTERSECT
    if let Some(ref set_op) = query.set_op {
        let op_str = match set_op.op {
            ast::UnionOperator::Union => "UNION",
            ast::UnionOperator::Except => "EXCEPT",
            ast::UnionOperator::Intersect => "INTERSECT",
        };
        sql.push_str(&format!(" {} ", op_str));
        if set_op.all {
            sql.push_str("ALL ");
        }
        sql.push_str(&query_stmt_to_sql(&set_op.right));
    }

    sql
}

/// Convert a parsed `Expr` back to a SQL fragment.
fn expr_to_sql(expr: &ast::Expr) -> String {
    use ast::{Expr, UnaryOp};

    match expr {
        Expr::Literal(lit) => literal_to_sql(lit),
        Expr::ColumnRef { table, column } => {
            if let Some(t) = table {
                format!("{}.{}", t, column)
            } else {
                column.clone()
            }
        }
        Expr::BinaryOp { left, op, right } => {
            format!(
                "({} {} {})",
                expr_to_sql(left),
                binary_op_to_sql(op),
                expr_to_sql(right)
            )
        }
        Expr::UnaryOp { op, expr: e } => match op {
            UnaryOp::Not => format!("NOT {}", expr_to_sql(e)),
            UnaryOp::Negate => format!("-{}", expr_to_sql(e)),
        },
        Expr::Wildcard => "*".to_string(),
        Expr::FunctionCall {
            name,
            args,
            distinct,
        } => {
            let args_sql: Vec<String> = args.iter().map(expr_to_sql).collect();
            if *distinct {
                format!("{}(DISTINCT {})", name, args_sql.join(", "))
            } else {
                format!("{}({})", name, args_sql.join(", "))
            }
        }
        Expr::IsNull { expr: e, negated } => {
            if *negated {
                format!("{} IS NOT NULL", expr_to_sql(e))
            } else {
                format!("{} IS NULL", expr_to_sql(e))
            }
        }
        Expr::InList {
            expr: e,
            list,
            negated,
        } => {
            let list_sql: Vec<String> = list.iter().map(expr_to_sql).collect();
            if *negated {
                format!("{} NOT IN ({})", expr_to_sql(e), list_sql.join(", "))
            } else {
                format!("{} IN ({})", expr_to_sql(e), list_sql.join(", "))
            }
        }
        Expr::Between {
            expr: e,
            low,
            high,
            negated,
        } => {
            if *negated {
                format!(
                    "{} NOT BETWEEN {} AND {}",
                    expr_to_sql(e),
                    expr_to_sql(low),
                    expr_to_sql(high)
                )
            } else {
                format!(
                    "{} BETWEEN {} AND {}",
                    expr_to_sql(e),
                    expr_to_sql(low),
                    expr_to_sql(high)
                )
            }
        }
        Expr::CaseWhen { cases, else_expr } => {
            let mut s = "CASE".to_string();
            for wt in cases {
                s.push_str(&format!(
                    " WHEN {} THEN {}",
                    expr_to_sql(&wt.when),
                    expr_to_sql(&wt.then)
                ));
            }
            if let Some(else_e) = else_expr {
                s.push_str(&format!(" ELSE {}", expr_to_sql(else_e)));
            }
            s.push_str(" END");
            s
        }
        Expr::Like {
            expr: e,
            pattern,
            negated,
        } => {
            if *negated {
                format!("{} NOT LIKE {}", expr_to_sql(e), expr_to_sql(pattern))
            } else {
                format!("{} LIKE {}", expr_to_sql(e), expr_to_sql(pattern))
            }
        }
        Expr::Cast {
            expr: e,
            target_type,
        } => {
            format!("CAST({} AS {})", expr_to_sql(e), target_type)
        }
        Expr::Subquery(q) => {
            format!("({})", query_stmt_to_sql(q))
        }
        Expr::Exists(q) => {
            format!("EXISTS ({})", query_stmt_to_sql(q))
        }
        Expr::InSubquery {
            expr: e,
            query: q,
            negated,
        } => {
            if *negated {
                format!("{} NOT IN ({})", expr_to_sql(e), query_stmt_to_sql(q))
            } else {
                format!("{} IN ({})", expr_to_sql(e), query_stmt_to_sql(q))
            }
        }
        Expr::Default => "DEFAULT".to_string(),
    }
}

/// Convert a parsed `LiteralValue` back to a SQL fragment.
fn literal_to_sql(lit: &ast::LiteralValue) -> String {
    match lit {
        ast::LiteralValue::Null => "NULL".to_string(),
        ast::LiteralValue::Boolean(b) => b.to_string(),
        ast::LiteralValue::Int64(i) => i.to_string(),
        ast::LiteralValue::Float64(f) => f.to_string(),
        ast::LiteralValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        ast::LiteralValue::Date(d) => format!("'{}'", d),
    }
}

/// Convert a parsed `BinaryOp` back to a SQL operator string.
fn binary_op_to_sql(op: &ast::BinaryOp) -> &'static str {
    match op {
        ast::BinaryOp::Eq => "=",
        ast::BinaryOp::NotEq => "<>",
        ast::BinaryOp::Lt => "<",
        ast::BinaryOp::LtEq => "<=",
        ast::BinaryOp::Gt => ">",
        ast::BinaryOp::GtEq => ">=",
        ast::BinaryOp::And => "AND",
        ast::BinaryOp::Or => "OR",
        ast::BinaryOp::Plus => "+",
        ast::BinaryOp::Minus => "-",
        ast::BinaryOp::Multiply => "*",
        ast::BinaryOp::Divide => "/",
        ast::BinaryOp::Modulo => "%",
        ast::BinaryOp::Like => "LIKE",
        ast::BinaryOp::NotLike => "NOT LIKE",
        ast::BinaryOp::In => "IN",
        ast::BinaryOp::NotIn => "NOT IN",
        ast::BinaryOp::BitwiseAnd => "&",
        ast::BinaryOp::BitwiseOr => "|",
        ast::BinaryOp::BitwiseXor => "^",
    }
}

/// Convert a parsed `TableRef` back to a SQL fragment.
fn table_ref_to_sql(t: &ast::TableRef) -> String {
    match t {
        ast::TableRef::Table { name, alias } => {
            if let Some(a) = alias {
                format!("{} AS {}", name, a)
            } else {
                name.clone()
            }
        }
        ast::TableRef::Join {
            left,
            right,
            r#type,
            condition,
        } => {
            let join_type = match r#type {
                ast::JoinType::Inner => "INNER JOIN",
                ast::JoinType::LeftOuter => "LEFT JOIN",
                ast::JoinType::RightOuter => "RIGHT JOIN",
                ast::JoinType::FullOuter => "FULL JOIN",
                ast::JoinType::Cross => "CROSS JOIN",
            };
            let mut s = format!(
                "{} {} {}",
                table_ref_to_sql(left),
                join_type,
                table_ref_to_sql(right)
            );
            if let Some(cond) = condition {
                s.push_str(&format!(" ON {}", expr_to_sql(cond)));
            }
            s
        }
        ast::TableRef::Subquery { query, alias } => {
            format!("({}) AS {}", query_stmt_to_sql(query), alias)
        }
    }
}

impl RorisQueryHandler {
    /// Evaluate a WHERE expression on a RecordBatch using DataFusion.
    ///
    /// Returns a boolean mask where `true` means the row matches the condition.
    /// Unlike the hand-written `evaluate_where_filter` (which only handles `column op literal`),
    /// this function supports BETWEEN, IN, LIKE, IS NULL, function calls, and arbitrary expressions.
    ///
    /// The approach:
    /// 1. Convert the WHERE AST back to SQL using `expr_to_sql`
    /// 2. Wrap the RecordBatch in a MemTable with a row index column
    /// 3. Execute `SELECT __row_index FROM __tmp WHERE <condition>` via DataFusion
    /// 4. Build the boolean mask from the matched row indices
    pub(crate) fn evaluate_where_with_datafusion(
        &self,
        batch: &RecordBatch,
        where_expr: &ast::Expr,
    ) -> Result<Vec<bool>, String> {
        let schema = batch.schema();
        let num_rows = batch.num_rows();

        // Add a row index column so we can identify which rows match the WHERE condition
        let row_indices: Int64Array = (0i64..num_rows as i64).collect();
        let indexed_fields: Vec<Field> =
            schema.fields().iter().map(|f| f.as_ref().clone()).collect();
        let mut all_fields = indexed_fields;
        all_fields.push(Field::new("__row_index", ADT::Int64, false));
        let indexed_schema = Arc::new(Schema::new(all_fields));

        let mut indexed_columns: Vec<ArrayRef> = batch.columns().to_vec();
        indexed_columns.push(Arc::new(row_indices));
        let indexed_batch = RecordBatch::try_new(indexed_schema.clone(), indexed_columns)
            .map_err(|e| format!("Failed to create indexed batch: {}", e))?;

        // Build the WHERE clause SQL from the AST
        let where_sql = expr_to_sql(where_expr);

        // Clone the session context and register the batch as a MemTable
        let mem_table = MemTable::try_new(indexed_schema, vec![vec![indexed_batch]])
            .map_err(|e| format!("Failed to create MemTable: {}", e))?;

        // Use a fresh SessionContext to avoid schema provider conflicts
        let ctx = SessionContext::new();
        ctx.register_table("__tmp", Arc::new(mem_table))
            .map_err(|e| format!("Failed to register table: {}", e))?;

        let sql = format!("SELECT __row_index FROM __tmp WHERE {}", where_sql);

        // Execute with concurrency-limited thread
        let rt = self.tokio_runtime.clone();
        let matching_batches = self.run_datafusion(move || {
            rt.block_on(async {
                let df = ctx.sql(&sql).await.map_err(|e| e.to_string())?;
                let batches = df.collect().await.map_err(|e| e.to_string())?;
                Ok::<_, String>(batches)
            })
        })?;

        // Build the boolean mask from the matched row indices
        let mut matching = vec![false; num_rows];
        for batch in &matching_batches {
            let idx_col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| "Failed to downcast row index column".to_string())?;
            for i in 0..idx_col.len() {
                matching[idx_col.value(i) as usize] = true;
            }
        }

        Ok(matching)
    }

    /// Evaluate a SET expression for every row in the batch using DataFusion.
    /// Returns an ArrayRef containing the computed value for each row.
    ///
    /// The approach:
    /// 1. Convert the SET expression AST back to SQL using `expr_to_sql`
    /// 2. Register the RecordBatch as a MemTable
    /// 3. Execute `SELECT <expr> AS __new_val FROM __tmp` via DataFusion
    /// 4. Collect and return the result column
    pub(crate) fn evaluate_set_expr_with_datafusion(
        &self,
        batch: &RecordBatch,
        set_expr: &ast::Expr,
    ) -> Result<ArrayRef, String> {
        let schema = batch.schema();
        let num_rows = batch.num_rows();

        // Build the SELECT expression SQL from the AST
        let expr_sql = expr_to_sql(set_expr);

        // Clone the session context and register the batch as a MemTable
        let mem_table = MemTable::try_new(schema.clone(), vec![vec![batch.clone()]])
            .map_err(|e| format!("Failed to create MemTable: {}", e))?;

        // Use a fresh SessionContext to avoid schema provider conflicts
        let ctx = SessionContext::new();
        ctx.register_table("__tmp", Arc::new(mem_table))
            .map_err(|e| format!("Failed to register table: {}", e))?;

        let sql = format!("SELECT {} AS __new_val FROM __tmp", expr_sql);

        // Execute with concurrency-limited thread
        let rt = self.tokio_runtime.clone();
        let result_batches = self.run_datafusion(move || {
            rt.block_on(async {
                let df = ctx.sql(&sql).await.map_err(|e| e.to_string())?;
                let batches = df.collect().await.map_err(|e| e.to_string())?;
                Ok::<_, String>(batches)
            })
        })?;

        if result_batches.is_empty() || num_rows == 0 {
            return Err("SET expression evaluation returned no batches".to_string());
        }

        // Concatenate all result batches into one column
        let mut arrays: Vec<ArrayRef> = Vec::new();
        for b in &result_batches {
            arrays.push(b.column(0).clone());
        }

        let combined = if arrays.len() == 1 {
            arrays.into_iter().next().unwrap()
        } else {
            datafusion::arrow::compute::concat(
                &arrays.iter().map(|a| a.as_ref()).collect::<Vec<_>>(),
            )
            .map_err(|e| format!("Failed to concat result arrays: {}", e))?
        };

        Ok(combined)
    }
}

impl RorisQueryHandler {
    pub(crate) fn update(
        &self,
        conn_id: u32,
        stmt: &ast::UpdateStmt,
    ) -> Result<QueryResult, String> {
        let parts: Vec<&str> = stmt.table.split('.').collect();
        let (database, table_name) = match parts.len() {
            1 => {
                let current_db = self.get_session(conn_id);
                (current_db.clone(), stmt.table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.get_session(conn_id);
                (current_db.clone(), stmt.table.clone())
            }
        };

        let set_clauses = stmt.set_clauses.clone();
        let selection = stmt.selection.clone();

        let total_updated = self
            .storage
            .update(&database, &table_name, |batch| {
                let mut total_updated = 0usize;
                let update_mask: Vec<bool> = if let Some(ref sel) = selection {
                    self.evaluate_where_with_datafusion(&batch, sel)
                        .map_err(|e| fe_storage::StorageError::Other(e))?
                } else {
                    vec![true; batch.num_rows()]
                };
                total_updated += update_mask.iter().filter(|&u| *u).count();

                let mut updated_batch = batch;
                for set_clause in &set_clauses {
                    let col_idx = updated_batch
                        .schema()
                        .index_of(&set_clause.column)
                        .map_err(|e| fe_storage::StorageError::Other(e.to_string()))?;

                    // Try fast path: literal value
                    if let Some(val_str) = expr_to_string_value(&set_clause.value) {
                        update_column_in_batch(&mut updated_batch, col_idx, &val_str, &update_mask)
                            .map_err(|e| fe_storage::StorageError::Other(e))?;
                    } else {
                        // Expression path: evaluate per-row via DataFusion
                        let new_values = self
                            .evaluate_set_expr_with_datafusion(&updated_batch, &set_clause.value)
                            .map_err(|e| {
                                fe_storage::StorageError::Other(format!(
                                    "Failed to evaluate SET expression: {}",
                                    e
                                ))
                            })?;

                        // Replace column values where update_mask is true
                        let old_col = updated_batch.column(col_idx);
                        let merged = merge_columns(old_col, &new_values, &update_mask)
                            .map_err(|e| fe_storage::StorageError::Other(e))?;

                        let mut new_columns: Vec<ArrayRef> = updated_batch.columns().to_vec();
                        new_columns[col_idx] = merged;
                        updated_batch = RecordBatch::try_new(updated_batch.schema(), new_columns)
                            .map_err(|e| {
                            fe_storage::StorageError::Other(format!(
                                "Failed to create updated batch: {}",
                                e
                            ))
                        })?;
                    }
                }
                Ok((updated_batch, total_updated))
            })
            .map_err(|e| format!("Update failed: {}", e))?;

        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "affected_rows".to_string(),
                col_type: ColumnType::Int,
            }],
            vec![vec![Some(total_updated.to_string())]],
        ))
    }

    pub(crate) fn delete(&self, conn_id: u32, stmt: &DeleteStmt) -> Result<QueryResult, String> {
        let target_tables = if stmt.tables.is_empty() {
            if let Some(ref from) = stmt.from {
                fn get_base_table_name(t: &fe_sql_parser::ast::TableRef) -> String {
                    match t {
                        fe_sql_parser::ast::TableRef::Table { name, .. } => name.clone(),
                        fe_sql_parser::ast::TableRef::Join { left, .. } => {
                            get_base_table_name(left)
                        }
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
                let current_db = self.get_session(conn_id);
                (current_db.clone(), primary_table.clone())
            }
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                let current_db = self.get_session(conn_id);
                (current_db.clone(), primary_table.clone())
            }
        };

        let selection = stmt.selection.clone();
        let order_by = stmt.order_by.clone();
        let delete_limit = stmt.limit;

        let total_deleted = self
            .storage
            .delete(&database, &table_name, |batch| {
                if let Some(ref sel) = selection {
                    let mut match_mask = self
                        .evaluate_where_with_datafusion(&batch, sel)
                        .map_err(|e| fe_storage::StorageError::Other(e))?;

                    // Apply ORDER BY + LIMIT to the match_mask
                    if !order_by.is_empty() || delete_limit.is_some() {
                        // Collect matching row indices
                        let matching_indices: Vec<usize> = match_mask
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| **m)
                            .map(|(i, _)| i)
                            .collect();

                        if !matching_indices.is_empty() {
                            let mut sorted_indices = matching_indices;

                            // If ORDER BY is specified, sort the indices using Arrow sort
                            if !order_by.is_empty() {
                                use datafusion::arrow::compute::kernels::sort::{
                                    SortColumn, SortOptions,
                                };

                                let sort_columns: Vec<SortColumn> = order_by
                                    .iter()
                                    .map(|item| {
                                        let col_name = match &item.expr {
                                            ast::Expr::ColumnRef { column, .. } => column.clone(),
                                            _ => String::new(),
                                        };
                                        let col_idx =
                                            batch.schema().index_of(&col_name).unwrap_or(0);
                                        SortColumn {
                                            values: batch.column(col_idx).clone(),
                                            options: Some(SortOptions {
                                                descending: !item.ascending,
                                                nulls_first: item.nulls_first,
                                            }),
                                        }
                                    })
                                    .collect();

                                // Get sort order for all rows using lexsort
                                let sort_indices = datafusion::arrow::compute::lexsort_to_indices(
                                    &sort_columns,
                                    None,
                                )
                                .map_err(|e| fe_storage::StorageError::Arrow(e.to_string()))?;

                                // Filter to only matching rows, preserving sort order
                                sorted_indices = (0..sort_indices.len())
                                    .map(|i| sort_indices.value(i) as usize)
                                    .filter(|&row_idx| match_mask[row_idx])
                                    .collect();
                            }

                            // If LIMIT is specified, truncate to first N
                            if let Some(limit_n) = delete_limit {
                                sorted_indices.truncate(limit_n);
                            }

                            // Rebuild match_mask: only the sorted+limited indices are "to delete"
                            match_mask = vec![false; match_mask.len()];
                            for &idx in &sorted_indices {
                                match_mask[idx] = true;
                            }
                        }
                    }

                    let deleted_count = match_mask.iter().filter(|&&m| m).count();
                    // keep rows that do NOT match
                    let keep_mask: Vec<bool> = match_mask.iter().map(|&m| !m).collect();
                    let filtered = datafusion::arrow::compute::filter_record_batch(
                        &batch,
                        &datafusion::arrow::array::BooleanArray::from(keep_mask),
                    )
                    .map_err(|e| fe_storage::StorageError::Arrow(e.to_string()))?;
                    Ok((filtered, deleted_count))
                } else {
                    let count = batch.num_rows();
                    let schema = batch.schema();
                    let empty = datafusion::arrow::record_batch::RecordBatch::new_empty(schema);
                    Ok((empty, count))
                }
            })
            .map_err(|e| format!("Delete failed: {}", e))?;

        Ok(QueryResult::with_rows(
            vec![ColumnDef {
                name: "affected_rows".to_string(),
                col_type: ColumnType::Int,
            }],
            vec![vec![Some(total_deleted.to_string())]],
        ))
    }

    pub(crate) fn start_transaction(&self, conn_id: u32) -> Result<QueryResult, String> {
        if self.in_transaction(conn_id) {
            // Nested BEGIN is a no-op in non-savepoint mode (matches MySQL behavior)
            return Ok(QueryResult::ok());
        }
        self.begin_transaction(conn_id);
        Ok(QueryResult::ok())
    }

    pub(crate) fn commit_tx(&self, conn_id: u32) -> Result<QueryResult, String> {
        if !self.in_transaction(conn_id) {
            return Err("No transaction to commit".to_string());
        }
        self.commit_transaction(conn_id);
        Ok(QueryResult::ok())
    }

    pub(crate) fn rollback_tx(&self, conn_id: u32) -> Result<QueryResult, String> {
        if !self.in_transaction(conn_id) {
            return Err("No transaction to rollback".to_string());
        }
        self.rollback_transaction(conn_id);
        Ok(QueryResult::ok())
    }

    pub(crate) fn savepoint_cmd(&self, conn_id: u32, name: String) -> Result<QueryResult, String> {
        self.with_session_mut(conn_id, |s| s.transaction.savepoint(name))
            .map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn rollback_to_savepoint_cmd(
        &self,
        conn_id: u32,
        name: String,
    ) -> Result<QueryResult, String> {
        self.with_session_mut(conn_id, |s| s.transaction.rollback_to_savepoint(&name))
            .map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn release_savepoint_cmd(
        &self,
        conn_id: u32,
        name: String,
    ) -> Result<QueryResult, String> {
        self.with_session_mut(conn_id, |s| s.transaction.release_savepoint(&name))
            .map_err(|e| e)?;
        Ok(QueryResult::ok())
    }

    pub(crate) fn set_transaction_isolation(
        &self,
        conn_id: u32,
        level: String,
    ) -> Result<QueryResult, String> {
        self.with_session_mut(conn_id, |s| {
            s.transaction.set_isolation_level(level.clone())
        });
        tracing::info!("Setting transaction isolation level to: {}", level);
        Ok(QueryResult::ok())
    }
}
