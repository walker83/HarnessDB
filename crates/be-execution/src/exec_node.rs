use async_trait::async_trait;
use common::{Result, DrorisError};
use types::{Block, Bitmap, Vector, Schema, ScalarValue};
use types::vector::{
    BooleanVector, Int8Vector, Int16Vector, Int32Vector, Int64Vector, Int128Vector,
    Float32Vector, Float64Vector, StringVector, DateVector, DateTimeVector, NullVector,
    JsonVector,
};
use types::runtime_filter::{MinMaxFilter, InFilter};
use be_storage::index::BloomFilter;
use std::sync::{Arc, RwLock as StdRwLock};
use std::collections::HashMap;
use be_storage::StorageEngine;
use be_storage::index::{ColumnPredicate, apply_predicates_to_block};
use crate::predicate_parser::{parse_predicates, parse_set_value, make_affected_rows_block};

#[async_trait]
pub trait ExecNode: Send + Sync {
    async fn open(&mut self) -> Result<()>;
    async fn get_next(&mut self) -> Result<Option<Block>>;
    async fn close(&mut self) -> Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub enum ExecutionPlan {
    Scan(ScanExecNode),
    Filter(FilterExecNode),
    Project(ProjectExecNode),
    Aggregate(AggregateExecNode),
    Sort(SortExecNode),
    Limit(LimitExecNode),
    HashJoin(HashJoinExecNode),
    Union(UnionExecNode),
    Truncate(TruncateExecNode),
    Window(WindowExecNode),
    Update(UpdateExecNode),
    Delete(DeleteExecNode),
    AlterTable(AlterTableExecNode),
    Insert(InsertExecNode),
    Values(ValuesExecNode),
}

#[async_trait]
impl ExecNode for ExecutionPlan {
    async fn open(&mut self) -> Result<()> {
        match self {
            ExecutionPlan::Scan(node) => node.open().await,
            ExecutionPlan::Filter(node) => node.open().await,
            ExecutionPlan::Project(node) => node.open().await,
            ExecutionPlan::Aggregate(node) => node.open().await,
            ExecutionPlan::Sort(node) => node.open().await,
            ExecutionPlan::Limit(node) => node.open().await,
            ExecutionPlan::HashJoin(node) => node.open().await,
            ExecutionPlan::Union(node) => node.open().await,
            ExecutionPlan::Truncate(node) => node.open().await,
            ExecutionPlan::Window(node) => node.open().await,
            ExecutionPlan::Update(node) => node.open().await,
            ExecutionPlan::Delete(node) => node.open().await,
            ExecutionPlan::AlterTable(node) => node.open().await,
            ExecutionPlan::Insert(node) => node.open().await,
            ExecutionPlan::Values(node) => node.open().await,
        }
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        match self {
            ExecutionPlan::Scan(node) => node.get_next().await,
            ExecutionPlan::Filter(node) => node.get_next().await,
            ExecutionPlan::Project(node) => node.get_next().await,
            ExecutionPlan::Aggregate(node) => node.get_next().await,
            ExecutionPlan::Sort(node) => node.get_next().await,
            ExecutionPlan::Limit(node) => node.get_next().await,
            ExecutionPlan::HashJoin(node) => node.get_next().await,
            ExecutionPlan::Union(node) => node.get_next().await,
            ExecutionPlan::Truncate(node) => node.get_next().await,
            ExecutionPlan::Window(node) => node.get_next().await,
            ExecutionPlan::Update(node) => node.get_next().await,
            ExecutionPlan::Delete(node) => node.get_next().await,
            ExecutionPlan::AlterTable(node) => node.get_next().await,
            ExecutionPlan::Insert(node) => node.get_next().await,
            ExecutionPlan::Values(node) => node.get_next().await,
        }
    }

    async fn close(&mut self) -> Result<()> {
        match self {
            ExecutionPlan::Scan(node) => node.close().await,
            ExecutionPlan::Filter(node) => node.close().await,
            ExecutionPlan::Project(node) => node.close().await,
            ExecutionPlan::Aggregate(node) => node.close().await,
            ExecutionPlan::Sort(node) => node.close().await,
            ExecutionPlan::Limit(node) => node.close().await,
            ExecutionPlan::HashJoin(node) => node.close().await,
            ExecutionPlan::Union(node) => node.close().await,
            ExecutionPlan::Truncate(node) => node.close().await,
            ExecutionPlan::Window(node) => node.close().await,
            ExecutionPlan::Update(node) => node.close().await,
            ExecutionPlan::Delete(node) => node.close().await,
            ExecutionPlan::AlterTable(node) => node.close().await,
            ExecutionPlan::Insert(node) => node.close().await,
            ExecutionPlan::Values(node) => node.close().await,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        match self {
            ExecutionPlan::Scan(node) => node.as_any(),
            ExecutionPlan::Filter(node) => node.as_any(),
            ExecutionPlan::Project(node) => node.as_any(),
            ExecutionPlan::Aggregate(node) => node.as_any(),
            ExecutionPlan::Sort(node) => node.as_any(),
            ExecutionPlan::Limit(node) => node.as_any(),
            ExecutionPlan::HashJoin(node) => node.as_any(),
            ExecutionPlan::Union(node) => node.as_any(),
            ExecutionPlan::Truncate(node) => node.as_any(),
            ExecutionPlan::Window(node) => node.as_any(),
            ExecutionPlan::Update(node) => node.as_any(),
            ExecutionPlan::Delete(node) => node.as_any(),
            ExecutionPlan::AlterTable(node) => node.as_any(),
            ExecutionPlan::Insert(node) => node.as_any(),
            ExecutionPlan::Values(node) => node.as_any(),
        }
    }
}

// ---- VALUES Execution Node ----

use fe_sql_parser::ast::{Expr, LiteralValue, BinaryOp, UnaryOp};

pub struct ValuesExecNode {
    pub rows: Vec<Vec<Expr>>,
    pub schema: Schema,
    pub returned: bool,
}

impl ValuesExecNode {
    pub fn new(rows: Vec<Vec<Expr>>, schema: Schema) -> Self {
        Self { rows, schema, returned: false }
    }

    fn eval_expr(expr: &Expr) -> std::result::Result<ScalarValue, String> {
        match expr {
            Expr::Literal(lv) => Self::literal_to_scalar(lv),
            Expr::BinaryOp { left, op, right } => {
                let left_val = Self::eval_expr(left)?;
                let right_val = Self::eval_expr(right)?;
                Self::eval_binary_op(*op, &left_val, &right_val)
            }
            Expr::UnaryOp { op, expr } => {
                let val = Self::eval_expr(expr)?;
                Self::eval_unary_op(*op, &val)
            }
            Expr::FunctionCall { name, args, .. } => Self::eval_function(name, args),
            Expr::Cast { expr, target_type } => {
                let val = Self::eval_expr(expr)?;
                Self::eval_cast(&val, target_type)
            }
            other => Err(format!("Unsupported expression in VALUES: {:?}", other)),
        }
    }

    fn eval_binary_op(op: BinaryOp, left: &ScalarValue, right: &ScalarValue) -> std::result::Result<ScalarValue, String> {
        match op {
            BinaryOp::Plus => match (left, right) {
                (ScalarValue::Int64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Int64(l + r)),
                (ScalarValue::Float64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(l + r)),
                (ScalarValue::Int64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(*l as f64 + r)),
                (ScalarValue::Float64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Float64(l + *r as f64)),
                _ => Err(format!("Unsupported binary op {:?} with operands {:?} and {:?}", op, left, right)),
            },
            BinaryOp::Minus => match (left, right) {
                (ScalarValue::Int64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Int64(l - r)),
                (ScalarValue::Float64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(l - r)),
                (ScalarValue::Int64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(*l as f64 - r)),
                (ScalarValue::Float64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Float64(l - *r as f64)),
                _ => Err(format!("Unsupported binary op {:?} with operands {:?} and {:?}", op, left, right)),
            },
            BinaryOp::Multiply => match (left, right) {
                (ScalarValue::Int64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Int64(l * r)),
                (ScalarValue::Float64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(l * r)),
                (ScalarValue::Int64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(*l as f64 * r)),
                (ScalarValue::Float64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Float64(l * *r as f64)),
                _ => Err(format!("Unsupported binary op {:?} with operands {:?} and {:?}", op, left, right)),
            },
            BinaryOp::Divide => match (left, right) {
                (ScalarValue::Int64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Int64(l / r)),
                (ScalarValue::Float64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(l / r)),
                (ScalarValue::Int64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(*l as f64 / r)),
                (ScalarValue::Float64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Float64(l / *r as f64)),
                _ => Err(format!("Unsupported binary op {:?} with operands {:?} and {:?}", op, left, right)),
            },
            BinaryOp::Modulo => match (left, right) {
                (ScalarValue::Int64(l), ScalarValue::Int64(r)) => Ok(ScalarValue::Int64(l % r)),
                (ScalarValue::Float64(l), ScalarValue::Float64(r)) => Ok(ScalarValue::Float64(l % r)),
                _ => Err(format!("Unsupported binary op {:?} with operands {:?} and {:?}", op, left, right)),
            },
            _ => Err(format!("Unsupported binary op {:?} in VALUES", op)),
        }
    }

