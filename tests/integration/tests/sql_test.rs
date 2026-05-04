use std::sync::Arc;

use fe_catalog::table::TableColumn;
use fe_catalog::CatalogManager;
use fe_sql_planner::{PlanNodeType, Planner};

use integration_tests::common;

// ===========================================================================
// CREATE DATABASE tests
// ===========================================================================

#[test]
fn test_create_database() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("mydb").unwrap();

    let dbs = catalog.list_databases();
    assert!(dbs.contains(&"mydb".to_string()));
}

#[test]
fn test_create_database_duplicate() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("mydb").unwrap();
    let result = catalog.create_database("mydb");
    assert!(result.is_err());
}

#[test]
fn test_create_database_via_planner() {
    let catalog = Arc::new(CatalogManager::new());
    let _planner = Planner::new(catalog.clone());

    // Note: The current parser doesn't fully support CREATE DATABASE yet,
    // so we test the catalog directly
    catalog.create_database("test_planner_db").unwrap();
    assert!(catalog.get_database("test_planner_db").is_some());
}

// ===========================================================================
// DROP DATABASE tests
// ===========================================================================

#[test]
fn test_drop_database() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("temp_db").unwrap();
    catalog.drop_database("temp_db").unwrap();
    assert!(catalog.get_database("temp_db").is_none());
}

#[test]
fn test_drop_database_nonexistent() {
    let catalog = Arc::new(CatalogManager::new());
    let result = catalog.drop_database("nonexistent");
    assert!(result.is_err());
}

// ===========================================================================
// CREATE TABLE with various column types
// ===========================================================================

#[test]
fn test_create_table_with_types() {
    let catalog = common::create_test_catalog();
    let db = catalog.get_database("test_db").unwrap();
    assert!(db.get_table("employees").is_some());

    let table = db.get_table("employees").unwrap();
    assert_eq!(table.columns.len(), 4);
    assert_eq!(table.columns[0].name, "id");
    assert_eq!(table.columns[0].data_type, types::DataType::Int64);
    assert_eq!(table.columns[1].name, "name");
    assert_eq!(table.columns[1].data_type, types::DataType::String);
    assert_eq!(table.columns[2].name, "department");
    assert_eq!(table.columns[3].name, "salary");
    assert_eq!(table.columns[3].data_type, types::DataType::Float64);
}

#[test]
fn test_create_table_multiple_types() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("typed_db").unwrap();

    let table = fe_catalog::Table {
        id: 100,
        name: "all_types".to_string(),
        database: "typed_db".to_string(),
        columns: vec![
            TableColumn {
                name: "col_bool".into(),
                data_type: types::DataType::Boolean,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_int8".into(),
                data_type: types::DataType::Int8,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_int32".into(),
                data_type: types::DataType::Int32,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_int64".into(),
                data_type: types::DataType::Int64,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_float32".into(),
                data_type: types::DataType::Float32,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_float64".into(),
                data_type: types::DataType::Float64,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_string".into(),
                data_type: types::DataType::String,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_date".into(),
                data_type: types::DataType::Date,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "col_datetime".into(),
                data_type: types::DataType::DateTime,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
        ],
        keys_type: fe_catalog::table::KeysType::Duplicate,
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: std::collections::HashMap::new(),
        row_count: 0,
        data_size: 0,
    };

    catalog.create_table("typed_db", table).unwrap();

    let db = catalog.get_database("typed_db").unwrap();
    let t = db.get_table("all_types").unwrap();
    assert_eq!(t.columns.len(), 9);
    assert_eq!(t.columns[0].data_type, types::DataType::Boolean);
    assert_eq!(t.columns[3].data_type, types::DataType::Int64);
    assert_eq!(t.columns[8].data_type, types::DataType::DateTime);
}

// ===========================================================================
// INSERT and SELECT tests (via Block operations)
// ===========================================================================

