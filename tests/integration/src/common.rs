use std::sync::Arc;
use std::collections::HashMap;

use fe_catalog::table::TableColumn;
use fe_catalog::{CatalogManager, Table};
use types::{
    vector::{Float64Vector, Int64Vector, StringVector},
    Block, DataType, Field, Schema, Vector,
};

/// Create a CatalogManager pre-loaded with a test database and sample tables.
///
/// Database: "test_db"
/// Tables:
///   - "employees" (id INT64, name STRING, department STRING, salary FLOAT64)
///   - "departments" (id INT64, name STRING, budget FLOAT64)
pub fn create_test_catalog() -> Arc<CatalogManager> {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("test_db").unwrap();

    // Employees table
    let employees = Table {
        id: 1,
        name: "employees".to_string(),
        database: "test_db".to_string(),
        columns: vec![
            TableColumn {
                name: "id".into(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "name".into(),
                data_type: DataType::String,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "department".into(),
                data_type: DataType::String,
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "salary".into(),
                data_type: DataType::Float64,
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
        properties: HashMap::new(),
        row_count: 5,
        data_size: 0,
        stats: None,
    };
    catalog.create_table("test_db", employees).unwrap();

    // Departments table
    let departments = Table {
        id: 2,
        name: "departments".to_string(),
        database: "test_db".to_string(),
        columns: vec![
            TableColumn {
                name: "id".into(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "name".into(),
                data_type: DataType::String,
                nullable: false,
                default_value: None,
                agg_type: None,
                comment: String::new(),
            },
            TableColumn {
                name: "budget".into(),
                data_type: DataType::Float64,
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
        properties: HashMap::new(),
        row_count: 3,
        data_size: 0,
        stats: None,
    };
    catalog.create_table("test_db", departments).unwrap();

    catalog
}

/// Create a sample employees Block with 5 rows of test data.
pub fn create_employees_block() -> Block {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
        Field::new("department", DataType::String, true),
        Field::new("salary", DataType::Float64, true),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::String(StringVector::from_vec(vec![
            "Alice", "Bob", "Charlie", "Diana", "Eve",
        ])),
        Vector::String(StringVector::from_option_vec(vec![
            Some("Engineering".to_string()),
            Some("Marketing".to_string()),
            Some("Engineering".to_string()),
            Some("Marketing".to_string()),
            Some("Sales".to_string()),
        ])),
        Vector::Float64(Float64Vector::from_nullable_vec(vec![
            Some(95000.0),
            Some(75000.0),
            Some(110000.0),
            Some(82000.0),
            Some(68000.0),
        ])),
    ];

    Block::new(schema, columns)
}

/// Create a sample departments Block with 3 rows of test data.
pub fn create_departments_block() -> Block {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
        Field::new("budget", DataType::Float64, true),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::String(StringVector::from_vec(vec![
            "Engineering", "Marketing", "Sales",
        ])),
        Vector::Float64(Float64Vector::from_nullable_vec(vec![
            Some(500000.0),
            Some(300000.0),
            Some(200000.0),
        ])),
    ];

    Block::new(schema, columns)
}

/// Create a StorageEngine with a sample tablet for testing.
/// Uses a temporary directory for data storage.
pub fn create_test_storage_engine() -> be_storage::StorageEngine {
    use be_storage::tablet::{TabletColumn, TabletSchema};

    let temp_dir = std::env::temp_dir().join("rovisdb_test_storage");
    let engine = be_storage::StorageEngine::open(&temp_dir).unwrap();

    let schema = TabletSchema {
        tablet_id: 1,
        columns: vec![
            TabletColumn {
                name: "id".into(),
                data_type: DataType::Int64,
                nullable: false,
                is_key: true,
                agg_type: None,
            },
            TabletColumn {
                name: "value".into(),
                data_type: DataType::String,
                nullable: true,
                is_key: false,
                agg_type: None,
            },
        ],
        keys_type: "DUP".to_string(),
        num_rows_per_row_block: 1024,
    };

    engine.create_tablet(1, schema).unwrap();
    engine
}
