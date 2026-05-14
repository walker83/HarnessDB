use mysql::prelude::*;
use mysql::{Pool};

fn main() {
    // stmt_cache_size=0 禁用 prepared statement
    println!("Test: Pool with stmt_cache_size=0 (no prepared statements)");
    let url = "mysql://root@127.0.0.1:9030?stmt_cache_size=0";
    match Pool::new(url) {
        Ok(pool) => {
            println!("Pool created OK");
            match pool.get_conn() {
                Ok(mut conn) => {
                    println!("get_conn() OK");
                    
                    // Use ltc database
                    conn.query_drop("USE ltc").unwrap();
                    
                    // Test simple query
                    let result: Option<u64> = conn.query_first("SELECT COUNT(*) FROM token_records").ok().flatten();
                    println!("COUNT(*) result: {:?}", result);
                    
                    // Test exec without prepare
                    println!("Testing exec...");
                    let result: Vec<u64> = conn.exec("SELECT COUNT(*) FROM token_records", ()).unwrap_or_default();
                    println!("Exec result: {:?}", result);
                    
                    // Test with WHERE condition
                    println!("Testing exec with params...");
                    let result: Vec<u64> = conn.exec("SELECT COUNT(*) FROM token_records WHERE timestamp > ?", (0i64,)).unwrap_or_default();
                    println!("Exec with params result: {:?}", result);
                }
                Err(e) => println!("get_conn() error: {:?}", e),
            }
        }
        Err(e) => println!("Pool::new error: {:?}", e),
    }
}
