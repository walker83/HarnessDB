//! SQL Analyzer with semantic analysis and symbol table management
//!
//! This module provides deep semantic analysis including:
//! - Column resolution and type checking
//! - Scope management via SymbolTable
//! - Expression type inference

use crate::ast::{
    BinaryOp, DeleteStmt, Expr, InsertStmt, LiteralValue, OrderByItem, QueryStmt, SelectItem,
    Statement, TableRef, UnaryOp, UpdateStmt,
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Analysis errors for semantic analysis
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("column not found: {0}")]
    ColumnNotFound(String),

    #[error("table not found: {0}")]
    TableNotFound(String),

    #[error("type mismatch: {actual} vs {expected}")]
    TypeMismatch { actual: String, expected: String },

    #[error("ambiguous column reference: {0}")]
    AmbiguousColumn(String),

    #[error("invalid expression: {0}")]
    InvalidExpression(String),

    #[error("analysis not supported for statement type: {0}")]
    UnsupportedStatement(String),
}

/// Resolved column information
#[derive(Debug, Clone)]
pub struct ResolvedColumn {
    pub name: String,
    pub table: Option<String>,
    pub data_type: DataType,
    pub nullable: bool,
}

/// Data types for typed expressions
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Int64,
    Float64,
    Boolean,
    String,
    Date,
    Null,
    Unknown,
}

/// Typed expression with resolved types
#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub expr: Expr,
    pub data_type: DataType,
    pub nullable: bool,
}

/// Typed select item
#[derive(Debug, Clone)]
pub struct TypedSelectItem {
    pub select_item: SelectItem,
    pub data_type: DataType,
}

/// Typed query statement
#[derive(Debug, Clone)]
pub struct TypedQueryStmt {
    pub select_list: Vec<TypedSelectItem>,
    pub from: Option<ResolvedTableRef>,
    pub r#where: Option<TypedExpr>,
    pub group_by: Vec<TypedExpr>,
    pub having: Option<TypedExpr>,
    pub order_by: Vec<TypedOrderByItem>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Resolved table reference
#[derive(Debug, Clone)]
pub struct ResolvedTableRef {
    pub name: String,
    pub alias: Option<String>,
    pub columns: Vec<ResolvedColumn>,
}

/// Typed order by item
#[derive(Debug, Clone)]
pub struct TypedOrderByItem {
    pub expr: TypedExpr,
    pub ascending: bool,
    pub nulls_first: bool,
}

/// Typed statement result
#[derive(Debug, Clone)]
pub enum TypedStatement {
    Query(TypedQueryStmt),
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<TypedExpr>>,
        query: Option<TypedQueryStmt>,
    },
    Update(UpdateStmt), // TODO: type this fully
    Delete(DeleteStmt), // TODO: type this fully
    Other(Statement),
}