#[test]
fn test_insert_and_select_block() {
    let block = common::create_employees_block();
    assert_eq!(block.num_rows(), 5);

    // SELECT * FROM employees
    let row = block.row(0);
    assert_eq!(row[0], types::ScalarValue::Int64(1));
    assert_eq!(row[1], types::ScalarValue::String("Alice".to_string()));
}

#[test]
fn test_select_with_filter() {
    let block = common::create_employees_block();

    // SELECT * FROM employees WHERE id > 2
    let id_col = block.column(0).unwrap();
    let mut selection = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let val = id_col.scalar_at(i);
        let pass = matches!(val, types::ScalarValue::Int64(v) if v > 2);
        selection.push(pass);
    }

    let filtered = block.filter(&selection);
    assert_eq!(filtered.num_rows(), 3);

    // Verify the first filtered row is Charlie (id=3)
    let row = filtered.row(0);
    assert_eq!(row[0], types::ScalarValue::Int64(3));
}

// ===========================================================================
// WHERE clause tests
// ===========================================================================

#[test]
fn test_where_salary_gt() {
    let block = common::create_employees_block();

    // SELECT * FROM employees WHERE salary > 80000
    let salary_col = block.column_by_name("salary").unwrap().1;
    let mut selection = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let val = salary_col.scalar_at(i);
        let pass = matches!(val, types::ScalarValue::Float64(v) if v > 80000.0);
        selection.push(pass);
    }

    let filtered = block.filter(&selection);
    assert_eq!(filtered.num_rows(), 3); // Alice 95k, Charlie 110k, Diana 82k
}

#[test]
fn test_where_department_eq() {
    let block = common::create_employees_block();

    // SELECT * FROM employees WHERE department = 'Engineering'
    let dept_col = block.column_by_name("department").unwrap().1;
    let mut selection = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let val = dept_col.scalar_at(i);
        let pass = matches!(val, types::ScalarValue::String(ref s) if s == "Engineering");
        selection.push(pass);
    }

    let filtered = block.filter(&selection);
    assert_eq!(filtered.num_rows(), 2); // Alice and Charlie
}

// ===========================================================================
// GROUP BY tests
// ===========================================================================

#[test]
fn test_group_by_department_count() {
    let block = common::create_employees_block();

    // Simulate GROUP BY department, COUNT(*)
    let dept_col = block.column_by_name("department").unwrap().1;
    let mut groups: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for i in 0..block.num_rows() {
        let dept = dept_col.scalar_at(i);
        if let types::ScalarValue::String(d) = dept {
            *groups.entry(d).or_insert(0) += 1;
        }
    }

    assert_eq!(*groups.get("Engineering").unwrap(), 2);
    assert_eq!(*groups.get("Marketing").unwrap(), 2);
    assert_eq!(*groups.get("Sales").unwrap(), 1);
}

// ===========================================================================
// ORDER BY tests
// ===========================================================================

#[test]
fn test_order_by_salary_desc() {
    let block = common::create_employees_block();

    // Simulate ORDER BY salary DESC
    let salary_col = block.column_by_name("salary").unwrap().1;
    let mut indices: Vec<usize> = (0..block.num_rows()).collect();

    indices.sort_by(|&a, &b| {
        let sa = salary_col.scalar_at(a);
        let sb = salary_col.scalar_at(b);
        match (sa, sb) {
            (types::ScalarValue::Float64(va), types::ScalarValue::Float64(vb)) => {
                vb.partial_cmp(&va).unwrap()
            }
            _ => std::cmp::Ordering::Equal,
        }
    });

    // First should be Charlie (110k)
    let row = block.row(indices[0]);
    assert_eq!(row[0], types::ScalarValue::Int64(3)); // Charlie

    // Last should be Eve (68k)
    let row = block.row(indices[4]);
    assert_eq!(row[0], types::ScalarValue::Int64(5)); // Eve
}

// ===========================================================================
// LIMIT tests
// ===========================================================================

#[test]
fn test_limit() {
    let block = common::create_employees_block();

    // SELECT * FROM employees LIMIT 2
    let sliced = block.slice(0, 2);
    assert_eq!(sliced.num_rows(), 2);
}

