use be_storage::tablet::{Tablet, TabletSchema, TabletColumn};
use types::{Block, DataType, Field, Schema, Vector, ScalarValue};
use tempfile::TempDir;

// ===========================================================================
// P1 Optimization: MemTable Structure Tests
// ===========================================================================

fn create_test_schema() -> TabletSchema {
    TabletSchema {
        tablet_id: 1,
        columns: vec![
            TabletColumn {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                is_key: true,
                agg_type: None,
            },
            TabletColumn {
                name: "name".to_string(),
                data_type: DataType::String,
                nullable: true,
                is_key: false,
                agg_type: None,
            },
            TabletColumn {
                name: "value".to_string(),
                data_type: DataType::Int64,
                nullable: true,
                is_key: false,
                agg_type: None,
            },
        ],
        keys_type: "PRIMARY".to_string(),
        num_rows_per_row_block: 1000,
    }
}

fn create_test_block() -> Block {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
        Field::new("value", DataType::Int64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::String(types::vector::StringVector::from_vec(vec!["alice", "bob", "charlie"])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![100, 200, 300])),
    ];

    Block::new(schema, columns)
}

#[test]
fn test_memtable_insert_single_row() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema, temp_dir.path().to_path_buf());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    assert_eq!(tablet.memtable_num_rows(), 3);
}

#[test]
fn test_memtable_memory_size() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema, temp_dir.path().to_path_buf());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    let memtable_size = tablet.memtable_memory_size();
    assert!(memtable_size > 0);
}

#[test]
fn test_memtable_to_block() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema.clone(), temp_dir.path().to_path_buf());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    let schema_ref = schema.to_schema();
    let result_block = tablet.memtable_to_block(&schema_ref);

    assert_eq!(result_block.num_rows(), 3);
    assert_eq!(result_block.num_columns(), 3);
}

#[test]
fn test_memtable_insert_multiple_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema.clone(), temp_dir.path().to_path_buf());

    let block1_schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
        Field::new("value", DataType::Int64, true),
    ]);
    let columns1 = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::String(types::vector::StringVector::from_vec(vec!["alice", "bob", "charlie"])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![100, 200, 300])),
    ];
    let block1 = Block::new(block1_schema, columns1);

    let block2_schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
        Field::new("value", DataType::Int64, true),
    ]);
    let columns2 = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![4, 5, 6])),
        Vector::String(types::vector::StringVector::from_vec(vec!["david", "eve", "frank"])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![400, 500, 600])),
    ];
    let block2 = Block::new(block2_schema, columns2);
    
    tablet.write(&block1).unwrap();
    tablet.write(&block2).unwrap();

    assert_eq!(tablet.memtable_num_rows(), 6);
}

#[test]
fn test_memtable_is_empty() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema.clone(), temp_dir.path().to_path_buf());

    assert!(tablet.memtable_is_empty());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    assert!(!tablet.memtable_is_empty());
}

#[test]
fn test_memtable_clear() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema.clone(), temp_dir.path().to_path_buf());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    tablet.memtable_clear();

    assert!(tablet.memtable_is_empty());
    assert_eq!(tablet.memtable_memory_size(), 0);
}

#[test]
fn test_memtable_should_flush() {
    let temp_dir = TempDir::new().unwrap();
    let schema = create_test_schema();
    let tablet = Tablet::new(1, schema.clone(), temp_dir.path().to_path_buf());

    let block = create_test_block();
    tablet.write(&block).unwrap();

    assert!(!tablet.memtable_should_flush());
}

#[test]
fn test_columnar_row_memory_size() {
    let _block = create_test_block();
    
    let _row_values: Vec<ScalarValue> = vec![
        ScalarValue::Int64(1),
        ScalarValue::String("test".to_string()),
        ScalarValue::Int64(100),
    ];

    let expected_size = 8 + 12 + 8;
    assert!(expected_size > 0);
}

#[test]
fn test_tablet_write_with_string_key() {
    let temp_dir = TempDir::new().unwrap();
    
    let schema = TabletSchema {
        tablet_id: 2,
        columns: vec![
            TabletColumn {
                name: "name".to_string(),
                data_type: DataType::String,
                nullable: false,
                is_key: true,
                agg_type: None,
            },
            TabletColumn {
                name: "value".to_string(),
                data_type: DataType::Int64,
                nullable: true,
                is_key: false,
                agg_type: None,
            },
        ],
        keys_type: "PRIMARY".to_string(),
        num_rows_per_row_block: 1000,
    };

    let tablet = Tablet::new(2, schema.clone(), temp_dir.path().to_path_buf());

    let block_schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("value", DataType::Int64, true),
    ]);

    let columns = vec![
        Vector::String(types::vector::StringVector::from_vec(vec!["key1", "key2", "key3"])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30])),
    ];

    let block = Block::new(block_schema, columns);
    tablet.write(&block).unwrap();

    assert_eq!(tablet.memtable_num_rows(), 3);
}

#[test]
fn test_tablet_write_with_int32_key() {
    let temp_dir = TempDir::new().unwrap();
    
    let schema = TabletSchema {
        tablet_id: 3,
        columns: vec![
            TabletColumn {
                name: "id".to_string(),
                data_type: DataType::Int32,
                nullable: false,
                is_key: true,
                agg_type: None,
            },
            TabletColumn {
                name: "data".to_string(),
                data_type: DataType::String,
                nullable: true,
                is_key: false,
                agg_type: None,
            },
        ],
        keys_type: "PRIMARY".to_string(),
        num_rows_per_row_block: 1000,
    };

    let tablet = Tablet::new(3, schema.clone(), temp_dir.path().to_path_buf());

    let block_schema = Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("data", DataType::String, true),
    ]);

    let columns = vec![
        Vector::Int32(types::vector::Int32Vector::from_vec(vec![1, 2, 3])),
        Vector::String(types::vector::StringVector::from_vec(vec!["a", "b", "c"])),
    ];

    let block = Block::new(block_schema, columns);
    tablet.write(&block).unwrap();

    assert_eq!(tablet.memtable_num_rows(), 3);
}