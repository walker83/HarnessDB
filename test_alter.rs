use fe_sql_parser::parse_sql;

fn main() {
    let sql = "ALTER TABLE employees ADD COLUMN age INT64";
    println!("Parsing SQL: {}", sql);
    
    match parse_sql(sql) {
        Ok(statements) => {
            println!("Parsed successfully!");
            for stmt in statements {
                println!("Statement: {:?}", stmt);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
