use std::any::Any;
use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType as ArrowDataType, Schema as ArrowSchema, SchemaRef};
use datafusion::common::config::ConfigOptions;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::LogicalPlan;
use datafusion::logical_expr::{AggregateUDF, ScalarUDF, WindowUDF};
use datafusion::sql::parser::{DFParser, Statement as DFStatement};
use datafusion::sql::planner::SqlToRel;
use thiserror::Error;

pub fn is_dml_sql(sql: &str) -> bool {
    let upper = sql.trim().to_uppercase();
    let first_word = upper.split_whitespace().next().unwrap_or("");
    matches!(first_word, "SELECT" | "WITH" | "INSERT" | "UPDATE" | "DELETE" | "VALUES" | "EXPLAIN")
}

#[derive(Debug, Error)]
pub enum DataFusionParseError {
    #[error("DataFusion error: {0}")]
    DataFusion(#[from] DataFusionError),

    #[error("SQL syntax error: {0}")]
    SyntaxError(String),

    #[error("Unsupported SQL statement: {0}")]
    Unsupported(String),
}

pub fn try_parse_dml_with_datafusion(sql: &str) -> Result<LogicalPlan, DataFusionParseError> {
    let mut parser = DFParser::new(sql)
        .map_err(|e| DataFusionParseError::SyntaxError(e.to_string()))?;
    let statements = parser.parse_statements()
        .map_err(|e| DataFusionParseError::SyntaxError(e.to_string()))?;

    if statements.is_empty() {
        return Err(DataFusionParseError::SyntaxError("Empty SQL statement".to_string()));
    }

    let stmt = statements.into_iter().next().unwrap();
    let context_provider = EmptyContextProvider::new();
    let planner = SqlToRel::new(&context_provider);

    match stmt {
        DFStatement::Statement(s) => {
            planner.sql_statement_to_plan(*s)
                .map_err(DataFusionParseError::DataFusion)
        }
        _ => Err(DataFusionParseError::Unsupported(format!("{:?}", stmt))),
    }
}

struct EmptyContextProvider {
    options: ConfigOptions,
}

impl EmptyContextProvider {
    fn new() -> Self {
        Self {
            options: ConfigOptions::new(),
        }
    }
}

impl datafusion::sql::planner::ContextProvider for EmptyContextProvider {
    fn get_table_source(
        &self,
        _name: datafusion::sql::TableReference,
    ) -> datafusion::common::Result<Arc<dyn datafusion::logical_expr::TableSource>> {
        Ok(Arc::new(EmptyTableSource {
            schema: Arc::new(ArrowSchema::empty()),
        }))
    }

    fn get_function_meta(&self, _name: &str) -> Option<Arc<ScalarUDF>> {
        None
    }

    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }

    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<ArrowDataType> {
        None
    }

    fn options(&self) -> &ConfigOptions {
        &self.options
    }

    fn udf_names(&self) -> Vec<String> {
        vec![]
    }

    fn udaf_names(&self) -> Vec<String> {
        vec![]
    }

    fn udwf_names(&self) -> Vec<String> {
        vec![]
    }
}

#[derive(Debug)]
struct EmptyTableSource {
    schema: SchemaRef,
}

impl datafusion::logical_expr::TableSource for EmptyTableSource {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let result = try_parse_dml_with_datafusion("SELECT 1 + 2 AS result");
        assert!(result.is_ok());
    }

    #[test]
    fn test_select_with_cte() {
        let result = try_parse_dml_with_datafusion(
            "WITH cte AS (SELECT 1 AS id) SELECT * FROM cte"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_select_subquery() {
        let result = try_parse_dml_with_datafusion(
            "SELECT * FROM (SELECT 1 AS id) AS sub WHERE id > 0"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_dml_sql() {
        assert!(is_dml_sql("SELECT 1"));
        assert!(is_dml_sql("  WITH cte AS (SELECT 1) SELECT * FROM cte"));
        assert!(is_dml_sql("INSERT INTO t VALUES (1)"));
        assert!(is_dml_sql("UPDATE t SET a = 1"));
        assert!(is_dml_sql("DELETE FROM t WHERE id = 1"));
        assert!(!is_dml_sql("CREATE TABLE t (id INT)"));
        assert!(!is_dml_sql("SHOW TABLES"));
        assert!(!is_dml_sql("ALTER TABLE t ADD COLUMN a INT"));
    }
}