// ===========================================================================
// JOIN tests (inner, left)
// ===========================================================================

#[test]
fn test_inner_join_block() {
    let employees = common::create_employees_block();
    let departments = common::create_departments_block();

    // Simulate: SELECT e.name, d.name FROM employees e
    //           INNER JOIN departments d ON e.department = d.name

    let emp_dept_col = employees.column_by_name("department").unwrap().1;
    let dept_name_col = departments.column_by_name("name").unwrap().1;
    let emp_name_col = employees.column_by_name("name").unwrap().1;

    let mut result_names = Vec::new();
    let mut result_depts = Vec::new();

    for i in 0..employees.num_rows() {
        let emp_dept = emp_dept_col.scalar_at(i);
        if let types::ScalarValue::String(ref dept) = emp_dept {
            for j in 0..departments.num_rows() {
                let dept_name = dept_name_col.scalar_at(j);
                if let types::ScalarValue::String(ref dn) = dept_name
                    && dept == dn
                        && let types::ScalarValue::String(ref name) = emp_name_col.scalar_at(i) {
                            result_names.push(name.clone());
                            result_depts.push(dn.clone());
                        }
            }
        }
    }

    assert_eq!(result_names.len(), 5); // All employees match a department
    assert_eq!(result_names[0], "Alice");
    assert_eq!(result_depts[0], "Engineering");
}

#[test]
fn test_left_join_block() {
    let employees = common::create_employees_block();
    let departments = common::create_departments_block();

    // Simulate a LEFT JOIN: even if no department match, employee is included
    let emp_dept_col = employees.column_by_name("department").unwrap().1;
    let dept_name_col = departments.column_by_name("name").unwrap().1;
    let _dept_budget_col = departments.column_by_name("budget").unwrap().1;

    let mut join_count = 0;
    let mut null_budget_count = 0;

    for i in 0..employees.num_rows() {
        let emp_dept = emp_dept_col.scalar_at(i);
        let mut matched = false;

        if let types::ScalarValue::String(ref dept) = emp_dept {
            for j in 0..departments.num_rows() {
                let dept_name = dept_name_col.scalar_at(j);
                if let types::ScalarValue::String(ref dn) = dept_name
                    && dept == dn {
                        matched = true;
                        join_count += 1;
                    }
            }
        }

        if !matched {
            null_budget_count += 1;
        }
    }

    // All 5 employees have matching departments, so no nulls
    assert_eq!(join_count, 5);
    assert_eq!(null_budget_count, 0);
}

// ===========================================================================
// Aggregate functions (COUNT, SUM, AVG, MIN, MAX)
// ===========================================================================

#[test]
fn test_aggregate_count() {
    let block = common::create_employees_block();
    // SELECT COUNT(*) FROM employees
    assert_eq!(block.num_rows(), 5);
}

#[test]
fn test_aggregate_sum() {
    let block = common::create_employees_block();
    // SELECT SUM(salary) FROM employees
    let salary_col = block.column_by_name("salary").unwrap().1;
    let mut sum = 0.0;
    for i in 0..block.num_rows() {
        if let types::ScalarValue::Float64(v) = salary_col.scalar_at(i) {
            sum += v;
        }
    }
    // 95000 + 75000 + 110000 + 82000 + 68000 = 430000
    assert!((sum - 430000.0).abs() < 0.01);
}

#[test]
fn test_aggregate_avg() {
    let block = common::create_employees_block();
    // SELECT AVG(salary) FROM employees
    let salary_col = block.column_by_name("salary").unwrap().1;
    let mut sum = 0.0;
    let mut count = 0;
    for i in 0..block.num_rows() {
        if let types::ScalarValue::Float64(v) = salary_col.scalar_at(i) {
            sum += v;
            count += 1;
        }
    }
    let avg = sum / count as f64;
    // 430000 / 5 = 86000
    assert!((avg - 86000.0).abs() < 0.01);
}

