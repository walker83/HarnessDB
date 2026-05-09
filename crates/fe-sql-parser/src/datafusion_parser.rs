//! DataFusion-based SQL parser integration for RorisDB.
//!
//! This module provides a wrapper around DataFusion's SQL parser (DFParser)
//! to enable modern SQL parsing with semantic analysis capabilities.

use std::sync::Arc;

use datafusion::common::DFSchema;
use datafusion::error::DataFusionError;
use datafusion::sql::parser::DFParser;
use datafusion::sql::planner::{ContextProvider, SqlToRel};
use thiserror::Error;

// Re-export Statement from our existing AST for compatibility
pub use crate::ast::Statement;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("DataFusion parsing error: {0}")]
    DataFusion(#[from] DataFusionError),

    #[error("SQL syntax error at line {line}, column {col}: {message}")]
    SyntaxError {
        line: usize,
        col: usize,
        message: String,
    },

    #[error("Unsupported SQL statement type: {0}")]
    Unsupported(String),
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// RorisParser wraps DataFusion's DFParser to provide SQL parsing for RorisDB.
/// It integrates with RorisDB's catalog and type system.
pub struct RorisParser {
    schema_provider: Option<Arc<dyn RorisSchemaProvider>>,
}

impl RorisParser {
    /// Create a new RorisParser with an optional schema provider
    pub fn new(schema_provider: Option<Arc<dyn RorisSchemaProvider>>) -> Self {
        Self { schema_provider }
    }

    /// Parse a SQL string into a DataFusion LogicalPlan
    pub fn parse(&self, sql: &str) -> ParseResult<datafusion::logical_plan::LogicalPlan> {
        let statements = DFParser::parse_sql(sql)
            .map_err(|e| ParseError::SyntaxError {
                line: e.line(),
                col: e.column(),
                message: e.message().to_string(),
            })?;

        if statements.is_empty() {
            return Err(ParseError::SyntaxError {
                line: 0,
                col: 0,
                message: "Empty SQL statement".to_string(),
            });
        }

        // For now, we only support a single statement
        let stmt = &statements[0];

        // Create a context provider for SQL-to-rel planning
        let provider = RorisContextProvider {
            schema_provider: self.schema_provider.clone(),
        };

        let planner = SqlToRel::new(&provider);

        planner.statement_to_plan(stmt.clone())
            .map_err(ParseError::DataFusion)
    }

    /// Parse multiple SQL statements
    pub fn parse_batch(&self, sql: &str) -> ParseResult<Vec<datafusion::logical_plan::LogicalPlan>> {
        let statements = DFParser::parse_sql(sql)
            .map_err(|e| ParseError::SyntaxError {
                line: e.line(),
                col: e.column(),
                message: e.message().to_string(),
            })?;

        let provider = RorisContextProvider {
            schema_provider: self.schema_provider.clone(),
        };

        let planner = SqlToRel::new(&provider);

        let mut plans = Vec::new();
        for stmt in statements {
            let plan = planner.statement_to_plan(stmt)
                .map_err(ParseError::DataFusion)?;
            plans.push(plan);
        }

        Ok(plans)
    }
}

/// Context provider for SQL-to-Rel conversion.
/// This provides table metadata during query planning.
pub trait RorisSchemaProvider: Send + Sync {
    /// Get the schema for a table
    fn get_table_schema(&self, name: &str) -> Option<Arc<DFSchema>>;

    /// List all available tables
    fn list_tables(&self) -> Vec<String>;
}

/// Context provider implementation for RorisDB
pub struct RorisContextProvider {
    schema_provider: Option<Arc<dyn RorisSchemaProvider>>,
}

impl ContextProvider for RorisContextProvider {
    fn get_table_provider(
        &self,
        name: datafusion::sql::TableReference,
    ) -> Option<Arc<dyn datafusion::catalog::TableProvider>> {
        let table_name = name.table();
        self.schema_provider
            .as_ref()
            .and_then(|p| p.get_table_schema(table_name))
            .map(|_| Arc::new(RorisTableAdapter {}) as Arc<dyn datafusion::catalog::TableProvider>)
    }

    fn get_function_meta(&self, _name: &str) -> Option<Arc<datafusion::arrow::datatypes::Schema>> {
        None
    }

    fn getAggregateMeta(&self, _name: &str) -> Option<Arc<datafusion::arrow::datatypes::Schema>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<datafusion::arrow::datatypes::DataType> {
        None
    }
}

/// Adapter to bridge RorisTableProvider to DataFusion TableProvider
pub struct RorisTableAdapter;

impl datafusion::catalog::TableProvider for RorisTableAdapter {
    fn schema(&self) -> Arc<datafusion::arrow::datatypes::Schema> {
        Arc::new(datafusion::arrow::datatypes::Schema::empty())
    }

    fn scan(
        &self,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::logical_plan::Expression],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn datafusion::catalog::TableProvider>, DataFusionError> {
        Ok(Arc::new(RorisTableAdapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let parser = RorisParser::new(None);
        let sql = "SELECT 1 + 2 AS result";
        let result = parser.parse(sql);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.to_string(), "Projection: Int64(1) + Int64(2) AS result\n  EmptyRelation");
    }

    #[test]
    fn test_select_from_table() {
        let parser = RorisParser::new(None);
        let sql = "SELECT id, name FROM users WHERE age > 18";
        let result = parser.parse(sql);
        assert!(result.is_ok());
    }
}