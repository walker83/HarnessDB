use std::sync::Arc;
use std::collections::HashMap;

use fe_catalog::table::{TableColumn, KeysType, PartitionInfo, Partition, DistributionInfo};
use fe_catalog::{CatalogManager, Table};
use integration_tests::common;
use types::DataType;

// ===========================================================================
// 1.1 CREATE/DROP DATABASE full lifecycle
// ===========================================================================

#[test]
fn test_database_create_show_use_drop() {
    let catalog = Arc::new(CatalogManager::new());

    // Create
    catalog.create_database("lifecycle_db").unwrap();

    // Show (list_databases includes it)
    let dbs = catalog.list_databases();
    assert!(dbs.contains(&"lifecycle_db".to_string()));

    // Get
    assert!(catalog.get_database("lifecycle_db").is_some());

    // Drop
    catalog.drop_database("lifecycle_db").unwrap();
    assert!(catalog.get_database("lifecycle_db").is_none());
    assert!(!catalog.list_databases().contains(&"lifecycle_db".to_string()));
}

#[test]
fn test_create_database_duplicate_error() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("dup_db").unwrap();
    let result = catalog.create_database("dup_db");
    assert!(result.is_err());
}

#[test]
fn test_drop_database_nonexistent_error() {
    let catalog = Arc::new(CatalogManager::new());
    let result = catalog.drop_database("nonexistent_db");
    assert!(result.is_err());
}

// ===========================================================================
// 1.2 CREATE TABLE with all data types
// ===========================================================================

#[test]
fn test_create_table_all_types() {
    let catalog = common::create_test_catalog();
    let db = catalog.get_database("test_db").unwrap();
    let table = db.get_table("employees").unwrap();

    assert_eq!(table.columns.len(), 4);
    assert_eq!(table.columns[0].data_type, DataType::Int64);
    assert_eq!(table.columns[1].data_type, DataType::String);
    assert_eq!(table.columns[2].data_type, DataType::String);
    assert_eq!(table.columns[3].data_type, DataType::Float64);
    assert!(!table.columns[0].nullable);
    assert!(table.columns[2].nullable);
}

#[test]
fn test_create_table_parse() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    // Parse CREATE TABLE SQL
    let result = fe_sql_parser::parse_sql("CREATE TABLE employees (id INT64, name STRING, department STRING, salary FLOAT64)");
    assert!(result.is_ok());
}

#[test]
fn test_create_table_if_not_exists_parse() {
    let result = fe_sql_parser::parse_sql("CREATE TABLE IF NOT EXISTS t (id INT64)");
    assert!(result.is_ok());
}

#[test]
fn test_drop_table_parse() {
    let result = fe_sql_parser::parse_sql("DROP TABLE employees");
    assert!(result.is_ok());
}

#[test]
fn test_drop_table_if_exists_parse() {
    let result = fe_sql_parser::parse_sql("DROP TABLE IF EXISTS nonexistent");
    assert!(result.is_ok());
}

#[test]
fn test_truncate_table_parse() {
    let result = fe_sql_parser::parse_sql("TRUNCATE TABLE employees");
    assert!(result.is_ok());
}

#[test]
fn test_create_table_all_data_types_parse() {
    let sql = "CREATE TABLE all_types (
        col_bool BOOLEAN,
        col_i8 INT8,
        col_i16 INT16,
        col_i32 INT32,
        col_i64 INT64,
        col_f32 FLOAT32,
        col_f64 FLOAT64,
        col_str STRING,
        col_date DATE,
        col_datetime DATETIME
    )";
    let result = fe_sql_parser::parse_sql(sql);
    assert!(result.is_ok());
}

// ===========================================================================
// 1.3 Partition table
// ===========================================================================

#[test]
fn test_create_table_with_range_partition() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let table = Table {
        id: 10,
        name: "orders".into(),
        database: "test_db".into(),
        columns: vec![
            TableColumn { name: "id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "order_date".into(), data_type: DataType::String, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "amount".into(), data_type: DataType::Float64, nullable: true, default_value: None, agg_type: None, comment: String::new() },
        ],
        keys_type: KeysType::Duplicate,
        unique_keys: vec![],
        partition_info: Some(PartitionInfo {
            partition_type: "RANGE".into(),
            columns: vec!["order_date".into()],
            partitions: vec![
                Partition { id: 1, name: "p202401".into(), range_start: Some("2024-01-01".into()), range_end: Some("2024-02-01".into()) },
                Partition { id: 2, name: "p202402".into(), range_start: Some("2024-02-01".into()), range_end: Some("2024-03-01".into()) },
                Partition { id: 3, name: "p202403".into(), range_start: Some("2024-03-01".into()), range_end: Some("2024-04-01".into()) },
            ],
        }),
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };

    catalog.create_table("test_db", table).unwrap();
    let t = catalog.get_table("test_db", "orders").unwrap();
    let pi = t.partition_info.as_ref().unwrap();
    assert_eq!(pi.partition_type, "RANGE");
    assert_eq!(pi.columns.len(), 1);
    assert_eq!(pi.partitions.len(), 3);
    assert_eq!(pi.partitions[0].name, "p202401");
    assert_eq!(pi.partitions[2].range_end, Some("2024-04-01".into()));
}

// ===========================================================================
// 1.4 Distribution table
// ===========================================================================

#[test]
fn test_create_table_with_hash_distribution() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let table = Table {
        id: 11,
        name: "events".into(),
        database: "test_db".into(),
        columns: vec![
            TableColumn { name: "event_id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "user_id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "event_type".into(), data_type: DataType::String, nullable: false, default_value: None, agg_type: None, comment: String::new() },
        ],
        keys_type: KeysType::Duplicate,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: Some(DistributionInfo {
            dist_type: "HASH".into(),
            columns: vec!["user_id".into()],
            buckets: 4,
        }),
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };

    catalog.create_table("test_db", table).unwrap();
    let t = catalog.get_table("test_db", "events").unwrap();
    let di = t.distribution_info.as_ref().unwrap();
    assert_eq!(di.dist_type, "HASH");
    assert_eq!(di.columns, vec!["user_id"]);
    assert_eq!(di.buckets, 4);
}