#[test]
fn test_aggregate_min_max() {
    let block = common::create_employees_block();

    let salary_col = block.column_by_name("salary").unwrap().1;
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;

    for i in 0..block.num_rows() {
        if let types::ScalarValue::Float64(v) = salary_col.scalar_at(i) {
            min_val = min_val.min(v);
            max_val = max_val.max(v);
        }
    }

    assert!((min_val - 68000.0).abs() < 0.01); // Eve
    assert!((max_val - 110000.0).abs() < 0.01); // Charlie
}

#[test]
fn test_aggregate_sum_by_group() {
    let block = common::create_employees_block();

    // SELECT department, SUM(salary) FROM employees GROUP BY department
    let dept_col = block.column_by_name("department").unwrap().1;
    let salary_col = block.column_by_name("salary").unwrap().1;

    let mut groups: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    for i in 0..block.num_rows() {
        let dept = dept_col.scalar_at(i);
        let salary = salary_col.scalar_at(i);
        if let (types::ScalarValue::String(d), types::ScalarValue::Float64(s)) = (dept, salary) {
            *groups.entry(d).or_insert(0.0) += s;
        }
    }

    // Engineering: 95000 + 110000 = 205000
    assert!((groups.get("Engineering").unwrap() - 205000.0).abs() < 0.01);
    // Marketing: 75000 + 82000 = 157000
    assert!((groups.get("Marketing").unwrap() - 157000.0).abs() < 0.01);
    // Sales: 68000
    assert!((groups.get("Sales").unwrap() - 68000.0).abs() < 0.01);
}

// ===========================================================================
// SHOW DATABASES / SHOW TABLES
// ===========================================================================

#[test]
fn test_show_databases() {
    let catalog = common::create_test_catalog();
    let dbs = catalog.list_databases();

    assert!(dbs.contains(&"test_db".to_string()));
    assert!(dbs.contains(&"information_schema".to_string()));
}

#[test]
fn test_show_tables() {
    let catalog = common::create_test_catalog();
    let tables = catalog.list_tables("test_db").unwrap();

    assert!(tables.contains(&"employees".to_string()));
    assert!(tables.contains(&"departments".to_string()));
    assert_eq!(tables.len(), 2);
}

#[test]
fn test_show_tables_nonexistent_db() {
    let catalog = common::create_test_catalog();
    let result = catalog.list_tables("nonexistent");
    assert!(result.is_none());
}

// ===========================================================================
// SQL parsing integration tests
// ===========================================================================

#[test]
fn test_parse_simple_select() {
    let result = fe_sql_parser::parse_sql("SELECT 1");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
}