/// Symbol table for scope management
#[derive(Debug, Clone)]
pub struct SymbolTable {
    tables: HashMap<String, ResolvedTableRef>,
    /// Columns indexed by (table_alias, column_name) or (None, column_name)
    columns: HashMap<(Option<String>, String), ResolvedColumn>,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            columns: HashMap::new(),
        }
    }

    /// Register a table in the symbol table
    pub fn add_table(&mut self, table: ResolvedTableRef) {
        let table_name = table.name.clone();
        let alias = table.alias.clone();

        // Add by table name
        self.tables.insert(table_name.clone(), table.clone());

        // Also add by alias if present
        if let Some(alias_str) = alias {
            self.tables.insert(alias_str.clone(), table.clone());

            // Register columns with table alias prefix
            for col in &table.columns {
                self.columns
                    .insert((Some(alias_str.clone()), col.name.clone()), col.clone());
            }
        }

        // Register columns without table prefix (for unqualified references)
        for col in &table.columns {
            self.columns.insert((None, col.name.clone()), col.clone());
        }
    }

    /// Resolve a column reference
    pub fn resolve_column(
        &self,
        table_alias: &Option<String>,
        col_name: &str,
    ) -> Result<ResolvedColumn, AnalysisError> {
        // First try with table alias if provided
        if let Some(alias) = table_alias {
            if let Some(col) = self
                .columns
                .get(&(Some(alias.clone()), col_name.to_string()))
            {
                return Ok(col.clone());
            }
        }

        // Search all columns for this name
        let mut matches: Vec<&ResolvedColumn> = self
            .columns
            .values()
            .filter(|c| c.name == col_name)
            .collect();

        match matches.len() {
            0 => Err(AnalysisError::ColumnNotFound(col_name.to_string())),
            1 => Ok(matches[0].clone()),
            _ => Err(AnalysisError::AmbiguousColumn(col_name.to_string())),
        }
    }

    /// Get a table by name or alias
    pub fn get_table(&self, name: &str) -> Option<&ResolvedTableRef> {
        self.tables.get(name)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL Analyzer for semantic analysis
pub struct Analyzer {
    /// Optional catalog for table metadata resolution
    catalog: Option<Arc<dyn CatalogProvider>>,
}

impl Analyzer {
    /// Create a new analyzer without catalog (for standalone parsing)
    pub fn new() -> Self {
        Self { catalog: None }
    }

    /// Create an analyzer with a catalog provider
    pub fn with_catalog(catalog: Arc<dyn CatalogProvider>) -> Self {
        Self {
            catalog: Some(catalog),
        }
    }

    /// Analyze a statement and return a typed statement
    pub fn analyze(&self, stmt: &Statement) -> Result<TypedStatement, AnalysisError> {
        match stmt {
            Statement::Query(query) => self.analyze_query(query),
            Statement::Insert(insert) => self.analyze_insert(insert),
            Statement::Update(update) => Ok(TypedStatement::Update(update.clone())),
            Statement::Delete(delete) => Ok(TypedStatement::Delete(delete.clone())),
            other => Ok(TypedStatement::Other(other.clone())),
        }
    }

    /// Analyze a query statement
    pub fn analyze_query(&self, query: &QueryStmt) -> Result<TypedStatement, AnalysisError> {
        // Build symbol table from FROM clause
        let mut symbol_table = SymbolTable::new();
        if let Some(table_ref) = &query.from {
            self.resolve_table_ref(table_ref, &mut symbol_table)?;
        }

        // Analyze SELECT list
        let mut typed_select_list = Vec::new();
        for item in &query.select_list {
            let typed_item = self.analyze_select_item(item, &symbol_table)?;
            typed_select_list.push(typed_item);
        }

        // Analyze WHERE clause
        let typed_where = if let Some(where_expr) = &query.r#where {
            Some(self.analyze_expr(where_expr, &symbol_table)?)
        } else {
            None
        };

        // Analyze GROUP BY
        let typed_group_by = query
            .group_by
            .iter()
            .map(|expr| self.analyze_expr(expr, &symbol_table))
            .collect::<Result<Vec<_>, _>>()?;

        // Analyze HAVING
        let typed_having = if let Some(having_expr) = &query.having {
            Some(self.analyze_expr(having_expr, &symbol_table)?)
        } else {
            None
        };

        // Analyze ORDER BY
        let typed_order_by = query
            .order_by
            .iter()
            .map(|item| self.analyze_order_by_item(item, &symbol_table))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TypedStatement::Query(TypedQueryStmt {
            select_list: typed_select_list,
            from: symbol_table.tables.get(&"".to_string()).cloned(),
            r#where: typed_where,
            group_by: typed_group_by,
            having: typed_having,
            order_by: typed_order_by,
            limit: query.limit,
            offset: query.offset,
        }))
    }

    /// Analyze a select item
    fn analyze_select_item(
        &self,
        item: &SelectItem,
        symbol_table: &SymbolTable,
    ) -> Result<TypedSelectItem, AnalysisError> {
        let typed_expr = self.analyze_expr(&item.expr, symbol_table)?;
        Ok(TypedSelectItem {
            select_item: item.clone(),
            data_type: typed_expr.data_type,
        })
    }

    /// Analyze an order by item
    fn analyze_order_by_item(
        &self,
        item: &OrderByItem,
        symbol_table: &SymbolTable,
    ) -> Result<TypedOrderByItem, AnalysisError> {
        let typed_expr = self.analyze_expr(&item.expr, symbol_table)?;
        Ok(TypedOrderByItem {
            expr: typed_expr,
            ascending: item.ascending,
            nulls_first: item.nulls_first,
        })
    }

    /// Analyze an insert statement
    pub fn analyze_insert(&self, insert: &InsertStmt) -> Result<TypedStatement, AnalysisError> {
        let typed_values: Vec<Vec<TypedExpr>> = insert
            .values
            .iter()
            .map(|row| {
                row.iter()
                    .map(|expr| self.analyze_expr(expr, &SymbolTable::new()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        let typed_query = if let Some(query) = &insert.query {
            match self.analyze_query(query)? {
                TypedStatement::Query(q) => Some(q),
                _ => None,
            }
        } else {
            None
        };

        Ok(TypedStatement::Insert {
            table: insert.table.clone(),
            columns: insert.columns.clone(),
            values: typed_values,
            query: typed_query,
        })
    }

    /// Resolve a table reference and populate the symbol table
    fn resolve_table_ref(
        &self,
        table_ref: &TableRef,
        symbol_table: &mut SymbolTable,
    ) -> Result<(), AnalysisError> {
        match table_ref {
            TableRef::Table { name, alias } => {
                let resolved_table = self.resolve_table_name(name, alias.as_deref())?;
                symbol_table.add_table(resolved_table);
            }
            TableRef::Join { left, right, .. } => {
                self.resolve_table_ref(left, symbol_table)?;
                self.resolve_table_ref(right, symbol_table)?;
            }
            TableRef::Subquery { query, alias } => {
                let typed_query = match self.analyze_query(query)? {
                    TypedStatement::Query(q) => q,
                    _ => {
                        return Err(AnalysisError::InvalidExpression(
                            "Subquery must be a Query".to_string(),
                        ))
                    }
                };
                let columns: Vec<ResolvedColumn> = typed_query
                    .select_list
                    .iter()
                    .map(|item| ResolvedColumn {
                        name: item
                            .select_item
                            .alias
                            .clone()
                            .unwrap_or_else(|| format!("col_{}", item.data_type)),
                        table: Some(alias.clone()),
                        data_type: item.data_type.clone(),
                        nullable: true,
                    })
                    .collect();

                let table_ref = ResolvedTableRef {
                    name: alias.clone(),
                    alias: Some(alias.clone()),
                    columns,
                };
                symbol_table.add_table(table_ref);
            }
        }
        Ok(())
    }

    /// Resolve a table name to a table reference with columns
    fn resolve_table_name(
        &self,
        name: &str,
        alias: Option<&str>,
    ) -> Result<ResolvedTableRef, AnalysisError> {
        // Check catalog if available
        if let Some(catalog) = &self.catalog {
            if let Some(table_info) = catalog.get_table(name) {
                let columns: Vec<ResolvedColumn> = table_info
                    .columns
                    .iter()
                    .map(|col| ResolvedColumn {
                        name: col.name.clone(),
                        table: Some(name.to_string()),
                        data_type: col.data_type.clone(),
                        nullable: col.nullable,
                    })
                    .collect();

                return Ok(ResolvedTableRef {
                    name: name.to_string(),
                    alias: alias.map(String::from),
                    columns,
                });
            }
        }

        // If no catalog or table not found, create placeholder
        // This allows analysis to proceed for testing without full catalog
        Ok(ResolvedTableRef {
            name: name.to_string(),
            alias: alias.map(String::from),
            columns: vec![],
        })
    }

    /// Analyze an expression
    pub fn analyze_expr(
        &self,
        expr: &Expr,
        symbol_table: &SymbolTable,
    ) -> Result<TypedExpr, AnalysisError> {
        match expr {
            Expr::Literal(lit) => {
                let (data_type, nullable) = self.analyze_literal(lit);
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type,
                    nullable,
                })
            }
            Expr::ColumnRef { table, column } => {
                let resolved = symbol_table.resolve_column(table, column)?;
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: resolved.data_type,
                    nullable: resolved.nullable,
                })
            }
            Expr::BinaryOp { left, op, right } => {
                let left_typed = self.analyze_expr(left, symbol_table)?;
                let right_typed = self.analyze_expr(right, symbol_table)?;
                self.check_binary_op_type(op, &left_typed, &right_typed)?;

                // Determine result type
                let result_type =
                    self.binary_op_result_type(&left_typed.data_type, &right_typed.data_type);
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: result_type,
                    nullable: left_typed.nullable || right_typed.nullable,
                })
            }
            Expr::UnaryOp { op, expr: inner } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                self.check_unary_op_type(op, &inner_typed)?;

                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: inner_typed.data_type,
                    nullable: inner_typed.nullable,
                })
            }
            Expr::FunctionCall { name, args, .. } => {
                let typed_args: Vec<TypedExpr> = args
                    .iter()
                    .map(|arg| self.analyze_expr(arg, symbol_table))
                    .collect::<Result<Vec<_>, _>>()?;

                // Infer return type from function name and argument types
                let return_type = self.infer_function_return_type(name, &typed_args);
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: return_type,
                    nullable: true,
                })
            }
            Expr::Between {
                expr: inner,
                low,
                high,
                ..
            } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                let low_typed = self.analyze_expr(low, symbol_table)?;
                let high_typed = self.analyze_expr(high, symbol_table)?;

                // Check all operands are comparable
                self.check_comparable(&inner_typed)?;
                self.check_comparable(&low_typed)?;
                self.check_comparable(&high_typed)?;

                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: inner_typed.nullable || low_typed.nullable || high_typed.nullable,
                })
            }
            Expr::InList {
                expr: inner, list, ..
            } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                for item in list {
                    let item_typed = self.analyze_expr(item, symbol_table)?;
                    self.check_comparable(&inner_typed)?;
                }

                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: inner_typed.nullable,
                })
            }
            Expr::InSubquery {
                expr: inner, query, ..
            } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                self.analyze_query(query)?; // Validate subquery

                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: inner_typed.nullable,
                })
            }
            Expr::Exists(query) => {
                self.analyze_query(query)?; // Validate subquery
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: false,
                })
            }
            Expr::Subquery(query) => {
                let typed_query = self.analyze_query(query)?;
                // Use the first column of the subquery as the type
                let data_type = match typed_query {
                    TypedStatement::Query(q) => q.select_list
                        .first()
                        .map(|s| s.data_type.clone())
                        .unwrap_or(DataType::Null),
                    _ => DataType::Null,
                };
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type,
                    nullable: true,
                })
            }
            Expr::IsNull { expr: inner, .. } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: false,
                })
            }
            Expr::Like {
                expr: inner,
                pattern,
                ..
            } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                let _ = self.analyze_expr(pattern, symbol_table)?;
                self.check_string_compatible(&inner_typed)?;

                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: DataType::Boolean,
                    nullable: inner_typed.nullable,
                })
            }
            Expr::Cast {
                expr: inner,
                target_type,
            } => {
                let inner_typed = self.analyze_expr(inner, symbol_table)?;
                let target = self.parse_data_type(target_type)?;
                Ok(TypedExpr {
                    expr: expr.clone(),
                    data_type: target,
                    nullable: inner_typed.nullable,
                })
            }
            Expr::Wildcard => Ok(TypedExpr {
                expr: expr.clone(),
                data_type: DataType::Unknown,
                nullable: true,
            }),
            Expr::Default => Ok(TypedExpr {
                expr: expr.clone(),
                data_type: DataType::Null,
                nullable: true,
            }),
        }
    }

    /// Analyze a literal value
    fn analyze_literal(&self, lit: &LiteralValue) -> (DataType, bool) {
        match lit {
            LiteralValue::Null => (DataType::Null, true),
            LiteralValue::Boolean(_) => (DataType::Boolean, false),
            LiteralValue::Int64(_) => (DataType::Int64, false),
            LiteralValue::Float64(_) => (DataType::Float64, false),
            LiteralValue::String(_) => (DataType::String, false),
            LiteralValue::Date(_) => (DataType::Date, false),
        }
    }

    /// Check binary operator type compatibility
    fn check_binary_op_type(
        &self,
        op: &BinaryOp,
        left: &TypedExpr,
        right: &TypedExpr,
    ) -> Result<(), AnalysisError> {
        match op {
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                self.check_comparable(left)?;
                self.check_comparable(right)?;
            }
            BinaryOp::And | BinaryOp::Or => {
                if left.data_type != DataType::Boolean {
                    return Err(AnalysisError::TypeMismatch {
                        actual: format!("{:?}", left.data_type),
                        expected: "Boolean".to_string(),
                    });
                }
                if right.data_type != DataType::Boolean {
                    return Err(AnalysisError::TypeMismatch {
                        actual: format!("{:?}", right.data_type),
                        expected: "Boolean".to_string(),
                    });
                }
            }
            BinaryOp::Plus
            | BinaryOp::Minus
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo => {
                self.check_numeric(left)?;
                self.check_numeric(right)?;
            }
            BinaryOp::Like | BinaryOp::NotLike => {
                self.check_string_compatible(left)?;
                self.check_string_compatible(right)?;
            }
            BinaryOp::In | BinaryOp::NotIn => {
                // Already handled in expression analysis
            }
        }
        Ok(())
    }

    /// Check unary operator type compatibility
    fn check_unary_op_type(&self, op: &UnaryOp, expr: &TypedExpr) -> Result<(), AnalysisError> {
        match op {
            UnaryOp::Not => {
                if expr.data_type != DataType::Boolean {
                    return Err(AnalysisError::TypeMismatch {
                        actual: format!("{:?}", expr.data_type),
                        expected: "Boolean".to_string(),
                    });
                }
            }
            UnaryOp::Negate => {
                self.check_numeric(expr)?;
            }
        }
        Ok(())
    }

    /// Determine result type of binary operation
    fn binary_op_result_type(&self, left: &DataType, right: &DataType) -> DataType {
        match (left, right) {
            (DataType::Int64, DataType::Int64) => DataType::Int64,
            (DataType::Float64, _) | (_, DataType::Float64) => DataType::Float64,
            (DataType::String, DataType::String) => DataType::String,
            _ => DataType::Unknown,
        }
    }

    /// Check if expression is comparable (for ORDER BY, WHERE, etc.)
    fn check_comparable(&self, expr: &TypedExpr) -> Result<(), AnalysisError> {
        match expr.data_type {
            DataType::Null
            | DataType::Int64
            | DataType::Float64
            | DataType::String
            | DataType::Date => Ok(()),
            _ => Err(AnalysisError::TypeMismatch {
                actual: format!("{:?}", expr.data_type),
                expected: "comparable type".to_string(),
            }),
        }
    }

    /// Check if expression is numeric
    fn check_numeric(&self, expr: &TypedExpr) -> Result<(), AnalysisError> {
        match expr.data_type {
            DataType::Int64 | DataType::Float64 => Ok(()),
            _ => Err(AnalysisError::TypeMismatch {
                actual: format!("{:?}", expr.data_type),
                expected: "numeric type".to_string(),
            }),
        }
    }

    /// Check if expression is string compatible
    fn check_string_compatible(&self, expr: &TypedExpr) -> Result<(), AnalysisError> {
        if expr.data_type == DataType::String {
            Ok(())
        } else {
            Err(AnalysisError::TypeMismatch {
                actual: format!("{:?}", expr.data_type),
                expected: "String".to_string(),
            })
        }
    }

    /// Infer function return type
    fn infer_function_return_type(&self, name: &str, args: &[TypedExpr]) -> DataType {
        let name_lower = name.to_lowercase();

        // Aggregate functions
        if name_lower == "count" {
            return DataType::Int64;
        }
        if name_lower == "sum" {
            if let Some(first) = args.first() {
                match first.data_type {
                    DataType::Int64 | DataType::Float64 => return first.data_type.clone(),
                    _ => {}
                }
            }
            return DataType::Int64;
        }
        if name_lower == "avg" {
            return DataType::Float64;
        }
        if name_lower == "min" || name_lower == "max" {
            return args
                .first()
                .map(|a| a.data_type.clone())
                .unwrap_or(DataType::Null);
        }

        // Scalar functions - return based on argument type or default to Unknown
        if name_lower == "upper"
            || name_lower == "lower"
            || name_lower == "trim"
            || name_lower == "concat"
        {
            return DataType::String;
        }
        if name_lower == "length" || name_lower == "char_length" {
            return DataType::Int64;
        }
        if name_lower == "abs" {
            if let Some(first) = args.first() {
                return first.data_type.clone();
            }
            return DataType::Int64;
        }

        // Window functions typically return the same type as input
        if name_lower == "row_number" || name_lower == "rank" || name_lower == "dense_rank" {
            return DataType::Int64;
        }
        if name_lower == "lead" || name_lower == "lag" {
            return args
                .first()
                .map(|a| a.data_type.clone())
                .unwrap_or(DataType::Null);
        }

        DataType::Unknown
    }

    /// Parse a data type string
    fn parse_data_type(&self, type_str: &str) -> Result<DataType, AnalysisError> {
        let lower = type_str.to_lowercase();
        match lower.as_str() {
            "int" | "int64" | "bigint" | "tinyint" | "smallint" => Ok(DataType::Int64),
            "float" | "float64" | "double" | "decimal" => Ok(DataType::Float64),
            "boolean" | "bool" => Ok(DataType::Boolean),
            "varchar" | "string" | "text" | "char" => Ok(DataType::String),
            "date" | "datetime" | "timestamp" => Ok(DataType::Date),
            "null" => Ok(DataType::Null),
            _ => Ok(DataType::Unknown),
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for catalog provider to look up table metadata
pub trait CatalogProvider: Send + Sync {
    /// Get table information by name
    fn get_table(&self, name: &str) -> Option<TableInfo>;
}

/// Table information from catalog
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

/// Column information
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_sql;

    fn get_analyzer() -> Analyzer {
        Analyzer::new()
    }

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();

        table.add_table(ResolvedTableRef {
            name: "users".to_string(),
            alias: Some("u".to_string()),
            columns: vec![
                ResolvedColumn {
                    name: "id".to_string(),
                    table: Some("users".to_string()),
                    data_type: DataType::Int64,
                    nullable: false,
                },
                ResolvedColumn {
                    name: "name".to_string(),
                    table: Some("users".to_string()),
                    data_type: DataType::String,
                    nullable: true,
                },
            ],
        });

        // Resolve with alias
        let col = table.resolve_column(&Some("u".to_string()), "id").unwrap();
        assert_eq!(col.name, "id");

        // Resolve without alias (ambiguous if multiple tables have same column)
        let col = table.resolve_column(&None, "id").unwrap();
        assert_eq!(col.name, "id");
    }

    #[test]
    fn test_column_not_found() {
        let table = SymbolTable::new();
        let result = table.resolve_column(&None, "nonexistent");
        assert!(matches!(result, Err(AnalysisError::ColumnNotFound(_))));
    }

    #[test]
    fn test_analyze_literal() {
        let analyzer = get_analyzer();

        // Test integer literal
        let expr = Expr::Literal(LiteralValue::Int64(42));
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Int64);
        assert!(!typed.nullable);

        // Test string literal
        let expr = Expr::Literal(LiteralValue::String("hello".to_string()));
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::String);

        // Test null literal
        let expr = Expr::Literal(LiteralValue::Null);
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Null);
        assert!(typed.nullable);
    }

    #[test]
    fn test_analyze_binary_op() {
        let analyzer = get_analyzer();

        // 1 + 2
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Int64(1))),
            op: BinaryOp::Plus,
            right: Box::new(Expr::Literal(LiteralValue::Int64(2))),
        };
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Int64);

        // 1.5 + 2.5
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Float64(1.5))),
            op: BinaryOp::Plus,
            right: Box::new(Expr::Literal(LiteralValue::Float64(2.5))),
        };
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Float64);

        // int + float = float
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Int64(1))),
            op: BinaryOp::Plus,
            right: Box::new(Expr::Literal(LiteralValue::Float64(2.5))),
        };
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Float64);
    }

    #[test]
    fn test_analyze_column_ref() {
        let mut table = SymbolTable::new();
        table.add_table(ResolvedTableRef {
            name: "users".to_string(),
            alias: None,
            columns: vec![ResolvedColumn {
                name: "id".to_string(),
                table: Some("users".to_string()),
                data_type: DataType::Int64,
                nullable: false,
            }],
        });

        let analyzer = get_analyzer();
        let expr = Expr::ColumnRef {
            table: None,
            column: "id".to_string(),
        };
        let typed = analyzer.analyze_expr(&expr, &table).unwrap();
        assert_eq!(typed.data_type, DataType::Int64);
    }

    #[test]
    fn test_analyze_query() {
        let analyzer = get_analyzer();
        let sql = "SELECT id, name FROM users WHERE age > 18";
        let parsed = parse_sql(sql).unwrap();

        if let Statement::Query(query) = &parsed[0] {
            let result = analyzer.analyze_query(query);
            assert!(result.is_ok(), "Query analysis failed: {:?}", result);
        } else {
            panic!("Expected Query statement");
        }
    }

    #[test]
    fn test_analyze_insert() {
        let analyzer = get_analyzer();
        let sql = "INSERT INTO users (id, name) VALUES (1, 'test')";
        let parsed = parse_sql(sql).unwrap();

        let result = analyzer.analyze(&parsed[0]);
        assert!(result.is_ok(), "Insert analysis failed: {:?}", result);
    }

    #[test]
    fn test_type_mismatch_detection() {
        let analyzer = get_analyzer();

        // Boolean AND with non-boolean should fail
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Int64(1))),
            op: BinaryOp::And,
            right: Box::new(Expr::Literal(LiteralValue::Int64(2))),
        };
        let result = analyzer.analyze_expr(&expr, &SymbolTable::new());
        assert!(matches!(result, Err(AnalysisError::TypeMismatch { .. })));
    }

    #[test]
    fn test_function_return_type_inference() {
        let analyzer = get_analyzer();

        // COUNT returns Int64
        let expr = Expr::FunctionCall {
            name: "COUNT".to_string(),
            args: vec![Expr::Wildcard],
            distinct: false,
        };
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::Int64);

        // UPPER returns String
        let expr = Expr::FunctionCall {
            name: "UPPER".to_string(),
            args: vec![Expr::Literal(LiteralValue::String("abc".to_string()))],
            distinct: false,
        };
        let typed = analyzer.analyze_expr(&expr, &SymbolTable::new()).unwrap();
        assert_eq!(typed.data_type, DataType::String);
    }

    #[test]
    fn test_analyze_with_cte() {
        let analyzer = get_analyzer();
        let sql = "WITH cte AS (SELECT id FROM t1) SELECT * FROM cte";
        let parsed = parse_sql(sql).unwrap();

        let result = analyzer.analyze(&parsed[0]);
        assert!(result.is_ok(), "CTE analysis failed: {:?}", result);
    }

    #[test]
    fn test_analyze_window_function() {
        let analyzer = get_analyzer();
        let sql = "SELECT id, ROW_NUMBER() OVER (ORDER BY id) FROM t";
        let parsed = parse_sql(sql).unwrap();

        let result = analyzer.analyze(&parsed[0]);
        assert!(
            result.is_ok(),
            "Window function analysis failed: {:?}",
            result
        );
    }
}
