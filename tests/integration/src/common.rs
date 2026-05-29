use std::collections::HashMap;
use std::sync::Arc;

use fe_catalog::table::TableColumn;
use fe_catalog::{CatalogManager, Table};
use types::{
    Block, DataType, Field, Schema, Vector,
    vector::{Float64Vector, Int64Vector, StringVector},
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
        tablet_id: 0,
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
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 5,
        data_size: 0,
        stats: None,
        view_definition: None,
    };
    catalog.create_table("test_db", employees).unwrap();

    // Departments table
    let departments = Table {
        tablet_id: 0,
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
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: 3,
        data_size: 0,
        stats: None,
        view_definition: None,
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
            "Engineering",
            "Marketing",
            "Sales",
        ])),
        Vector::Float64(Float64Vector::from_nullable_vec(vec![
            Some(500000.0),
            Some(300000.0),
            Some(200000.0),
        ])),
    ];

    Block::new(schema, columns)
}

// ---------------------------------------------------------------------------
// SSB (Star Schema Benchmark) test data
// ---------------------------------------------------------------------------

/// Create a CatalogManager pre-loaded with the SSB database and dimension/fact tables.
pub fn create_ssb_catalog() -> Arc<CatalogManager> {
    let catalog = Arc::new(CatalogManager::new());
    catalog.create_database("ssb").unwrap();

    // date_dim
    catalog
        .create_table(
            "ssb",
            Table {
                tablet_id: 0,
                id: 100,
                name: "date_dim".into(),
                database: "ssb".into(),
                columns: vec![
                    TableColumn {
                        name: "d_datekey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "d_date".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "d_year".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                ],
                keys_type: fe_catalog::table::KeysType::Duplicate,
                unique_keys: vec![],
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 100,
                data_size: 0,
                stats: None,
                view_definition: None,
            },
        )
        .unwrap();

    // supplier
    catalog
        .create_table(
            "ssb",
            Table {
                tablet_id: 0,
                id: 101,
                name: "supplier".into(),
                database: "ssb".into(),
                columns: vec![
                    TableColumn {
                        name: "s_suppkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "s_name".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "s_nation".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                ],
                keys_type: fe_catalog::table::KeysType::Duplicate,
                unique_keys: vec![],
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 20,
                data_size: 0,
                stats: None,
                view_definition: None,
            },
        )
        .unwrap();

    // customer
    catalog
        .create_table(
            "ssb",
            Table {
                tablet_id: 0,
                id: 102,
                name: "customer".into(),
                database: "ssb".into(),
                columns: vec![
                    TableColumn {
                        name: "c_custkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "c_name".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "c_nation".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                ],
                keys_type: fe_catalog::table::KeysType::Duplicate,
                unique_keys: vec![],
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 50,
                data_size: 0,
                stats: None,
                view_definition: None,
            },
        )
        .unwrap();

    // part
    catalog
        .create_table(
            "ssb",
            Table {
                tablet_id: 0,
                id: 103,
                name: "part".into(),
                database: "ssb".into(),
                columns: vec![
                    TableColumn {
                        name: "p_partkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "p_name".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "p_category".into(),
                        data_type: DataType::String,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                ],
                keys_type: fe_catalog::table::KeysType::Duplicate,
                unique_keys: vec![],
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 30,
                data_size: 0,
                stats: None,
                view_definition: None,
            },
        )
        .unwrap();

    // lineorder (fact table)
    catalog
        .create_table(
            "ssb",
            Table {
                tablet_id: 0,
                id: 104,
                name: "lineorder".into(),
                database: "ssb".into(),
                columns: vec![
                    TableColumn {
                        name: "lo_orderkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "lo_custkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "lo_suppkey".into(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                    TableColumn {
                        name: "lo_revenue".into(),
                        data_type: DataType::Float64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: String::new(),
                    },
                ],
                keys_type: fe_catalog::table::KeysType::Duplicate,
                unique_keys: vec![],
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 500,
                data_size: 0,
                stats: None,
                view_definition: None,
            },
        )
        .unwrap();

    catalog
}

/// Create a sample lineorder Block with 20 rows of SSB-style data.
pub fn create_lineorder_block() -> Block {
    let schema = Schema::new(vec![
        Field::new("lo_orderkey", DataType::Int64, false),
        Field::new("lo_custkey", DataType::Int64, false),
        Field::new("lo_suppkey", DataType::Int64, false),
        Field::new("lo_revenue", DataType::Float64, false),
    ]);
    let orderkeys: Vec<i64> = (1..=20).collect();
    let custkeys: Vec<i64> = (1..=5).cycle().take(20).collect();
    let suppkeys: Vec<i64> = vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5];
    let revenues: Vec<f64> = (1..=20).map(|i| i as f64 * 500.0).collect();

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(orderkeys)),
        Vector::Int64(Int64Vector::from_vec(custkeys)),
        Vector::Int64(Int64Vector::from_vec(suppkeys)),
        Vector::Float64(Float64Vector::from_vec(revenues)),
    ];
    Block::new(schema, columns)
}