#[test]
fn test_parse_select_with_where() {
    let result = fe_sql_parser::parse_sql(
        "SELECT id, name FROM employees WHERE salary > 80000",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_select_with_group_by() {
    let result = fe_sql_parser::parse_sql(
        "SELECT department, COUNT(*) FROM employees GROUP BY department",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_select_with_order_by_limit() {
    let result = fe_sql_parser::parse_sql(
        "SELECT * FROM employees ORDER BY salary DESC LIMIT 10",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_join() {
    let result = fe_sql_parser::parse_sql(
        "SELECT e.name, d.name FROM employees e INNER JOIN departments d ON e.department = d.name",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_left_join() {
    let result = fe_sql_parser::parse_sql(
        "SELECT e.name, d.budget FROM employees e LEFT JOIN departments d ON e.department = d.name",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_aggregate_functions() {
    let result = fe_sql_parser::parse_sql(
        "SELECT COUNT(*), SUM(salary), AVG(salary), MIN(salary), MAX(salary) FROM employees",
    );
    assert!(result.is_ok());
}

// ===========================================================================
// Planner integration tests
// ===========================================================================

#[test]
fn test_plan_select_query() {
    let catalog = common::create_test_catalog();
    let mut planner = Planner::new(catalog);
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql(
        "SELECT id, name FROM employees WHERE salary > 80000",
    ).unwrap();

    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan = plan.unwrap();
    // Should have: Scan -> Filter -> Project
    assert!(matches!(plan.node_type, PlanNodeType::Project(_)));
}

#[test]
fn test_plan_aggregate_query() {
    let catalog = common::create_test_catalog();
    let mut planner = Planner::new(catalog);
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql(
        "SELECT department, COUNT(*) FROM employees GROUP BY department",
    ).unwrap();

    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan = plan.unwrap();
    // Should have: Scan -> Aggregate -> Project
    assert!(matches!(plan.node_type, PlanNodeType::Project(_)));
}

#[test]
fn test_plan_order_limit_query() {
    let catalog = common::create_test_catalog();
    let mut planner = Planner::new(catalog);
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql(
        "SELECT * FROM employees ORDER BY salary DESC LIMIT 5",
    ).unwrap();

    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan = plan.unwrap();
    // Should have: Scan -> Sort -> Limit -> Project
    assert!(matches!(plan.node_type, PlanNodeType::Limit(_)));
}

// ===========================================================================
// CSV Data Import tests
// ===========================================================================

#[test]
fn test_csv_reader_zclawbench() {
    use std::fs::File;
    use std::io::BufReader;
    use data_io::csv_reader::CsvReader;

    // Read the ZClawBench simplified CSV (260KB, 696 rows)
    let file = File::open("/tmp/ZClawBench/zclawbench_simple.csv").unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = CsvReader::new(reader).with_header();

    // Read first batch (this also reads headers internally)
    let batch = csv_reader.next_batch().unwrap().unwrap();

    // Check headers were read
    let headers = csv_reader.headers();
    assert_eq!(headers.len(), 4);
    assert_eq!(headers[0], "task_id");
    assert_eq!(headers[1], "model_name");
    assert_eq!(headers[2], "task_category");
    assert_eq!(headers[3], "trajectory_summary");

    // Verify batch has data
    assert!(batch.num_rows() > 0);
    assert_eq!(batch.num_columns(), 4);

    // Verify schema has the expected columns
    let schema = batch.schema();
    assert_eq!(schema.num_fields(), 4);

    println!("CSV import test passed: {} rows, {} columns",
        batch.num_rows(), batch.num_columns());
}

#[test]
fn test_csv_create_table_and_query() {
    use std::fs::File;
    use std::io::BufReader;
    use data_io::csv_reader::CsvReader;
    use types::DataType;

    let file = File::open("/tmp/ZClawBench/zclawbench_simple.csv").unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = CsvReader::new(reader).with_header();
    let batch = csv_reader.next_batch().unwrap().unwrap();

    // Verify we can read the data correctly
    assert!(batch.num_rows() > 0);
    assert_eq!(batch.num_columns(), 4);

    // Test query planning with parsed SQL
    let catalog = common::create_test_catalog();
    let planner = Planner::new(catalog);

    // Plan a simple SELECT query
    let stmts = fe_sql_parser::parse_sql(
        "SELECT id, name, department, salary FROM employees WHERE salary > 3000",
    ).unwrap();

    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    println!("Query planning test passed");
}

// ===========================================================================
// Parquet Data Import tests
// ===========================================================================

#[test]
fn test_parquet_reader_zclawbench() {
    use data_io::parquet_reader::ParquetReader;

    // Read the ZClawBench parquet file directly (23MB, 696 rows)
    let mut reader = ParquetReader::open("/tmp/ZClawBench/train.parquet").unwrap();

    // Check metadata
    assert_eq!(reader.num_rows(), 696);
    assert_eq!(reader.num_columns(), 4);

    // Check schema
    let schema = reader.schema();
    assert_eq!(schema.num_fields(), 4);

    // Read first batch
    let batch = reader.next_batch().unwrap().unwrap();
    assert!(batch.num_rows() > 0);
    assert_eq!(batch.num_columns(), 4);

    println!("Parquet import test passed: {} rows, {} columns, {} total rows",
        batch.num_rows(), batch.num_columns(), reader.num_rows());
}

#[test]
fn test_parquet_read_performance() {
    use std::time::Instant;
    use data_io::parquet_reader::ParquetReader;

    // Read test
    let start = Instant::now();
    let mut reader = ParquetReader::open("/tmp/ZClawBench/train.parquet").unwrap();
    let mut total_rows = 0;
    let mut blocks = 0;

    while let Some(batch) = reader.next_batch().unwrap() {
        total_rows += batch.num_rows();
        blocks += 1;
    }
    let read_time = start.elapsed().as_secs_f64();

    println!("\n=== RorisDB Parquet Benchmark ===");
    println!("Read: {:.4}s, {} rows, {} blocks", read_time, total_rows, blocks);
    println!("Throughput: {:.2} MB/s", 23.0 / read_time);

    // Verify correct data
    assert_eq!(total_rows, 696);
}

// ===========================================================================
// Backup/Restore SQL parsing tests
// ===========================================================================

#[test]
fn test_parse_create_repository_local() {
    let result = fe_sql_parser::parse_sql("CREATE REPOSITORY local_repo");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        fe_sql_parser::ast::Statement::CreateRepository(stmt) => {
            assert_eq!(stmt.name, "local_repo");
            assert!(matches!(stmt.repo_type, fe_sql_parser::ast::RepositoryType::Local));
        }
        _ => panic!("Expected CreateRepository statement"),
    }
}

#[test]
fn test_parse_create_repository_with_properties() {
    let result = fe_sql_parser::parse_sql(
        "CREATE REPOSITORY s3_repo WITH S3 PROPERTIES (\"endpoint\" = \"http://localhost:9000\")",
    );
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::CreateRepository(stmt) => {
            assert_eq!(stmt.name, "s3_repo");
            assert!(matches!(stmt.repo_type, fe_sql_parser::ast::RepositoryType::S3));
            assert!(!stmt.properties.is_empty());
        }
        _ => panic!("Expected CreateRepository statement"),
    }
}

#[test]
fn test_parse_drop_repository() {
    let result = fe_sql_parser::parse_sql("DROP REPOSITORY my_repo");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::DropRepository(stmt) => {
            assert_eq!(stmt.name, "my_repo");
            assert!(!stmt.if_exists);
        }
        _ => panic!("Expected DropRepository statement"),
    }
}

#[test]
fn test_parse_drop_repository_if_exists() {
    let result = fe_sql_parser::parse_sql("DROP REPOSITORY IF EXISTS my_repo");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::DropRepository(stmt) => {
            assert_eq!(stmt.name, "my_repo");
            assert!(stmt.if_exists);
        }
        _ => panic!("Expected DropRepository statement"),
    }
}

#[test]
fn test_parse_show_repositories() {
    let result = fe_sql_parser::parse_sql("SHOW REPOSITORIES");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::ShowRepositories => {}
        _ => panic!("Expected ShowRepositories statement"),
    }
}

#[test]
fn test_parse_backup_database() {
    let result = fe_sql_parser::parse_sql("BACKUP DATABASE mydb TO my_repo");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::BackupDatabase(stmt) => {
            assert_eq!(stmt.database, "mydb");
            assert_eq!(stmt.repository, "my_repo");
        }
        _ => panic!("Expected BackupDatabase statement"),
    }
}

#[test]
fn test_parse_backup_database_with_name() {
    let result = fe_sql_parser::parse_sql("BACKUP DATABASE mydb TO my_repo BACKUP backup_20240101");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::BackupDatabase(stmt) => {
            assert_eq!(stmt.database, "mydb");
            assert_eq!(stmt.repository, "my_repo");
            assert_eq!(stmt.backup_name, "backup_20240101");
        }
        _ => panic!("Expected BackupDatabase statement"),
    }
}

#[test]
fn test_parse_restore_database() {
    let result = fe_sql_parser::parse_sql("RESTORE DATABASE mydb FROM my_repo BACKUP backup_20240101");
    assert!(result.is_ok());
    let stmts = result.unwrap();
    match &stmts[0] {
        fe_sql_parser::ast::Statement::RestoreDatabase(stmt) => {
            assert_eq!(stmt.database, "mydb");
            assert_eq!(stmt.repository, "my_repo");
            assert_eq!(stmt.backup_name, "backup_20240101");
        }
        _ => panic!("Expected RestoreDatabase statement"),
    }
}

#[test]
fn test_plan_backup_database() {
    let catalog = common::create_test_catalog();
    let mut planner = Planner::new(catalog);
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql("BACKUP DATABASE test_db TO my_repo").unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::BackupDatabase(_) => {}
        _ => panic!("Expected BackupDatabase plan node"),
    }
}

#[test]
fn test_plan_restore_database() {
    let catalog = common::create_test_catalog();
    let mut planner = Planner::new(catalog);
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql("RESTORE DATABASE test_db FROM my_repo BACKUP backup_001").unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::RestoreDatabase(_) => {}
        _ => panic!("Expected RestoreDatabase plan node"),
    }
}

// ===========================================================================
// Materialized View tests
// ===========================================================================

#[test]
fn test_parse_create_materialized_view() {
    let result = fe_sql_parser::parse_sql(
        "CREATE MATERIALIZED VIEW mv1 AS SELECT department, COUNT(*) FROM employees GROUP BY department",
    );
    assert!(result.is_ok());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
}

#[test]
fn test_parse_create_materialized_view_with_refresh() {
    let result = fe_sql_parser::parse_sql(
        "CREATE MATERIALIZED VIEW mv1 REFRESH COMPLETE AS SELECT department, COUNT(*) FROM employees GROUP BY department",
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_drop_materialized_view() {
    let result = fe_sql_parser::parse_sql("DROP MATERIALIZED VIEW mv1");
    assert!(result.is_ok());
}

#[test]
fn test_parse_drop_materialized_view_if_exists() {
    let result = fe_sql_parser::parse_sql("DROP MATERIALIZED VIEW IF EXISTS mv1");
    assert!(result.is_ok());
}

#[test]
fn test_parse_alter_materialized_view_pause() {
    let result = fe_sql_parser::parse_sql("ALTER MATERIALIZED VIEW mv1 PAUSE REFRESH");
    assert!(result.is_ok());
}

#[test]
fn test_parse_alter_materialized_view_resume() {
    let result = fe_sql_parser::parse_sql("ALTER MATERIALIZED VIEW mv1 RESUME REFRESH");
    assert!(result.is_ok());
}

#[test]
fn test_parse_refresh_materialized_view() {
    let result = fe_sql_parser::parse_sql("REFRESH MATERIALIZED VIEW mv1 COMPLETE");
    assert!(result.is_ok());
}

#[test]
fn test_parse_refresh_materialized_view_fast() {
    let result = fe_sql_parser::parse_sql("REFRESH MATERIALIZED VIEW mv1 FAST");
    assert!(result.is_ok());
}

#[test]
fn test_plan_create_materialized_view() {
    let catalog = Arc::new(CatalogManager::new());
    let mut planner = Planner::new(catalog.clone());
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql(
        "CREATE MATERIALIZED VIEW mv1 AS SELECT department, COUNT(*) as cnt FROM employees GROUP BY department",
    ).unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::CreateMaterializedView(_) => {}
        other => panic!("Expected CreateMaterializedView, got: {:?}", other),
    }
}

#[test]
fn test_plan_drop_materialized_view() {
    let catalog = Arc::new(CatalogManager::new());
    let mut planner = Planner::new(catalog.clone());
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql("DROP MATERIALIZED VIEW mv1").unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::DropMaterializedView(_) => {}
        other => panic!("Expected DropMaterializedView, got: {:?}", other),
    }
}

#[test]
fn test_plan_alter_materialized_view() {
    let catalog = Arc::new(CatalogManager::new());
    let mut planner = Planner::new(catalog.clone());
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql("ALTER MATERIALIZED VIEW mv1 PAUSE REFRESH").unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::AlterMaterializedView(_) => {}
        other => panic!("Expected AlterMaterializedView, got: {:?}", other),
    }
}

