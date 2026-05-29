use mysql::prelude::*;
use mysql::{Conn, Opts, OptsBuilder, Row, Value};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

// ===========================================================================
// Test Configuration
// ===========================================================================
const MYSQL_PORT: u16 = 19930;

// ===========================================================================
// Server lifecycle management
// ===========================================================================

struct E2eServer {
    child: Child,
    meta_dir: String,
    data_dir: String,
}

impl E2eServer {
    fn start() -> Self {
        let pid = std::process::id();
        let meta_dir = format!("/tmp/roris_e2e_meta_{}", pid);
        let data_dir = format!("/tmp/roris_e2e_data_{}", pid);

        let _ = std::fs::remove_dir_all(&meta_dir);
        let _ = std::fs::remove_dir_all(&data_dir);

        std::fs::create_dir_all(&meta_dir).expect("Failed to create meta directory");
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

        let binary = find_binary();
        let child = Command::new(&binary)
            .arg("--mysql-port")
            .arg(MYSQL_PORT.to_string())
            .arg("--meta-dir")
            .arg(&meta_dir)
            .arg("--data-dir")
            .arg(&data_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to start roris-fe binary '{}': {}", binary, e));

        E2eServer {
            child,
            meta_dir,
            data_dir,
        }
    }

    fn wait_ready(&self) {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(30);
        loop {
            if start.elapsed() > timeout {
                panic!("Server did not become ready within {:?}", timeout);
            }
            if std::net::TcpStream::connect(format!("127.0.0.1:{}", MYSQL_PORT)).is_ok() {
                thread::sleep(Duration::from_millis(1000));
                return;
            }
            thread::sleep(Duration::from_millis(300));
        }
    }
}

impl Drop for E2eServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.meta_dir);
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

fn find_binary() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{}/../../target/release/roris-fe", manifest_dir),
        format!("{}/../../target/debug/roris-fe", manifest_dir),
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return path.to_string();
        }
    }

    panic!(
        "roris-fe binary not found. Build with: cargo build --release\n\
         Expected at one of:\n  {}",
        candidates.join("\n  ")
    );
}

// ===========================================================================
// Query helpers
// ===========================================================================

fn make_conn() -> Conn {
    let opts = OptsBuilder::new()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(MYSQL_PORT)
        .user(Some("root"))
        .pass(None::<String>);
    Conn::new(Opts::from(opts)).expect("Failed to create connection")
}

fn exec_sql(conn: &mut Conn, sql: &str) {
    conn.query_drop(sql)
        .unwrap_or_else(|e| panic!("Query failed: '{}' -- {}", sql, e));
}

fn query_rows(conn: &mut Conn, sql: &str) -> Vec<Row> {
    conn.query(sql)
        .unwrap_or_else(|e| panic!("Query failed: '{}' -- {}", sql, e))
}

fn get_i64(row: &Row, idx: usize) -> i64 {
    match &row[idx] {
        Value::Int(n) => *n,
        Value::UInt(n) => *n as i64,
        Value::Bytes(b) => {
            let s = String::from_utf8_lossy(b);
            s.parse::<i64>()
                .unwrap_or_else(|e| panic!("Cannot parse Bytes({:?}) as i64: {}", s, e))
        }
        v => panic!("Expected integer at column {}, got {:?}", idx, v),
    }
}

fn get_f64(row: &Row, idx: usize) -> f64 {
    match &row[idx] {
        Value::Float(f) => *f as f64,
        Value::Double(d) => *d,
        Value::Int(n) => *n as f64,
        Value::Bytes(b) => {
            let s = String::from_utf8_lossy(b);
            s.parse::<f64>()
                .unwrap_or_else(|e| panic!("Cannot parse Bytes({:?}) as f64: {}", s, e))
        }
        v => panic!("Expected float at column {}, got {:?}", idx, v),
    }
}

fn get_string(row: &Row, idx: usize) -> String {
    match &row[idx] {
        Value::Bytes(b) => String::from_utf8_lossy(b).to_string(),
        Value::NULL => String::new(),
        v => format!("{:?}", v),
    }
}

// ===========================================================================
// The E2E test
// ===========================================================================

#[test]
fn test_doris_compat_e2e() {
    let server = E2eServer::start();
    server.wait_ready();

    let mut conn = make_conn();

    // a. CREATE DATABASE
    exec_sql(&mut conn, "CREATE DATABASE test_e2e");

    // b. USE database
    exec_sql(&mut conn, "USE test_e2e");

    // c. CREATE TABLE with Doris syntax
    exec_sql(
        &mut conn,
        "CREATE TABLE users (
            id INT,
            name VARCHAR(100),
            age INT,
            salary DOUBLE
        ) DUPLICATE KEY(id)
        DISTRIBUTED BY HASH(id) BUCKETS 1",
    );

    // d. INSERT single row
    exec_sql(
        &mut conn,
        "INSERT INTO users VALUES (1, 'Alice', 30, 50000.0)",
    );

    // e. INSERT multiple rows
    exec_sql(
        &mut conn,
        "INSERT INTO users VALUES (2, 'Bob', 25, 45000.0), (3, 'Charlie', 35, 60000.0)",
    );

    // f. SELECT with WHERE
    let rows = query_rows(&mut conn, "SELECT * FROM users WHERE age > 28");
    assert_eq!(rows.len(), 2, "WHERE age > 28 should return 2 rows");

    // g. SELECT with ORDER BY
    let rows = query_rows(
        &mut conn,
        "SELECT name, salary FROM users ORDER BY salary DESC",
    );
    assert_eq!(rows.len(), 3);
    assert_eq!(get_string(&rows[0], 0), "Charlie");
    assert_eq!(get_string(&rows[1], 0), "Alice");
    assert_eq!(get_string(&rows[2], 0), "Bob");

    // h. Aggregation
    let rows = query_rows(&mut conn, "SELECT COUNT(*), AVG(salary) FROM users");
    assert_eq!(rows.len(), 1);
    assert_eq!(get_i64(&rows[0], 0), 3);
    let avg = get_f64(&rows[0], 1);
    assert!(
        (avg - 51666.67).abs() < 100.0,
        "AVG should be ~51666, got {}",
        avg
    );

    // i. UPDATE
    exec_sql(
        &mut conn,
        "UPDATE users SET salary = 55000.0 WHERE name = 'Alice'",
    );

    // j. Verify UPDATE
    let rows = query_rows(&mut conn, "SELECT salary FROM users WHERE name = 'Alice'");
    assert_eq!(rows.len(), 1);
    assert!((get_f64(&rows[0], 0) - 55000.0).abs() < 0.01);

    // k. DELETE
    exec_sql(&mut conn, "DELETE FROM users WHERE age < 30");

    // l. Verify DELETE
    let rows = query_rows(&mut conn, "SELECT COUNT(*) FROM users");
    assert_eq!(get_i64(&rows[0], 0), 2);

    // m. DROP TABLE
    exec_sql(&mut conn, "DROP TABLE users");

    // n. DROP DATABASE
    exec_sql(&mut conn, "DROP DATABASE test_e2e");
}