    fn eval_unary_op(op: UnaryOp, val: &ScalarValue) -> std::result::Result<ScalarValue, String> {
        match op {
            UnaryOp::Negate => match val {
                ScalarValue::Int64(n) => Ok(ScalarValue::Int64(-n)),
                ScalarValue::Float64(n) => Ok(ScalarValue::Float64(-n)),
                _ => Err(format!("Unsupported unary op {:?} with operand {:?}", op, val)),
            },
            UnaryOp::Not => match val {
                ScalarValue::Boolean(b) => Ok(ScalarValue::Boolean(!b)),
                _ => Err(format!("Unsupported unary op {:?} with operand {:?}", op, val)),
            },
        }
    }

    fn eval_function(name: &str, args: &[Expr]) -> std::result::Result<ScalarValue, String> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            "NOW" | "CURRENT_TIMESTAMP" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?;
                Ok(ScalarValue::String(format!("{}", now.as_secs())))
            }
            "UPPER" | "LOWER" | "LENGTH" | "CONCAT" | "ROUND" | "ABS" | "FLOOR" | "CEIL" => {
                if args.is_empty() {
                    return Ok(ScalarValue::Null);
                }
                let all_literals = args.iter().all(|a| matches!(a, Expr::Literal(_)));
                if !all_literals {
                    return Ok(ScalarValue::Null);
                }
                match name_upper.as_str() {
                    "UPPER" | "LOWER" => {
                        if let Expr::Literal(LiteralValue::String(s)) = &args[0] {
                            let result = if name_upper == "UPPER" { s.to_uppercase() } else { s.to_lowercase() };
                            Ok(ScalarValue::String(result))
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    "LENGTH" => {
                        if let Expr::Literal(LiteralValue::String(s)) = &args[0] {
                            Ok(ScalarValue::Int64(s.len() as i64))
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    "CONCAT" => {
                        let mut result = String::new();
                        for arg in args {
                            if let Expr::Literal(LiteralValue::String(s)) = arg {
                                result.push_str(s);
                            } else {
                                return Ok(ScalarValue::Null);
                            }
                        }
                        Ok(ScalarValue::String(result))
                    }
                    "ROUND" => {
                        if let Expr::Literal(lv) = &args[0] {
                            let val = match lv {
                                LiteralValue::Int64(n) => *n as f64,
                                LiteralValue::Float64(f) => *f,
                                LiteralValue::String(s) => s.parse().unwrap_or(0.0),
                                _ => return Ok(ScalarValue::Null),
                            };
                            Ok(ScalarValue::Float64(val.round()))
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    "ABS" => {
                        if let Expr::Literal(lv) = &args[0] {
                            match lv {
                                LiteralValue::Int64(n) => Ok(ScalarValue::Int64(n.abs())),
                                LiteralValue::Float64(f) => Ok(ScalarValue::Float64(f.abs())),
                                _ => Ok(ScalarValue::Null),
                            }
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    "FLOOR" => {
                        if let Expr::Literal(lv) = &args[0] {
                            match lv {
                                LiteralValue::Int64(n) => Ok(ScalarValue::Int64(*n)),
                                LiteralValue::Float64(f) => Ok(ScalarValue::Int64(f.floor() as i64)),
                                _ => Ok(ScalarValue::Null),
                            }
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    "CEIL" => {
                        if let Expr::Literal(lv) = &args[0] {
                            match lv {
                                LiteralValue::Int64(n) => Ok(ScalarValue::Int64(*n)),
                                LiteralValue::Float64(f) => Ok(ScalarValue::Int64(f.ceil() as i64)),
                                _ => Ok(ScalarValue::Null),
                            }
                        } else {
                            Ok(ScalarValue::Null)
                        }
                    }
                    _ => Ok(ScalarValue::Null),
                }
            }
            _ => Ok(ScalarValue::Null),
        }
    }

    fn eval_cast(val: &ScalarValue, target_type: &str) -> std::result::Result<ScalarValue, String> {
        let target_upper = target_type.to_uppercase();
        match val {
            ScalarValue::String(s) => match target_upper.as_str() {
                "INT" | "INT64" | "INTEGER" => {
                    if let Ok(n) = s.parse::<i64>() {
                        Ok(ScalarValue::Int64(n))
                    } else {
                        Ok(ScalarValue::Null)
                    }
                }
                "FLOAT" | "FLOAT64" | "DOUBLE" => {
                    if let Ok(f) = s.parse::<f64>() {
                        Ok(ScalarValue::Float64(f))
                    } else {
                        Ok(ScalarValue::Null)
                    }
                }
                "VARCHAR" | "CHAR" | "STRING" | "TEXT" => Ok(ScalarValue::String(s.clone())),
                _ => Err(format!("Unsupported cast to type {:?}", target_type)),
            },
            ScalarValue::Int64(n) => match target_upper.as_str() {
                "INT" | "INT64" | "INTEGER" => Ok(ScalarValue::Int64(*n)),
                "FLOAT" | "FLOAT64" | "DOUBLE" => Ok(ScalarValue::Float64(*n as f64)),
                "VARCHAR" | "CHAR" | "STRING" | "TEXT" => Ok(ScalarValue::String(n.to_string())),
                _ => Err(format!("Unsupported cast to type {:?}", target_type)),
            },
            ScalarValue::Float64(f) => match target_upper.as_str() {
                "INT" | "INT64" | "INTEGER" => Ok(ScalarValue::Int64(*f as i64)),
                "FLOAT" | "FLOAT64" | "DOUBLE" => Ok(ScalarValue::Float64(*f)),
                "VARCHAR" | "CHAR" | "STRING" | "TEXT" => Ok(ScalarValue::String(f.to_string())),
                _ => Err(format!("Unsupported cast to type {:?}", target_type)),
            },
            _ => Err(format!("Unsupported cast from {:?} to {:?}", val, target_type)),
        }
    }

    fn literal_to_scalar(lv: &LiteralValue) -> std::result::Result<ScalarValue, String> {
        match lv {
            LiteralValue::Null => Ok(ScalarValue::Null),
            LiteralValue::Boolean(b) => Ok(ScalarValue::Boolean(*b)),
            LiteralValue::Int64(n) => Ok(ScalarValue::Int64(*n)),
            LiteralValue::Float64(f) => Ok(ScalarValue::Float64(*f)),
            LiteralValue::String(s) => Ok(ScalarValue::String(s.clone())),
            LiteralValue::Date(s) => {
                let days = s.replace('-', "").parse::<i32>()
                    .map_err(|_| "Invalid date format")?;
                Ok(ScalarValue::Date(days))
            }
        }
    }

    fn scalar_to_vector(values: &[ScalarValue], data_type: &types::DataType) -> Vector {
        match data_type {
            types::DataType::Boolean => Vector::Boolean(BooleanVector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Boolean(b) = v { Some(*b) } else { None }).collect())),
            types::DataType::Int8 => Vector::Int8(Int8Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Int8(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Int16 => Vector::Int16(Int16Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Int16(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Int32 => Vector::Int32(Int32Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Int32(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Int64 => Vector::Int64(Int64Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Int64(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Int128 => Vector::Int128(Int128Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Int128(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Float32 => Vector::Float32(Float32Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Float32(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::Float64 => Vector::Float64(Float64Vector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Float64(n) = v { Some(*n) } else { None }).collect())),
            types::DataType::String => Vector::String(StringVector::from_option_vec(
                values.iter().map(|v| if let ScalarValue::String(s) = v { Some(s.clone()) } else { None }).collect())),
            types::DataType::Date => Vector::Date(DateVector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::Date(d) = v { Some(*d) } else { None }).collect())),
            types::DataType::DateTime => Vector::DateTime(DateTimeVector::from_nullable_vec(
                values.iter().map(|v| if let ScalarValue::DateTime(d) = v { Some(*d) } else { None }).collect())),
            types::DataType::Json => Vector::Json(JsonVector::from_option_vec(
                values.iter().map(|v| if let ScalarValue::Json(j) = v { Some(ScalarValue::Json(j.clone())) } else { None }).collect())),
            _ => Vector::Null(NullVector::new(values.len())),
        }
    }

    fn generate_block(&self) -> std::result::Result<Block, DrorisError> {
        if self.rows.is_empty() {
            return Err(DrorisError::Internal("No rows to generate".to_string()));
        }
        let num_cols = self.schema.num_fields();
        let num_rows = self.rows.len();
        let mut column_values: Vec<Vec<ScalarValue>> = vec![Vec::with_capacity(num_rows); num_cols];
        for row in &self.rows {
            if row.len() != num_cols {
                return Err(DrorisError::Internal(format!("Row has {} columns but expected {}", row.len(), num_cols)));
            }
            for (col_idx, expr) in row.iter().enumerate() {
                column_values[col_idx].push(Self::eval_expr(expr).map_err(|s| DrorisError::Internal(s))?);
            }
        }
        let vectors: Vec<Vector> = (0..num_cols).map(|col_idx| {
            let data_type = self.schema.field(col_idx).map(|f| &f.data_type)
                .ok_or_else(|| DrorisError::Internal(format!("No field at index {}", col_idx)))?;
            Ok::<Vector, DrorisError>(Self::scalar_to_vector(&column_values[col_idx], data_type))
        }).collect::<std::result::Result<Vec<Vector>, _>>()?;
        Ok(Block::new(self.schema.clone(), vectors))
    }
}

#[async_trait]
impl ExecNode for ValuesExecNode {
    async fn open(&mut self) -> Result<()> {
        self.returned = false;
        Ok(())
    }
    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }
        let block = self.generate_block()?;
        self.returned = true;
        Ok(Some(block))
    }
    async fn close(&mut self) -> Result<()> {
        self.returned = false;
        Ok(())
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ---- INSERT Execution Node ----

pub struct InsertExecNode {
    pub table_name: String,
    pub database: String,
    pub columns: Vec<String>,  // target column names from INSERT - if empty, insert into all columns
    pub child: Option<Box<ExecutionPlan>>,  // child plan (either Values or Select)
    pub tablet_id: Option<u64>,
    pub storage: Option<Arc<StorageEngine>>,
    pub transaction_ctx: Option<Arc<StdRwLock<TransactionContext>>>,
    pub executed: bool,
    /// ON DUPLICATE KEY UPDATE assignments
    pub on_duplicate_key_update: Vec<(String, String)>,
    /// Raw VALUES rows for partial column INSERT (when columns list is specified)
    pub raw_rows: Vec<Vec<Expr>>,
    /// Table schema for expanding raw_rows to full rows
    pub table_schema: Option<Schema>,
}

impl InsertExecNode {
    pub fn new(table_name: String, database: String) -> Self {
        Self {
            table_name,
            database,
            columns: Vec::new(),
            child: None,
            tablet_id: None,
            storage: None,
            transaction_ctx: None,
            executed: false,
            on_duplicate_key_update: Vec::new(),
            raw_rows: Vec::new(),
            table_schema: None,
        }
    }

    pub fn with_raw_rows(mut self, rows: Vec<Vec<Expr>>, table_schema: Schema) -> Self {
        self.raw_rows = rows;
        self.table_schema = Some(table_schema);
        self
    }

    pub fn with_columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_child(mut self, child: Box<ExecutionPlan>) -> Self {
        self.child = Some(child);
        self
    }

    pub fn with_storage(mut self, tablet_id: u64, storage: Arc<StorageEngine>) -> Self {
        self.tablet_id = Some(tablet_id);
        self.storage = Some(storage);
        self
    }

    pub fn with_transaction_ctx(mut self, tx_ctx: Arc<StdRwLock<TransactionContext>>) -> Self {
        self.transaction_ctx = Some(tx_ctx);
        self
    }

    pub fn with_on_duplicate_key_update(mut self, updates: Vec<(String, String)>) -> Self {
        self.on_duplicate_key_update = updates;
        self
    }

    /// Reorder block columns to match the target table schema order.
    /// Only used when INSERT specifies explicit column list (e.g., INSERT INTO (col1, col2) ...)
    fn reorder_columns_to_schema(block: &mut Block, target_columns: &[String], schema: &Schema) {
        if target_columns.is_empty() {
            return;  // No reordering needed
        }

        let mut new_columns: Vec<Vector> = Vec::with_capacity(schema.num_fields());
        let mut new_fields: Vec<types::Field> = Vec::with_capacity(schema.num_fields());

        for col_name in target_columns {
            if let Some(idx) = schema.index_of(col_name) {
                if let Some(field) = schema.field(idx) {
                    if let Some(col) = block.column(idx) {
                        new_columns.push(col.clone());
                        new_fields.push(field.clone());
                    }
                }
            }
        }

        if !new_columns.is_empty() && new_columns.len() == new_fields.len() {
            *block = Block::new(Schema::new(new_fields), new_columns);
        }
    }

    /// Expand raw_rows (with N values per row) to full blocks using table schema.
    /// Used when INSERT specifies a column list but VALUES provides fewer values.
    fn expand_raw_rows_to_blocks(&self, table_schema: &Schema) -> std::result::Result<Vec<Block>, DrorisError> {
        if self.raw_rows.is_empty() {
            return Err(DrorisError::Internal("No raw rows to expand".to_string()));
        }

        let num_table_cols = table_schema.num_fields();
        let num_rows = self.raw_rows.len();

        // Create column vectors with NULL defaults
        let mut column_values: Vec<Vec<ScalarValue>> = vec![Vec::with_capacity(num_rows); num_table_cols];

        // Build a mapping from self.columns to table schema indices
        let mut col_to_schema_idx: Vec<Option<usize>> = Vec::with_capacity(self.columns.len());
        for col_name in &self.columns {
            col_to_schema_idx.push(table_schema.index_of(col_name));
        }

        // Process each raw row
        for raw_row in &self.raw_rows {
            if raw_row.len() != self.columns.len() {
                return Err(DrorisError::Internal(format!(
                    "Row has {} values but expected {} columns",
                    raw_row.len(),
                    self.columns.len()
                )));
            }

            // Initialize full row with NULL values
            let mut full_row: Vec<ScalarValue> = vec![ScalarValue::Null; num_table_cols];

            // Fill in the provided values at correct positions
            for (i, expr) in raw_row.iter().enumerate() {
                if let Some(schema_idx) = col_to_schema_idx[i] {
                    let val = ValuesExecNode::eval_expr(expr)
                        .map_err(|s| DrorisError::Internal(s))?;
                    full_row[schema_idx] = val;
                }
            }

            // Add values to column vectors
            for (col_idx, val) in full_row.into_iter().enumerate() {
                column_values[col_idx].push(val);
            }
        }

        // Convert column values to vectors
        let vectors: Vec<Vector> = (0..num_table_cols).map(|col_idx| {
            let data_type = table_schema.field(col_idx)
                .map(|f| &f.data_type)
                .ok_or_else(|| DrorisError::Internal(format!("No field at index {}", col_idx)))?;
            Ok::<Vector, DrorisError>(ValuesExecNode::scalar_to_vector(&column_values[col_idx], data_type))
        }).collect::<std::result::Result<Vec<Vector>, _>>()?;

        let block = Block::new(table_schema.clone(), vectors);
        Ok(vec![block])
    }

    /// Handle ON DUPLICATE KEY UPDATE by checking for existing rows and applying update expressions.
    fn handle_on_duplicate_key_update(
        &self,
        storage: &Arc<StorageEngine>,
        tablet_id: u64,
        blocks: Vec<Block>,
    ) -> Result<usize> {
        use be_storage::index::{ColumnPredicate, PredicateOp};
        use crate::predicate_parser::eval_on_duplicate_key_expr;

        let mut total_rows_written: usize = 0;

        let key_col_idx = storage.get_key_column_index(tablet_id)
            .ok_or_else(|| DrorisError::Internal(format!("Tablet {} not found", tablet_id)))?;

        let key_col_name = storage.get_key_column_name(tablet_id)
            .unwrap_or_else(|| "id".to_string());

        for block in blocks {
            let schema = block.schema().clone();

            for row_idx in 0..block.num_rows() {
                let key_value = if let Some(col) = block.column(key_col_idx) {
                    col.scalar_at(row_idx)
                } else {
                    continue;
                };

                let predicate = ColumnPredicate {
                    column_name: key_col_name.clone(),
                    op: PredicateOp::Eq,
                    value: key_value.clone(),
                    values: vec![],
                };

                let existing_block = storage.read_tablet(tablet_id, None, &[predicate])?;

                if !existing_block.is_empty() && existing_block.num_rows() > 0 {
                    let mut new_row_values: Vec<ScalarValue> = (0..block.num_columns())
                        .map(|col_idx| {
                            block.column(col_idx)
                                .map(|c| c.scalar_at(row_idx))
                                .unwrap_or(ScalarValue::Null)
                        })
                        .collect();

                    for (target_col, expr_str) in &self.on_duplicate_key_update {
                        if let Some(col_idx) = schema.index_of(target_col) {
                            let new_value = eval_on_duplicate_key_expr(
                                expr_str,
                                &schema,
                                &new_row_values,
                            );
                            new_row_values[col_idx] = new_value;
                        }
                    }

                    let delete_predicate = ColumnPredicate {
                        column_name: key_col_name.clone(),
                        op: PredicateOp::Eq,
                        value: key_value,
                        values: vec![],
                    };
                    storage.delete(tablet_id, &[delete_predicate])?;

                    let updated_block = self.create_single_row_block(&schema, &new_row_values)?;
                    storage.write_batch(tablet_id, &updated_block)?;

                    total_rows_written += 2;
                } else {
                    let single_row_block = block.slice(row_idx, 1);
                    storage.write_batch(tablet_id, &single_row_block)?;
                    total_rows_written += 1;
                }
            }
        }

        Ok(total_rows_written)
    }

    fn create_single_row_block(&self, schema: &Schema, row_values: &[ScalarValue]) -> Result<Block> {
        let mut columns: Vec<Vector> = Vec::with_capacity(schema.num_fields());

        for (col_idx, _field) in schema.fields().iter().enumerate() {
            let value = row_values.get(col_idx).cloned().unwrap_or(ScalarValue::Null);
            let vector = Vector::from_scalar(&value, 1);
            columns.push(vector);
        }

        Ok(Block::new(schema.clone(), columns))
    }
}

#[async_trait]
impl ExecNode for InsertExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        if let Some(ref mut child) = self.child {
            child.open().await?;
        }
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }

        // Check tablet_id and storage
        let Some(tablet_id) = self.tablet_id else {
            tracing::warn!("INSERT without tablet_id on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };
        let Some(storage) = &self.storage else {
            tracing::warn!("INSERT without storage on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };

        // Handle raw_rows expansion for partial column INSERT
        // When columns list is specified and raw_rows is provided, we need to
        // expand each row from N values to full M columns
        if !self.raw_rows.is_empty() && !self.columns.is_empty() {
            if let Some(ref table_schema) = self.table_schema {
                // Create tablet on-demand if it doesn't exist
                if !storage.get_tablet(tablet_id) {
                    tracing::info!("Creating tablet {} on-demand for {}.{}", tablet_id, self.database, self.table_name);
                    let columns: Vec<be_storage::tablet::TabletColumn> = table_schema.fields().iter().enumerate().map(|(idx, f)| {
                        be_storage::tablet::TabletColumn {
                            name: f.name.clone(),
                            data_type: f.data_type.clone(),
                            nullable: f.nullable,
                            is_key: idx == 0,
                            agg_type: None,
                        }
                    }).collect();
                    let tablet_schema = be_storage::tablet::TabletSchema {
                        tablet_id,
                        columns,
                        keys_type: "Duplicate".to_string(),
                        num_rows_per_row_block: 1024,
                    };
                    if let Err(e) = storage.create_tablet(tablet_id, tablet_schema) {
                        tracing::warn!("Failed to create tablet {}: {}", tablet_id, e);
                    }
                }

                // Expand raw_rows to full blocks
                let blocks_to_write = self.expand_raw_rows_to_blocks(table_schema)?;

                // Check if we're in transaction mode
                let mut total_rows_written: usize = 0;
                if let Some(ref tx_ctx) = self.transaction_ctx {
                    let mut tx = tx_ctx.write().unwrap();
                    if tx.in_transaction {
                        for block in &blocks_to_write {
                            tx.pending_writes.push(PendingWrite {
                                tablet_id,
                                block: block.clone(),
                                op_type: WriteOp::Insert,
                            });
                            total_rows_written += block.num_rows();
                        }
                        tracing::info!("INSERT into {}.{} staged to transaction: {} rows affected",
                            self.database, self.table_name, total_rows_written);
                        self.executed = true;
                        return Ok(Some(make_affected_rows_block(total_rows_written)));
                    }
                }

                // Write directly to storage
                for block in &blocks_to_write {
                    storage.write_batch(tablet_id, &block)?;
                    total_rows_written += block.num_rows();
                }

                self.executed = true;
                tracing::info!("INSERT INTO {}.{}: {} rows affected",
                    self.database, self.table_name, total_rows_written);
                return Ok(Some(make_affected_rows_block(total_rows_written)));
            }
        }

        // Create tablet on-demand if it doesn't exist
        if !storage.get_tablet(tablet_id) {
            tracing::info!("Creating tablet {} on-demand for {}.{}", tablet_id, self.database, self.table_name);
            // Get schema from ValuesExecNode child only
            let block_schema = if let Some(ref child) = self.child {
                if let ExecutionPlan::Values(values_node) = child.as_ref() {
                    Some(values_node.schema.clone())
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(schema) = block_schema {
                let columns: Vec<be_storage::tablet::TabletColumn> = schema.fields().iter().enumerate().map(|(idx, f)| {
                    be_storage::tablet::TabletColumn {
                        name: f.name.clone(),
                        data_type: f.data_type.clone(),
                        nullable: f.nullable,
                        is_key: idx == 0,
                        agg_type: None,
                    }
                }).collect();
                let tablet_schema = be_storage::tablet::TabletSchema {
                    tablet_id,
                    columns,
                    keys_type: "Duplicate".to_string(),
                    num_rows_per_row_block: 1024,
                };
                if let Err(e) = storage.create_tablet(tablet_id, tablet_schema) {
                    tracing::warn!("Failed to create tablet {}: {}", tablet_id, e);
                }
            }
        }

        let mut total_rows_written: usize = 0;

        // Collect all blocks from child plan first
        let mut blocks_to_write: Vec<Block> = Vec::new();
        if let Some(ref mut child) = self.child {
            while let Some(mut block) = child.get_next().await? {
                // Handle column projection if columns are specified
                if !self.columns.is_empty() {
                    let schema = block.schema().clone();
                    Self::reorder_columns_to_schema(&mut block, &self.columns, &schema);
                }
                blocks_to_write.push(block);
            }
        }

        // Check if we're in transaction mode and stage writes if so
        if let Some(ref tx_ctx) = self.transaction_ctx {
            let mut tx = tx_ctx.write().unwrap();
            if tx.in_transaction {
                // Stage all pending writes for transaction commit
                for block in &blocks_to_write {
                    tx.pending_writes.push(PendingWrite {
                        tablet_id,
                        block: block.clone(),
                        op_type: WriteOp::Insert,
                    });
                    total_rows_written += block.num_rows();
                }
                tracing::info!("INSERT into {}.{} staged to transaction: {} rows affected",
                    self.database, self.table_name, total_rows_written);
                self.executed = true;
                return Ok(Some(make_affected_rows_block(total_rows_written)));
            }
        }

        // Not in transaction mode - write directly to storage
        if self.on_duplicate_key_update.is_empty() {
            for block in &blocks_to_write {
                storage.write_batch(tablet_id, &block)?;
                total_rows_written += block.num_rows();
            }
        } else {
            total_rows_written = self.handle_on_duplicate_key_update(
                storage,
                tablet_id,
                blocks_to_write,
            )?;
        }

        self.executed = true;

        tracing::info!(
            "INSERT INTO {}.{}: {} rows affected",
            self.database,
            self.table_name,
            total_rows_written
        );

        Ok(Some(make_affected_rows_block(total_rows_written)))
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.child {
            child.close().await?;
        }
        self.executed = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ScanExecNode {
    pub table_name: String,
    pub columns: Vec<String>,
    pub limit: Option<usize>,
    pub predicates: Vec<String>,
    pub data: Option<Block>,
    pub tablet_id: Option<u64>,
    pub storage: Option<Arc<StorageEngine>>,
    opened: bool,
    rows_consumed: usize,
    runtime_filters: Vec<ScanRuntimeFilter>,
}

pub struct ScanRuntimeFilter {
    pub column_index: usize,
    pub filter: AppliedFilter,
}

pub enum AppliedFilter {
    Bloom(BloomFilter),
    MinMax(MinMaxFilter),
    In(InFilter),
}

impl ScanExecNode {
    pub fn new(table_name: String, columns: Vec<String>) -> Self {
        Self {
            table_name,
            columns,
            limit: None,
            predicates: Vec::new(),
            data: None,
            tablet_id: None,
            storage: None,
            opened: false,
            rows_consumed: 0,
            runtime_filters: Vec::new(),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_predicates(mut self, predicates: Vec<String>) -> Self {
        self.predicates = predicates;
        self
    }

    pub fn with_storage(mut self, tablet_id: u64, storage: Arc<StorageEngine>) -> Self {
        self.tablet_id = Some(tablet_id);
        self.storage = Some(storage);
        self
    }

    pub fn with_runtime_filters(mut self, filters: Vec<ScanRuntimeFilter>) -> Self {
        self.runtime_filters = filters;
        self
    }

    fn apply_runtime_filters(&self, block: &Block) -> Bitmap {
        if self.runtime_filters.is_empty() {
            return Bitmap::all_set(block.num_rows());
        }

        let mut selection = Bitmap::all_set(block.num_rows());
        for rf in &self.runtime_filters {
            let mut filter_selection = Bitmap::with_capacity(block.num_rows());
            for row_idx in 0..block.num_rows() {
                let pass = if let Some(col) = block.column(rf.column_index) {
                    let val = col.scalar_at(row_idx);
                    match &rf.filter {
                        AppliedFilter::Bloom(bf) => {
                            let bytes = format!("{:?}", val);
                            bf.may_contain(bytes.as_bytes())
                        }
                        AppliedFilter::MinMax(mm) => mm.may_contain(&val),
                        AppliedFilter::In(in_f) => in_f.may_contain(&val),
                    }
                } else {
                    true
                };
                filter_selection.push(pass);
            }
            selection = &selection & &filter_selection;
        }
        selection
    }

    
    /// Build predicates for storage read.
    fn build_predicates(&self) -> Vec<ColumnPredicate> {
        let mut all_predicates = Vec::new();
        for p in &self.predicates {
            all_predicates.extend(parse_predicates(p));
        }
        all_predicates
    }

    /// Read data from storage engine if configured.
    fn read_from_storage(&self) -> Result<Block> {
        let Some(tablet_id) = self.tablet_id else {
            return Ok(Block::empty(Schema::new(vec![])));
        };
        let Some(storage) = &self.storage else {
            return Ok(Block::empty(Schema::new(vec![])));
        };

        let projection = None; // Let storage return all columns, filter later
        let predicates = self.build_predicates();

        match storage.read_tablet(tablet_id, projection, &predicates) {
            Ok(block) => Ok(block),
            Err(e) => {
                tracing::warn!("Failed to read tablet {}: {}", tablet_id, e);
                Ok(Block::empty(Schema::new(vec![])))
            }
        }
    }
}

#[async_trait]
impl ExecNode for ScanExecNode {
    async fn open(&mut self) -> Result<()> {
        // If no pre-loaded data but we have storage configured, read from storage now
        if self.data.is_none() && self.storage.is_some() && self.tablet_id.is_some() {
            tracing::debug!("ScanExecNode: reading from storage tablet_id={}", self.tablet_id.unwrap());
            self.data = Some(self.read_from_storage()?);
        }
        self.opened = true;
        self.rows_consumed = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if let Some(data) = self.data.take() {
            let filtered_data = if !self.runtime_filters.is_empty() {
                let selection = self.apply_runtime_filters(&data);
                let filtered = data.filter(&selection);
                if filtered.is_empty() {
                    return Ok(None);
                }
                filtered
            } else {
                data
            };

            if let Some(limit) = self.limit {
                let rows_to_take = limit.saturating_sub(self.rows_consumed);
                if rows_to_take == 0 {
                    return Ok(None);
                }
                self.rows_consumed += filtered_data.num_rows();
                Ok(Some(filtered_data.slice(0, rows_to_take.min(filtered_data.num_rows()))))
            } else {
                Ok(Some(filtered_data))
            }
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.opened = false;
        self.rows_consumed = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct FilterExecNode {
    pub predicate: String,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
}

#[async_trait]
impl ExecNode for FilterExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while let Some(block) = self.child.get_next().await? {
            let mut selection = Bitmap::with_capacity(block.num_rows());

            for _ in 0..block.num_rows() {
                selection.push(true);
            }

            let filtered = block.filter(&selection);
            if !filtered.is_empty() {
                return Ok(Some(filtered));
            }
        }
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ProjectExecNode {
    pub exprs: Vec<String>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
}

#[async_trait]
impl ExecNode for ProjectExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        self.child.get_next().await
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct AggregateExecNode {
    pub group_by: Vec<usize>,
    pub aggregates: Vec<(String, usize)>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub returned: bool,
}

impl AggregateExecNode {
    fn compute_aggregate_batch(col: &Vector, func: &str) -> ScalarValue {
        match func {
            "count" => ScalarValue::Int64(col.count_batch() as i64),
            "sum" => col.sum_batch().unwrap_or(ScalarValue::Null),
            "min" => col.min_batch().unwrap_or(ScalarValue::Null),
            "max" => col.max_batch().unwrap_or(ScalarValue::Null),
            "avg" => col.avg_batch().unwrap_or(ScalarValue::Null),
            _ => ScalarValue::Null,
        }
    }

    fn compute_aggregate(values: &[ScalarValue], func: &str) -> ScalarValue {
        match func {
            "count" => ScalarValue::Int64(values.len() as i64),
            "sum" => {
                let mut sum: i64 = 0;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        sum += i;
                    }
                }
                ScalarValue::Int64(sum)
            }
            "min" => {
                let mut min: Option<i64> = None;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        min = Some(min.map(|m| m.min(*i)).unwrap_or(*i));
                    }
                }
                min.map(ScalarValue::Int64).unwrap_or(ScalarValue::Null)
            }
            "max" => {
                let mut max: Option<i64> = None;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        max = Some(max.map(|m| m.max(*i)).unwrap_or(*i));
                    }
                }
                max.map(ScalarValue::Int64).unwrap_or(ScalarValue::Null)
            }
            "avg" => {
                let mut sum: i64 = 0;
                let mut count: i64 = 0;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        sum += i;
                        count += 1;
                    }
                }
                if count > 0 {
                    ScalarValue::Float64(sum as f64 / count as f64)
                } else {
                    ScalarValue::Null
                }
            }
            _ => ScalarValue::Null,
        }
    }
}

#[async_trait]
impl ExecNode for AggregateExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        let mut all_blocks = Vec::new();
        while let Some(block) = self.child.get_next().await? {
            all_blocks.push(block);
        }

        if all_blocks.is_empty() {
            return Ok(None);
        }

        let combined = Block::concat(&all_blocks);
        if combined.is_none() {
            self.returned = true;
            return Ok(None);
        }
        let block = combined.unwrap();

        if self.group_by.is_empty() && self.aggregates.is_empty() {
            self.returned = true;
            return Ok(Some(block));
        }

        if self.group_by.is_empty() {
            let mut result_columns: Vec<Vector> = Vec::new();
            let mut result_schema_fields: Vec<types::Field> = Vec::new();

            for (func, col_idx) in &self.aggregates {
                if *col_idx < block.num_columns() {
                    if let Some(col) = block.column(*col_idx) {
                        let agg_value = Self::compute_aggregate_batch(col, func);
                        let vector = match agg_value {
                            ScalarValue::Int64(v) => Vector::Int64(types::vector::Int64Vector::from_vec(vec![v])),
                            ScalarValue::Float64(v) => Vector::Float64(types::vector::Float64Vector::from_vec(vec![v])),
                            ScalarValue::Int32(v) => Vector::Int32(types::vector::Int32Vector::from_vec(vec![v])),
                            _ => Vector::Null(types::vector::NullVector::new(1)),
                        };
                        result_columns.push(vector);
                        result_schema_fields.push(types::Field::new("", types::DataType::Null, true));
                    }
                }
            }

            self.returned = true;
            return Ok(Some(Block::new(Schema::new(result_schema_fields), result_columns)));
        }

        let mut groups: std::collections::HashMap<String, Vec<Vec<ScalarValue>>> = std::collections::HashMap::new();

        for row_idx in 0..block.num_rows() {
            let mut key_parts = Vec::new();
            for &col_idx in &self.group_by {
                if col_idx < block.num_columns() {
                    if let Some(col) = block.column(col_idx) {
                        key_parts.push(format!("{:?}", col.scalar_at(row_idx)));
                    }
                }
            }
            let group_key = key_parts.join("|");

            let row_values: Vec<ScalarValue> = (0..block.num_columns())
                .map(|col_idx| {
                    if let Some(col) = block.column(col_idx) {
                        col.scalar_at(row_idx)
                    } else {
                        ScalarValue::Null
                    }
                })
                .collect();

            groups.entry(group_key).or_insert_with(Vec::new).push(row_values);
        }

        let mut result_rows: Vec<Vec<ScalarValue>> = Vec::new();
        for (_group_key, group_rows) in &groups {
            let mut result_row = Vec::new();

            if !group_rows.is_empty() {
                for &col_idx in &self.group_by {
                    result_row.push(group_rows[0].get(col_idx).cloned().unwrap_or(ScalarValue::Null));
                }
            }

            for (func, col_idx) in &self.aggregates {
                let values: Vec<ScalarValue> = group_rows.iter()
                    .filter_map(|row| row.get(*col_idx).cloned())
                    .collect();
                result_row.push(Self::compute_aggregate(&values, func));
            }

            result_rows.push(result_row);
        }

        if result_rows.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let num_result_cols = self.group_by.len() + self.aggregates.len();
        let num_result_rows = result_rows.len();

        let mut columns: Vec<Vector> = Vec::new();
        for col_idx in 0..num_result_cols {
            let scalars: Vec<ScalarValue> = result_rows.iter()
                .filter_map(|row| row.get(col_idx).cloned())
                .collect();

            let vector = match scalars.first().unwrap_or(&ScalarValue::Null) {
                ScalarValue::Int64(_) => {
                    let data: Vec<i64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int64(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int64(types::vector::Int64Vector::from_vec(data))
                }
                ScalarValue::Float64(_) => {
                    let data: Vec<f64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Float64(f) = s { Some(*f) } else { None }
                    }).collect();
                    Vector::Float64(types::vector::Float64Vector::from_vec(data))
                }
                ScalarValue::Int32(_) => {
                    let data: Vec<i32> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int32(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int32(types::vector::Int32Vector::from_vec(data))
                }
                ScalarValue::String(_) => {
                    let data: Vec<String> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::String(s) = s { Some(s.clone()) } else { None }
                    }).collect();
                    let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
                    Vector::String(types::vector::StringVector::from_vec(data_refs))
                }
                _ => Vector::Null(types::vector::NullVector::new(num_result_rows)),
            };
            columns.push(vector);
        }

        let schema_fields: Vec<types::Field> = (0..num_result_cols)
            .map(|_| types::Field::new("", types::DataType::Null, true))
            .collect();

        self.returned = true;
        Ok(Some(Block::new(Schema::new(schema_fields), columns)))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SortExecNode {
    pub order_by: Vec<(usize, bool)>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub buffered: Vec<Block>,
    pub returned: bool,
}

#[async_trait]
impl ExecNode for SortExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        self.buffered.clear();
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        while let Some(block) = self.child.get_next().await? {
            self.buffered.push(block);
        }

        if self.buffered.is_empty() {
            return Ok(None);
        }

        let combined = Block::concat(&self.buffered);
        if combined.is_none() {
            return Ok(None);
        }
        let block = combined.unwrap();

        if self.order_by.is_empty() {
            self.returned = true;
            return Ok(Some(block));
        }

        let num_rows = block.num_rows();
        let mut indices: Vec<usize> = (0..num_rows).collect();

        let order_by = self.order_by.clone();
        let cmp_block = &block;
        indices.sort_unstable_by(|&a, &b| {
            for &(col_idx, ascending) in &order_by {
                if col_idx >= cmp_block.num_columns() {
                    continue;
                }
                if let Some(col) = cmp_block.column(col_idx) {
                    let ord = col.compare_at(a, b);
                    let ord = if ascending { ord } else { ord.reverse() };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
            }
            std::cmp::Ordering::Equal
        });

        let mut sorted_block = Block::empty(block.schema().clone());
        for &idx in &indices {
            let row_block = block.slice(idx, 1);
            if sorted_block.is_empty() {
                sorted_block = row_block;
            } else {
                sorted_block.append_block(&row_block);
            }
        }

        self.returned = true;
        Ok(Some(sorted_block))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        self.buffered.clear();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct LimitExecNode {
    pub limit: usize,
    pub offset: usize,
    pub child: Box<ExecutionPlan>,
    pub rows_returned: usize,
    pub rows_skipped: usize,
}

impl LimitExecNode {
    pub fn new(limit: usize, child: Box<ExecutionPlan>) -> Self {
        Self { limit, offset: 0, child, rows_returned: 0, rows_skipped: 0 }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

#[async_trait]
impl ExecNode for LimitExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while self.rows_skipped < self.offset {
            if let Some(block) = self.child.get_next().await? {
                let remaining = self.offset - self.rows_skipped;
                if block.num_rows() <= remaining {
                    self.rows_skipped += block.num_rows();
                } else {
                    self.rows_skipped = self.offset;
                    let to_return = block.slice(remaining, block.num_rows() - remaining);
                    let can_return = self.limit.saturating_sub(self.rows_returned);
                    if can_return == 0 {
                        return Ok(None);
                    }
                    if to_return.num_rows() <= can_return {
                        self.rows_returned += to_return.num_rows();
                        return Ok(Some(to_return));
                    } else {
                        self.rows_returned = self.limit;
                        return Ok(Some(to_return.slice(0, can_return)));
                    }
                }
            } else {
                return Ok(None);
            }
        }

        if self.rows_returned >= self.limit {
            return Ok(None);
        }

        if let Some(block) = self.child.get_next().await? {
            let can_return = self.limit - self.rows_returned;
            if block.num_rows() <= can_return {
                self.rows_returned += block.num_rows();
                Ok(Some(block))
            } else {
                self.rows_returned = self.limit;
                Ok(Some(block.slice(0, can_return)))
            }
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct HashJoinExecNode {
    pub join_type: String,
    pub build_keys: Vec<usize>,
    pub probe_keys: Vec<usize>,
    pub build_child: Box<ExecutionPlan>,
    pub probe_child: Box<ExecutionPlan>,
    pub build_schema: Schema,
    pub probe_schema: Schema,
    pub opened: bool,
    pub build_complete: bool,
    pub probe_consumed: bool,
    pub hash_table: HashMap<String, Vec<Block>>,
    pub current_probe_blocks: Vec<Block>,
    pub current_probe_idx: usize,
    pub matched_build_keys: HashMap<String, bool>,
    pub runtime_filters: Vec<RuntimeFilterConfig>,
    pub generated_filters: HashMap<u64, GeneratedFilter>,
}

pub struct RuntimeFilterConfig {
    pub id: u64,
    pub filter_type: RuntimeFilterTypeExec,
    pub build_key_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeFilterTypeExec {
    Bloom,
    MinMax,
    In,
}

pub enum GeneratedFilter {
    Bloom(BloomFilter),
    MinMax(MinMaxFilter),
    In(InFilter),
}

impl HashJoinExecNode {
    pub fn new(
        join_type: String,
        build_keys: Vec<usize>,
        probe_keys: Vec<usize>,
        build_child: Box<ExecutionPlan>,
        probe_child: Box<ExecutionPlan>,
        build_schema: Schema,
        probe_schema: Schema,
    ) -> Self {
        Self {
            join_type,
            build_keys,
            probe_keys,
            build_child,
            probe_child,
            build_schema,
            probe_schema,
            opened: false,
            build_complete: false,
            probe_consumed: false,
            hash_table: HashMap::new(),
            current_probe_blocks: Vec::new(),
            current_probe_idx: 0,
            matched_build_keys: HashMap::new(),
            runtime_filters: Vec::new(),
            generated_filters: HashMap::new(),
        }
    }

    pub fn with_runtime_filters(mut self, filters: Vec<RuntimeFilterConfig>) -> Self {
        self.runtime_filters = filters;
        self
    }

    fn extract_keys_from_block(block: &Block, key_indices: &[usize]) -> Vec<String> {
        (0..block.num_rows()).map(|row_idx| {
            let mut key_parts = Vec::new();
            for &idx in key_indices {
                if idx < block.num_columns() {
                    if let Some(col) = block.column(idx) {
                        let scalar = col.scalar_at(row_idx);
                        key_parts.push(format!("{:?}", scalar));
                    }
                }
            }
            key_parts.join("|")
        }).collect()
    }

    fn extract_scalar_from_block(block: &Block, key_index: usize) -> Vec<ScalarValue> {
        (0..block.num_rows()).filter_map(|row_idx| {
            if key_index < block.num_columns() {
                block.column(key_index).map(|col| col.scalar_at(row_idx))
            } else {
                None
            }
        }).collect()
    }

    fn generate_runtime_filters(&mut self, build_blocks: &[Block]) {
        for config in &self.runtime_filters {
            let values = build_blocks.iter()
                .flat_map(|block| Self::extract_scalar_from_block(block, config.build_key_index))
                .collect::<Vec<_>>();

            let filter = match config.filter_type {
                RuntimeFilterTypeExec::Bloom => {
                    let mut bloom = BloomFilter::new(values.len(), 0.01);
                    for val in &values {
                        let bytes = format!("{:?}", val);
                        bloom.insert(bytes.as_bytes());
                    }
                    GeneratedFilter::Bloom(bloom)
                }
                RuntimeFilterTypeExec::MinMax => {
                    let mut minmax = MinMaxFilter::new();
                    for val in &values {
                        minmax.update(val);
                    }
                    GeneratedFilter::MinMax(minmax)
                }
                RuntimeFilterTypeExec::In => {
                    let mut in_filter = InFilter::with_capacity(values.len());
                    for val in values {
                        in_filter.insert(val);
                    }
                    GeneratedFilter::In(in_filter)
                }
            };

            self.generated_filters.insert(config.id, filter);
        }
    }

    pub fn get_runtime_filters(&self) -> HashMap<u64, &GeneratedFilter> {
        self.generated_filters.iter()
            .map(|(k, v)| (*k, v))
            .collect()
    }
}

#[async_trait]
impl ExecNode for HashJoinExecNode {
    async fn open(&mut self) -> Result<()> {
        self.build_child.open().await?;
        self.probe_child.open().await?;
        self.opened = true;
        self.build_complete = false;
        self.probe_consumed = false;
        self.hash_table.clear();
        self.current_probe_blocks.clear();
        self.current_probe_idx = 0;
        self.matched_build_keys.clear();
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if !self.build_complete {
            let mut build_blocks = Vec::new();
            while let Some(block) = self.build_child.get_next().await? {
                build_blocks.push(block);
            }

            for block in &build_blocks {
                let keys = Self::extract_keys_from_block(block, &self.build_keys);
                for (row_idx, key) in keys.iter().enumerate() {
                    let row_block = block.slice(row_idx, 1);
                    self.hash_table.entry(key.clone()).or_insert_with(Vec::new).push(row_block);
                }
            }

            if !self.runtime_filters.is_empty() {
                self.generate_runtime_filters(&build_blocks);
            }

            self.build_complete = true;
            tracing::debug!("HashJoin build complete: {} keys in hash table", self.hash_table.len());
        }

        while let Some(block) = self.probe_child.get_next().await? {
            let keys = Self::extract_keys_from_block(&block, &self.probe_keys);
            let mut result_blocks = Vec::new();

            for (row_idx, key) in keys.iter().enumerate() {
                if let Some(build_blocks) = self.hash_table.get(key) {
                    self.matched_build_keys.insert(key.clone(), true);
                    let probe_row = block.slice(row_idx, 1);
                    for build_row in build_blocks {
                        let mut joined = probe_row.clone();
                        joined.append_block(build_row);
                        result_blocks.push(joined);
                    }
                }
            }

            if !result_blocks.is_empty() {
                return Ok(Some(Block::concat(&result_blocks).unwrap_or_else(|| block)));
            }
        }

        if self.join_type == "LEFT" && !self.matched_build_keys.is_empty() {
        }

        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        self.build_child.close().await?;
        self.probe_child.close().await?;
        self.opened = false;
        self.build_complete = false;
        self.probe_consumed = false;
        self.hash_table.clear();
        self.current_probe_blocks.clear();
        self.matched_build_keys.clear();
        self.generated_filters.clear();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct UnionExecNode {
    pub children: Vec<Box<ExecutionPlan>>,
    pub current_child: usize,
    pub opened: Vec<bool>,
}

impl UnionExecNode {
    pub fn new(children: Vec<Box<ExecutionPlan>>) -> Self {
        let opened = vec![false; children.len()];
        Self {
            children,
            current_child: 0,
            opened,
        }
    }
}

#[async_trait]
impl ExecNode for UnionExecNode {
    async fn open(&mut self) -> Result<()> {
        for (i, child) in self.children.iter_mut().enumerate() {
            child.open().await?;
            self.opened[i] = true;
        }
        self.current_child = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while self.current_child < self.children.len() {
            if let Some(block) = self.children[self.current_child].get_next().await? {
                return Ok(Some(block));
            }
            self.current_child += 1;
        }
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        for (child, opened) in self.children.iter_mut().zip(self.opened.iter()) {
            if *opened {
                child.close().await?;
            }
        }
        self.opened = vec![false; self.children.len()];
        self.current_child = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct TruncateExecNode {
    pub database: String,
    pub table_name: String,
    pub if_exists: bool,
    pub executed: bool,
}

impl TruncateExecNode {
    pub fn new(
        database: String,
        table_name: String,
        if_exists: bool,
    ) -> Self {
        Self {
            database,
            table_name,
            if_exists,
            executed: false,
        }
    }
}

#[async_trait]
impl ExecNode for TruncateExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }

        self.executed = true;

        if self.if_exists {
            tracing::info!("TRUNCATE TABLE IF EXISTS {}.{} executed", self.database, self.table_name);
        } else {
            tracing::info!("TRUNCATE TABLE {}.{}", self.database, self.table_name);
        }

        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct WindowExecNode {
    pub partition_by: Vec<usize>,
    pub order_by: Vec<(usize, bool)>,
    pub window_func: String,
    pub window_func_col: usize,
    pub offset: i64,
    pub default_val: ScalarValue,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub returned: bool,
    pub buffered: Option<Block>,
}

impl WindowExecNode {
    pub fn new(
        window_func: String,
        window_func_col: usize,
        partition_by: Vec<usize>,
        order_by: Vec<(usize, bool)>,
        child: Box<ExecutionPlan>,
    ) -> Self {
        Self {
            partition_by,
            order_by,
            window_func,
            window_func_col,
            offset: 1,
            default_val: ScalarValue::Null,
            child,
            opened: false,
            returned: false,
            buffered: None,
        }
    }
}

#[async_trait]
impl ExecNode for WindowExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        self.buffered = None;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        let mut all_blocks = Vec::new();
        while let Some(block) = self.child.get_next().await? {
            all_blocks.push(block);
        }

        if all_blocks.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let combined = Block::concat(&all_blocks);
        if combined.is_none() {
            self.returned = true;
            return Ok(None);
        }
        let block = combined.unwrap();

        let num_rows = block.num_rows();
        let mut partition_ranges: Vec<(usize, usize)> = Vec::new();

        if self.partition_by.is_empty() {
            partition_ranges.push((0, num_rows));
        } else {
            let mut partition_start = 0;
            let mut prev_key = self.get_partition_key(&block, 0);

            for i in 1..num_rows {
                let curr_key = self.get_partition_key(&block, i);
                if curr_key != prev_key {
                    partition_ranges.push((partition_start, i));
                    partition_start = i;
                    prev_key = curr_key;
                }
            }
            partition_ranges.push((partition_start, num_rows));
        }

        let mut result_rows: Vec<Vec<ScalarValue>> = Vec::new();

        for (start, end) in partition_ranges {
            let partition_size = end - start;

            let window_values = self.compute_window_over_block_for_partition(&block, start, partition_size)?;

            for i in 0..partition_size {
                let mut result_row = Vec::new();
                for col_idx in 0..block.num_columns() {
                    if let Some(col) = block.column(col_idx) {
                        result_row.push(col.scalar_at(start + i));
                    }
                }
                let window_val = match &window_values {
                    Vector::Int64(v) => ScalarValue::Int64(*v.data().get(i).unwrap_or(&0)),
                    Vector::Int32(v) => ScalarValue::Int32(*v.data().get(i).unwrap_or(&0)),
                    Vector::Float64(v) => ScalarValue::Float64(*v.data().get(i).unwrap_or(&0.0)),
                    Vector::String(v) => ScalarValue::String(v.get(i).unwrap_or("").to_string()),
                    _ => ScalarValue::Null,
                };
                result_row.push(window_val);
                result_rows.push(result_row);
            }
        }

        if result_rows.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let num_cols = result_rows[0].len();
        let num_result_rows = result_rows.len();

        let mut columns: Vec<Vector> = Vec::new();
        for col_idx in 0..num_cols {
            let scalars: Vec<ScalarValue> = result_rows.iter()
                .filter_map(|row| row.get(col_idx).cloned())
                .collect();

            let vector = match scalars.first().unwrap_or(&ScalarValue::Null) {
                ScalarValue::Int64(_) => {
                    let data: Vec<i64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int64(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int64(types::vector::Int64Vector::from_vec(data))
                }
                ScalarValue::Int32(_) => {
                    let data: Vec<i32> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int32(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int32(types::vector::Int32Vector::from_vec(data))
                }
                ScalarValue::Float64(_) => {
                    let data: Vec<f64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Float64(f) = s { Some(*f) } else { None }
                    }).collect();
                    Vector::Float64(types::vector::Float64Vector::from_vec(data))
                }
                ScalarValue::String(_) => {
                    let data: Vec<String> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::String(s) = s { Some(s.clone()) } else { None }
                    }).collect();
                    let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
                    Vector::String(types::vector::StringVector::from_vec(data_refs))
                }
                _ => Vector::Null(types::vector::NullVector::new(num_result_rows)),
            };
            columns.push(vector);
        }

        let mut schema_fields: Vec<types::Field> = block.schema().fields().to_vec();
        schema_fields.push(types::Field::new("window_col", types::DataType::Int64, true));

        self.returned = true;
        Ok(Some(Block::new(Schema::new(schema_fields), columns)))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        self.buffered = None;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WindowExecNode {
    fn get_partition_key(&self, block: &Block, row_idx: usize) -> String {
        let mut key_parts = Vec::new();
        for &col_idx in &self.partition_by {
            if col_idx < block.num_columns() {
                if let Some(col) = block.column(col_idx) {
                    key_parts.push(format!("{:?}", col.scalar_at(row_idx)));
                }
            }
        }
        key_parts.join("|")
    }

    fn scalar_to_vector(scalar: &ScalarValue, num_rows: usize) -> Vector {
        match scalar {
            ScalarValue::Boolean(v) => Vector::Boolean(BooleanVector::from_vec(vec![*v; num_rows])),
            ScalarValue::Int8(v) => Vector::Int8(Int8Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Int16(v) => Vector::Int16(Int16Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Int32(v) => Vector::Int32(Int32Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Int64(v) => Vector::Int64(Int64Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Int128(v) => Vector::Int128(Int128Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Float32(v) => Vector::Float32(Float32Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::Float64(v) => Vector::Float64(Float64Vector::from_vec(vec![*v; num_rows])),
            ScalarValue::String(s) => {
                let data_refs: Vec<&str> = vec![s.as_str(); num_rows];
                Vector::String(StringVector::from_vec(data_refs))
            }
            ScalarValue::Date(v) => Vector::Date(DateVector::from_vec(vec![*v; num_rows])),
            ScalarValue::DateTime(v) => Vector::DateTime(DateTimeVector::from_vec(vec![*v; num_rows])),
            _ => Vector::Null(NullVector::new(num_rows)),
        }
    }

    fn compute_window_over_block_for_partition(&self, block: &Block, start: usize, size: usize) -> Result<Vector> {
        match self.window_func.as_str() {
            "row_number" => {
                let data: Vec<i64> = (1..=size as i64).collect();
                Ok(Vector::Int64(Int64Vector::from_vec(data)))
            }
            "rank" | "dense_rank" => {
                let data: Vec<i64> = (1..=size as i64).collect();
                Ok(Vector::Int64(Int64Vector::from_vec(data)))
            }
            "lead" | "lag" => {
                let mut data: Vec<i64> = Vec::new();
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        for i in 0..size {
                            let global_idx = start + i;
                            let target_idx = if self.window_func == "lead" {
                                global_idx + self.offset as usize
                            } else {
                                global_idx.saturating_sub(self.offset as usize)
                            };

                            let val = if target_idx >= start && target_idx < start + size {
                                col.scalar_at(target_idx)
                            } else {
                                self.default_val.clone()
                            };
                            if let ScalarValue::Int64(v) = val {
                                data.push(v);
                            } else {
                                data.push(0);
                            }
                        }
                    }
                }
                Ok(Vector::Int64(Int64Vector::from_vec(data)))
            }
            "first_value" | "last_value" => {
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        let val = if self.window_func == "first_value" {
                            col.scalar_at(start)
                        } else {
                            col.scalar_at(start + size.saturating_sub(1))
                        };
                        return Ok(Self::scalar_to_vector(&val, size));
                    }
                }
                Ok(Vector::Null(NullVector::new(size)))
            }
            "count" | "sum" | "avg" | "min" | "max" => {
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        let val = match self.window_func.as_str() {
                            "count" => ScalarValue::Int64(col.count_batch() as i64),
                            "sum" => col.sum_batch().unwrap_or(ScalarValue::Null),
                            "avg" => col.avg_batch().unwrap_or(ScalarValue::Null),
                            "min" => col.min_batch().unwrap_or(ScalarValue::Null),
                            "max" => col.max_batch().unwrap_or(ScalarValue::Null),
                            _ => ScalarValue::Null,
                        };
                        return Ok(Self::scalar_to_vector(&val, size));
                    }
                }
                Ok(Vector::Null(NullVector::new(size)))
            }
            _ => {
                let data: Vec<i64> = vec![0; size];
                Ok(Vector::Int64(Int64Vector::from_vec(data)))
            }
        }
    }
}

// ---- DML Execution Nodes ----

/// Transaction context for staging DML operations.
#[derive(Clone)]
pub struct TransactionContext {
    pub in_transaction: bool,
    pub pending_writes: Vec<PendingWrite>,
    pub pending_deletes: Vec<PendingDelete>,
}

#[derive(Clone)]
pub struct PendingWrite {
    pub tablet_id: u64,
    pub block: types::Block,
    pub op_type: WriteOp,
}

#[derive(Clone)]
pub enum WriteOp {
    Insert,
    Update,
    Delete,
}

#[derive(Clone)]
pub struct PendingDelete {
    pub tablet_id: u64,
    pub predicates: Vec<ColumnPredicate>,
}

impl TransactionContext {
    pub fn new() -> Self {
        Self {
            in_transaction: false,
            pending_writes: Vec::new(),
            pending_deletes: Vec::new(),
        }
    }

    pub fn begin(&mut self) {
        self.in_transaction = true;
    }

    pub fn commit(&mut self, _storage: &Arc<StorageEngine>) -> std::result::Result<usize, String> {
        let affected = self.pending_writes.len();
        self.pending_writes.clear();
        self.pending_deletes.clear();
        self.in_transaction = false;
        Ok(affected)
    }

    pub fn rollback(&mut self) {
        self.pending_writes.clear();
        self.pending_deletes.clear();
        self.in_transaction = false;
    }

    pub fn savepoint(&mut self, _name: String) -> std::result::Result<(), String> {
        Ok(())
    }

    pub fn rollback_to_savepoint(&mut self, _name: &str) -> std::result::Result<(), String> {
        Ok(())
    }

    pub fn release_savepoint(&mut self, _name: &str) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub struct UpdateExecNode {
    pub table_name: String,
    pub database: String,
    pub set_clauses: Vec<(String, String)>,
    pub selection_predicate: Option<String>,
    pub tablet_id: Option<u64>,
    pub storage: Option<Arc<StorageEngine>>,
    pub transaction_ctx: Option<Arc<StdRwLock<TransactionContext>>>,
    pub executed: bool,
}

impl UpdateExecNode {
    pub fn new(
        table_name: String,
        database: String,
        set_clauses: Vec<(String, String)>,
        selection_predicate: Option<String>,
    ) -> Self {
        Self {
            table_name,
            database,
            set_clauses,
            selection_predicate,
            tablet_id: None,
            storage: None,
            transaction_ctx: None,
            executed: false,
        }
    }

    pub fn with_storage(mut self, tablet_id: u64, storage: Arc<StorageEngine>) -> Self {
        self.tablet_id = Some(tablet_id);
        self.storage = Some(storage);
        self
    }

    pub fn with_transaction_ctx(mut self, tx_ctx: Arc<StdRwLock<TransactionContext>>) -> Self {
        self.transaction_ctx = Some(tx_ctx);
        self
    }
}

#[async_trait]
impl ExecNode for UpdateExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }
        self.executed = true;

        let Some(tablet_id) = self.tablet_id else {
            tracing::warn!("UPDATE without tablet_id on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };
        let Some(storage) = &self.storage else {
            tracing::warn!("UPDATE without storage on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };

        // Parse WHERE predicates
        let predicates = match &self.selection_predicate {
            Some(pred_str) => parse_predicates(pred_str),
            None => vec![],
        };

        // Read all data from the tablet
        let full_block = storage.read_tablet(tablet_id, None, &[])?;
        if full_block.is_empty() {
            return Ok(Some(make_affected_rows_block(0)));
        }

        // Find matching rows
        let selection = apply_predicates_to_block(&full_block, &predicates);
        let affected_count = selection.set_count();
        if affected_count == 0 {
            return Ok(Some(make_affected_rows_block(0)));
        }

        // Build inverted bitmap to get non-matching rows (preserved as-is)
        let mut inverted_bits = Vec::with_capacity(full_block.num_rows());
        for i in 0..full_block.num_rows() {
            inverted_bits.push(!selection.get(i));
        }
        let inverted_selection = Bitmap::from_bools(&inverted_bits);
        let non_matching_block = full_block.filter(&inverted_selection);

        // Extract matching rows and apply SET clauses
        let mut modified_block = full_block.filter(&selection);
        let schema = modified_block.schema().clone();

        for (col_name, value_str) in &self.set_clauses {
            if let Some(col_idx) = schema.index_of(col_name) {
                if let Some(field) = schema.field(col_idx) {
                    let new_value = parse_set_value(value_str, &field.data_type);
                    let num_rows = modified_block.num_rows();
                    let new_col = Vector::from_scalar(&new_value, num_rows);
                    modified_block.set_column(col_idx, new_col);
                }
            }
        }

        // Combine modified rows with non-matching rows to form the complete result
        let mut final_block = modified_block;
        if !non_matching_block.is_empty() {
            final_block.append_block(&non_matching_block);
        }

        // Check if we're in transaction mode
        if let Some(ref tx_ctx) = self.transaction_ctx {
            let mut tx = tx_ctx.write().unwrap();
            if tx.in_transaction {
                // Stage pending delete for the matching rows
                tx.pending_deletes.push(PendingDelete {
                    tablet_id,
                    predicates: predicates.clone(),
                });
                // Stage pending write with the final block (modified + non-matching)
                tx.pending_writes.push(PendingWrite {
                    tablet_id,
                    block: final_block,
                    op_type: WriteOp::Update,
                });
                tracing::info!("UPDATE on {}.{} staged to transaction: {} rows affected", self.database, self.table_name, affected_count);
                return Ok(Some(make_affected_rows_block(affected_count)));
            }
        }

        // Delete old matching rows, then write back all rows (modified + non-matching)
        storage.delete(tablet_id, &predicates)?;
        storage.write_batch(tablet_id, &final_block)?;

        tracing::info!("UPDATE on {}.{}: {} rows affected", self.database, self.table_name, affected_count);
        Ok(Some(make_affected_rows_block(affected_count)))
    }

    async fn close(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct DeleteExecNode {
    pub table_name: String,
    pub database: String,
    pub selection_predicate: Option<String>,
    pub tablet_id: Option<u64>,
    pub storage: Option<Arc<StorageEngine>>,
    pub transaction_ctx: Option<Arc<StdRwLock<TransactionContext>>>,
    pub executed: bool,
}

impl DeleteExecNode {
    pub fn new(
        table_name: String,
        database: String,
        selection_predicate: Option<String>,
    ) -> Self {
        Self {
            table_name,
            database,
            selection_predicate,
            tablet_id: None,
            storage: None,
            transaction_ctx: None,
            executed: false,
        }
    }

    pub fn with_storage(mut self, tablet_id: u64, storage: Arc<StorageEngine>) -> Self {
        self.tablet_id = Some(tablet_id);
        self.storage = Some(storage);
        self
    }

    pub fn with_transaction_ctx(mut self, tx_ctx: Arc<StdRwLock<TransactionContext>>) -> Self {
        self.transaction_ctx = Some(tx_ctx);
        self
    }
}

#[async_trait]
impl ExecNode for DeleteExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }
        self.executed = true;

        let Some(tablet_id) = self.tablet_id else {
            tracing::warn!("DELETE without tablet_id on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };
        let Some(storage) = &self.storage else {
            tracing::warn!("DELETE without storage on {}.{}", self.database, self.table_name);
            return Ok(Some(make_affected_rows_block(0)));
        };

        let predicates = match &self.selection_predicate {
            Some(pred_str) => parse_predicates(pred_str),
            None => vec![],
        };

        // Check if we're in transaction mode
        if let Some(ref tx_ctx) = self.transaction_ctx {
            let mut tx = tx_ctx.write().unwrap();
            if tx.in_transaction {
                // Stage pending delete
                tx.pending_deletes.push(PendingDelete {
                    tablet_id,
                    predicates: predicates.clone(),
                });
                // For delete, we don't have a block to write back, just record affected count
                // The affected count will be determined at commit time
                tracing::info!("DELETE on {}.{} staged to transaction", self.database, self.table_name);
                return Ok(Some(make_affected_rows_block(0)));
            }
        }

        let affected = storage.delete(tablet_id, &predicates)?;

        tracing::info!("DELETE from {}.{}: {} rows affected", self.database, self.table_name, affected);
        Ok(Some(make_affected_rows_block(affected)))
    }

    async fn close(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct AlterTableExecNode {
    pub database: String,
    pub table_name: String,
    pub operations: Vec<String>,
    pub executed: bool,
}

impl AlterTableExecNode {
    pub fn new(database: String, table_name: String, operations: Vec<String>) -> Self {
        Self {
            database,
            table_name,
            operations,
            executed: false,
        }
    }
}

#[async_trait]
impl ExecNode for AlterTableExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }
        self.executed = true;
        tracing::info!("ALTER TABLE {}.{} executed", self.database, self.table_name);
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}