#[test]
fn test_plan_refresh_materialized_view() {
    let catalog = Arc::new(CatalogManager::new());
    let mut planner = Planner::new(catalog.clone());
    planner.set_database("test_db");

    let stmts = fe_sql_parser::parse_sql("REFRESH MATERIALIZED VIEW mv1 COMPLETE").unwrap();
    let plan = planner.plan(stmts.into_iter().next().unwrap());
    assert!(plan.is_ok());

    let plan_node = plan.unwrap();
    match plan_node.node_type {
        fe_sql_planner::PlanNodeType::RefreshMaterializedView(_) => {}
        other => panic!("Expected RefreshMaterializedView, got: {:?}", other),
    }
}

#[test]
fn test_catalog_materialized_view_crud() {
    use fe_catalog::materialized_view::{MaterializedView, MaterializedViewColumn, RefreshStrategy};

    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let mv = MaterializedView::new(1, "mv1".to_string(), "test_db".to_string(), "SELECT department, COUNT(*) FROM employees GROUP BY department".to_string())
        .with_base_tables(vec![("test_db".to_string(), "employees".to_string())])
        .with_refresh(RefreshStrategy::Manual)
        .with_schema(vec![
            MaterializedViewColumn { name: "department".to_string(), data_type: "String".to_string() },
            MaterializedViewColumn { name: "count".to_string(), data_type: "Int64".to_string() },
        ]);

    catalog.create_materialized_view(mv).unwrap();

    let retrieved = catalog.get_materialized_view("test_db", "mv1");
    assert!(retrieved.is_some());
    let mv = retrieved.unwrap();
    assert_eq!(mv.name, "mv1");
    assert_eq!(mv.base_tables, vec![("test_db".to_string(), "employees".to_string())]);

    let mvs_in_db = catalog.list_materialized_views("test_db");
    assert_eq!(mvs_in_db.len(), 1);
    assert_eq!(mvs_in_db[0].name, "mv1");

    catalog.drop_materialized_view("test_db", "mv1").unwrap();
    assert!(catalog.get_materialized_view("test_db", "mv1").is_none());
}