// ===========================================================================
// 1.5 KeysType tests
// ===========================================================================

#[test]
fn test_create_table_duplicate_key() {
    let catalog = common::create_test_catalog();
    let t = catalog.get_table("test_db", "employees").unwrap();
    assert!(matches!(t.keys_type, KeysType::Duplicate));
}

#[test]
fn test_create_table_aggregate_key() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let table = Table {
        id: 20,
        name: "agg_table".into(),
        database: "test_db".into(),
        columns: vec![
            TableColumn { name: "user_id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "total_sales".into(), data_type: DataType::Float64, nullable: true, default_value: None, agg_type: Some("SUM".into()), comment: String::new() },
            TableColumn { name: "visit_count".into(), data_type: DataType::Int64, nullable: true, default_value: None, agg_type: Some("REPLACE".into()), comment: String::new() },
        ],
        keys_type: KeysType::Aggregate,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };

    catalog.create_table("test_db", table).unwrap();
    let t = catalog.get_table("test_db", "agg_table").unwrap();
    assert!(matches!(t.keys_type, KeysType::Aggregate));
    assert_eq!(t.columns[1].agg_type, Some("SUM".into()));
    assert_eq!(t.columns[2].agg_type, Some("REPLACE".into()));
}

#[test]
fn test_create_table_unique_key() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let table = Table {
        id: 21,
        name: "unique_table".into(),
        database: "test_db".into(),
        columns: vec![
            TableColumn { name: "user_id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "user_name".into(), data_type: DataType::String, nullable: false, default_value: None, agg_type: None, comment: String::new() },
        ],
        keys_type: KeysType::Unique,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };

    catalog.create_table("test_db", table).unwrap();
    let t = catalog.get_table("test_db", "unique_table").unwrap();
    assert!(matches!(t.keys_type, KeysType::Unique));
}

#[test]
fn test_create_table_primary_key() {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    let table = Table {
        id: 22,
        name: "pk_table".into(),
        database: "test_db".into(),
        columns: vec![
            TableColumn { name: "id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "data".into(), data_type: DataType::String, nullable: true, default_value: None, agg_type: None, comment: String::new() },
        ],
        keys_type: KeysType::Primary,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };

    catalog.create_table("test_db", table).unwrap();
    let t = catalog.get_table("test_db", "pk_table").unwrap();
    assert!(matches!(t.keys_type, KeysType::Primary));
}

// ===========================================================================
// 1.6 ALTER TABLE
// ===========================================================================

#[test]
fn test_alter_table_add_column() {
    let result = fe_sql_parser::parse_sql("ALTER TABLE employees ADD COLUMN age INT64");
    assert!(result.is_ok(), "ALTER TABLE should parse successfully");
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        fe_sql_parser::ast::Statement::AlterTable(alter) => {
            assert_eq!(alter.table, "employees");
            assert_eq!(alter.operations.len(), 1);
        }
        other => panic!("Expected AlterTable, got {:?}", other),
    }
}

// ===========================================================================
// SHOW / DESCRIBE
// ===========================================================================

#[test]
fn test_show_databases_catalog() {
    let catalog = common::create_test_catalog();
    let dbs = catalog.list_databases();
    assert!(dbs.contains(&"test_db".to_string()));
    assert!(dbs.contains(&"information_schema".to_string()));
}

#[test]
fn test_show_tables_catalog() {
    let catalog = common::create_test_catalog();
    let tables = catalog.list_tables("test_db").unwrap();
    assert!(tables.contains(&"employees".to_string()));
    assert!(tables.contains(&"departments".to_string()));
}

#[test]
fn test_show_create_table_parse() {
    let result = fe_sql_parser::parse_sql("SHOW CREATE TABLE employees");
    assert!(result.is_ok());
}

// ===========================================================================
// CREATE VIEW
// ===========================================================================

#[test]
fn test_create_view_parse() {
    let result = fe_sql_parser::parse_sql(
        "CREATE VIEW high_earners AS SELECT name, salary FROM employees WHERE salary > 80000");
    assert!(result.is_ok());
}

// ===========================================================================
// Multiple DDL operations in sequence
// ===========================================================================

#[test]
fn test_ddl_full_lifecycle() {
    let catalog = Arc::new(CatalogManager::new());

    // Create DB
    catalog.create_database("lifecycle_test").unwrap();

    // Create table via catalog
    let table = Table {
        id: 100,
        name: "t1".into(),
        database: "lifecycle_test".into(),
        columns: vec![
            TableColumn { name: "id".into(), data_type: DataType::Int64, nullable: false, default_value: None, agg_type: None, comment: String::new() },
            TableColumn { name: "value".into(), data_type: DataType::String, nullable: true, default_value: None, agg_type: None, comment: String::new() },
        ],
        keys_type: KeysType::Duplicate,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 0,
        data_size: 0,
        stats: None,
        view_definition: None,
    };
    catalog.create_table("lifecycle_test", table).unwrap();

    // Verify table exists
    assert!(catalog.get_table("lifecycle_test", "t1").is_some());

    // Drop table
    catalog.drop_table("lifecycle_test", "t1").unwrap();
    assert!(catalog.get_table("lifecycle_test", "t1").is_none());

    // Drop database
    catalog.drop_database("lifecycle_test").unwrap();
    assert!(catalog.get_database("lifecycle_test").is_none());
}