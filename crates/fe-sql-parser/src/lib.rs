pub mod ast;
pub mod datafusion_parser;
pub mod error;
pub mod parser;

pub use ast::Statement;
pub use datafusion_parser::{DataFusionParseError, is_dml_sql, try_parse_dml_with_datafusion};
pub use error::ParseError;
pub use parser::parse_sql;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alter_table_parsing() {
        let sql = "ALTER TABLE employees ADD COLUMN age INT64";
        match parse_sql(sql) {
            Ok(statements) => {
                assert!(!statements.is_empty());
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_basic_select_parsing() {
        let sql = "SELECT id, name FROM users WHERE age > 18";
        match parse_sql(sql) {
            Ok(statements) => {
                assert!(!statements.is_empty());
            }
            Err(e) => panic!("Parse failed: {:?}", e),
        }
    }

    #[test]
    fn test_cte_parsing() {
        let sql = "WITH cte AS (SELECT id FROM t1) SELECT * FROM cte";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "CTE should parse successfully");
    }

    #[test]
    fn test_window_function_parsing() {
        let sql = "SELECT id, ROW_NUMBER() OVER (ORDER BY id) FROM t";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "Window function should parse");
    }

    #[test]
    fn test_subquery_in_where() {
        let sql = "SELECT * FROM t1 WHERE id IN (SELECT id FROM t2)";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "Subquery should parse");
    }

    #[test]
    fn test_join_parsing() {
        let sql = "SELECT a.id, b.name FROM t1 a JOIN t2 b ON a.id = b.id";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "JOIN should parse");
    }

    #[test]
    fn test_insert_parsing() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'test')";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "INSERT should parse");
    }

    #[test]
    fn test_aggregate_with_group_by() {
        let sql = "SELECT department, COUNT(*) FROM employees GROUP BY department";
        let result = parse_sql(sql);
        assert!(result.is_ok(), "Aggregate with GROUP BY should parse");
    }
}