#[test]
fn test_materialized_view_rewrite_basic() {
    use fe_catalog::materialized_view::{MaterializedView, MaterializedViewColumn, RefreshStrategy};
    use fe_sql_planner::materialized_view::rewrite_query;

    let catalog = common::create_test_catalog();

    let mv = MaterializedView::new(1, "dept_cnt".to_string(), "test_db".to_string(), "SELECT department, COUNT(*) FROM employees GROUP BY department".to_string())
        .with_base_tables(vec![("test_db".to_string(), "employees".to_string())])
        .with_schema(vec![
            MaterializedViewColumn { name: "department".to_string(), data_type: "String".to_string() },
            MaterializedViewColumn { name: "count".to_string(), data_type: "Int64".to_string() },
        ]);

    catalog.create_materialized_view(mv).unwrap();

    let query = fe_sql_parser::parse_sql("SELECT department, count FROM dept_cnt")
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    if let fe_sql_parser::ast::Statement::Query(query_stmt) = query {
        let rewritten = rewrite_query(&query_stmt, &catalog);
        assert!(rewritten.is_some());
    }
}

#[test]
fn test_materialized_view_rewrite_no_mv() {
    use fe_sql_planner::materialized_view::rewrite_query;

    let catalog = common::create_test_catalog();

    let query = fe_sql_parser::parse_sql("SELECT department, COUNT(*) FROM employees GROUP BY department")
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    if let fe_sql_parser::ast::Statement::Query(query_stmt) = query {
        let rewritten = rewrite_query(&query_stmt, &catalog);
        assert!(rewritten.is_none());
    }
}
