use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = MySqlDialect {};
    let sql = "ALTER TABLE employees ADD COLUMN age INT64";
    
    match Parser::parse_sql(&dialect, sql) {
        Ok(statements) => {
            println!("Parsed successfully!");
            for (i, stmt) in statements.iter().enumerate() {
                println!("Statement {}: {:?}", i, stmt);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
