use be_storage::compaction::{CompactionManager, CompactionTask, CompactionType};
use be_storage::engine::StorageEngine;
use be_storage::meta::{StorageType, TabletMeta};
use be_storage::rowset::{Rowset, RowsetMeta, SegmentRef};
use be_storage::tablet::{Tablet, TabletColumn, TabletSchema, TabletConfig};
use types::DataType;
use types::vector::{Float64Vector, Int64Vector, StringVector};
use types::{Block, Field, Schema, ScalarValue, Vector};

use integration_tests::common;

// ===========================================================================
// Helper: create a test tablet schema
// ===========================================================================

fn test_tablet_schema() -> TabletSchema {
    TabletSchema {
        tablet_id: 0,
        columns: vec![
            TabletColumn { name: "id".into(), data_type: DataType::Int64, nullable: false, is_key: true, agg_type: None },
            TabletColumn { name: "name".into(), data_type: DataType::String, nullable: false, is_key: false, agg_type: None },
            TabletColumn { name: "value".into(), data_type: DataType::Float64, nullable: true, is_key: false, agg_type: None },
        ],
        keys_type: "DUP".into(),
        num_rows_per_row_block: 1024,
    }
}

fn test_tablet_schema_with_id(id: u64) -> TabletSchema {
    let mut s = test_tablet_schema();
    s.tablet_id = id;
    s
}

fn test_data_block() -> Block {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
        Field::new("value", DataType::Float64, true),
    ]);
    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::String(StringVector::from_vec(vec!["a", "b", "c", "d", "e"])),
        Vector::Float64(Float64Vector::from_nullable_vec(vec![
            Some(10.0), Some(20.0), Some(30.0), Some(40.0), Some(50.0),
        ])),
    ];
    Block::new(schema, columns)
}

// ===========================================================================
// 4.1 Data write and read
// ===========================================================================

#[test]
fn test_tablet_write_read_lifecycle() {
    let dir = std::env::temp_dir().join("rorisdb_test_write_read");
    let _ = std::fs::remove_dir_all(&dir);
    let schema = test_tablet_schema();
    let tablet = Tablet::new(1, schema, TabletConfig::new(dir));

    assert_eq!(tablet.tablet_id, 1);
    assert_eq!(tablet.rowset_count(), 0);

    // Add first rowset (simulate flush)
    let meta = RowsetMeta::new(1, 1, 1);
    tablet.add_rowset(Rowset::new(meta));
    assert_eq!(tablet.rowset_count(), 1);
    assert_eq!(tablet.max_version(), 1);
}

#[test]
fn test_multiple_rowsets() {
    let dir = std::env::temp_dir().join("rorisdb_test_multi_rowset");
    let _ = std::fs::remove_dir_all(&dir);
    let schema = test_tablet_schema();
    let tablet = Tablet::new(1, schema, TabletConfig::new(dir));

    for i in 1..=5 {
        let meta = RowsetMeta::new(i, 1, i as u64);
        tablet.add_rowset(Rowset::new(meta));
    }
    assert_eq!(tablet.rowset_count(), 5);
    assert_eq!(tablet.max_version(), 5);
}

#[test]
fn test_column_projection() {
    let block = test_data_block();
    assert_eq!(block.num_rows(), 5);
    assert_eq!(block.num_columns(), 3);

    // Project only id and value
    let projected = block.project(&[0, 2]);
    assert_eq!(projected.num_columns(), 2);
    assert_eq!(projected.schema().field(0).unwrap().name, "id");
    assert_eq!(projected.schema().field(1).unwrap().name, "value");

    let row = projected.row(0);
    assert_eq!(row[0], ScalarValue::Int64(1));
    assert_eq!(row[1], ScalarValue::Float64(10.0));
}

#[test]
fn test_rowset_with_segment_data() {
    let meta = RowsetMeta::new(1, 1, 1);
    let segments = vec![
        SegmentRef { segment_id: 1, path: "/tmp/seg1.dat".into(), num_rows: 100, size: 5000 },
        SegmentRef { segment_id: 2, path: "/tmp/seg2.dat".into(), num_rows: 200, size: 10000 },
        SegmentRef { segment_id: 3, path: "/tmp/seg3.dat".into(), num_rows: 150, size: 7500 },
    ];
    let rowset = Rowset::with_segments(meta, segments);

    assert_eq!(rowset.num_rows(), 450);
    assert_eq!(rowset.data_size(), 22500);
    assert_eq!(rowset.segments.len(), 3);
}

// ===========================================================================
// 4.2 Filter / predicate pushdown simulation
// ===========================================================================

#[test]
fn test_read_with_filter_eq() {
    let block = test_data_block();
    let id_col = block.column(0).unwrap();
    let mut sel = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let pass = matches!(id_col.scalar_at(i), ScalarValue::Int64(v) if v == 3);
        sel.push(pass);
    }
    let filtered = block.filter(&sel);
    assert_eq!(filtered.num_rows(), 1);
    assert_eq!(filtered.row(0)[0], ScalarValue::Int64(3));
}

#[test]
fn test_read_with_filter_range() {
    let block = test_data_block();
    let value_col = block.column(2).unwrap();
    let mut sel = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let pass = matches!(value_col.scalar_at(i), ScalarValue::Float64(v) if v >= 20.0 && v <= 40.0);
        sel.push(pass);
    }
    let filtered = block.filter(&sel);
    assert_eq!(filtered.num_rows(), 3); // 20, 30, 40
}

#[test]
fn test_read_with_filter_null_handling() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Float64, true),
    ]);
    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::Float64(Float64Vector::from_nullable_vec(vec![Some(10.0), None, Some(30.0)])),
    ];
    let block = Block::new(schema, columns);

    let value_col = block.column(1).unwrap();
    let mut sel = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let pass = !matches!(value_col.scalar_at(i), ScalarValue::Null);
        sel.push(pass);
    }
    let filtered = block.filter(&sel);
    assert_eq!(filtered.num_rows(), 2); // Rows 1 and 3 (not null)
}

// ===========================================================================
// 4.3 Compaction
// ===========================================================================

#[test]
fn test_cumulative_compaction() {
    let mut mgr = CompactionManager::new(4);

    // Simulate multiple small rowsets needing compaction
    mgr.submit(CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1, 2, 3],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 100000,
    });

    let task = mgr.poll_task().unwrap();
    assert_eq!(task.tablet_id, 1);
    assert_eq!(task.rowset_ids.len(), 3);
    assert_eq!(task.compaction_type, CompactionType::Cumulative);

    mgr.complete();
}

#[test]
fn test_base_compaction() {
    let mut mgr = CompactionManager::new(4);

    mgr.submit(CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1, 2, 3, 4, 5],
        compaction_type: CompactionType::Base,
        estimated_size: 5000000,
    });

    let task = mgr.poll_task().unwrap();
    assert_eq!(task.compaction_type, CompactionType::Base);
    mgr.complete();
}

#[test]
fn test_compaction_concurrency_limit() {
    let mut mgr = CompactionManager::new(2);

    mgr.submit(CompactionTask { tablet_id: 1, rowset_ids: vec![1], compaction_type: CompactionType::Cumulative, estimated_size: 100 });
    mgr.submit(CompactionTask { tablet_id: 2, rowset_ids: vec![2], compaction_type: CompactionType::Cumulative, estimated_size: 200 });
    mgr.submit(CompactionTask { tablet_id: 3, rowset_ids: vec![3], compaction_type: CompactionType::Base, estimated_size: 300 });

    // Max 2 concurrent
    assert!(mgr.poll_task().is_some());
    assert!(mgr.poll_task().is_some());
    assert!(mgr.poll_task().is_none()); // blocked

    mgr.complete(); // Free a slot
    assert!(mgr.poll_task().is_some()); // Now can run
    mgr.complete();
}

#[test]
fn test_compaction_priority() {
    let mut mgr = CompactionManager::new(4);

    mgr.submit(CompactionTask { tablet_id: 1, rowset_ids: vec![1], compaction_type: CompactionType::Cumulative, estimated_size: 100 });
    mgr.submit(CompactionTask { tablet_id: 2, rowset_ids: vec![2], compaction_type: CompactionType::Base, estimated_size: 500 });
    mgr.submit(CompactionTask { tablet_id: 3, rowset_ids: vec![3], compaction_type: CompactionType::Cumulative, estimated_size: 300 });

    // Should get largest first (priority queue)
    let t1 = mgr.poll_task().unwrap();
    assert_eq!(t1.tablet_id, 2); // size 500
    let t2 = mgr.poll_task().unwrap();
    assert_eq!(t2.tablet_id, 3); // size 300
    mgr.complete();
    mgr.complete();
    mgr.complete();
}

// ===========================================================================
// 4.4 Index verification
// ===========================================================================

#[test]
fn test_zone_map_build_and_query() {
    let values = vec![
        ScalarValue::Int64(10),
        ScalarValue::Int64(20),
        ScalarValue::Null,
        ScalarValue::Int64(5),
        ScalarValue::Int64(100),
    ];
    let zm = be_storage::index::ZoneMap::build(&values);
    assert_eq!(zm.null_count, 1);
    assert_eq!(zm.num_rows, 5);
}

#[test]
fn test_bloom_filter_insert_and_lookup() {
    let mut bf = be_storage::index::BloomFilter::new(1000, 0.01);

    for i in 0i64..100 {
        bf.insert(&i.to_le_bytes());
    }

    // All inserted should be found
    for i in 0i64..100 {
        assert!(bf.may_contain(&i.to_le_bytes()));
    }

    // Non-existent values (high confidence due to low FPR)
    assert!(!bf.may_contain(b"nonexistent_value_999999"));
    assert_eq!(bf.len(), 100);
}

#[test]
fn test_bloom_filter_string_lookup() {
    let mut bf = be_storage::index::BloomFilter::new(100, 0.01);
    bf.insert(b"Engineering");
    bf.insert(b"Marketing");
    bf.insert(b"Sales");

    assert!(bf.may_contain(b"Engineering"));
    assert!(bf.may_contain(b"Marketing"));
    assert!(bf.may_contain(b"Sales"));
    assert!(!bf.may_contain(b"HR_department_nonexistent"));
}

#[test]
fn test_predicate_evaluation() {
    use be_storage::index::{eval_predicate, PredicateOp};

    let val = ScalarValue::Int64(50);
    assert!(eval_predicate(&PredicateOp::Eq, &val, &ScalarValue::Int64(50)));
    assert!(!eval_predicate(&PredicateOp::Eq, &val, &ScalarValue::Int64(51)));
    assert!(eval_predicate(&PredicateOp::Lt, &val, &ScalarValue::Int64(100)));
    assert!(eval_predicate(&PredicateOp::Gt, &val, &ScalarValue::Int64(25)));
    assert!(eval_predicate(&PredicateOp::Le, &val, &ScalarValue::Int64(50)));
    assert!(eval_predicate(&PredicateOp::Ge, &val, &ScalarValue::Int64(50)));
    assert!(!eval_predicate(&PredicateOp::Eq, &ScalarValue::Null, &ScalarValue::Int64(50)));
}

// ===========================================================================
// 4.5 Compression / Codec
// ===========================================================================

#[test]
fn test_lz4_compress_decompress() {
    let data = b"hello world this is a test of lz4 compression in rorisdb storage engine";
    let compressed = be_storage::codec::lz4_compress(data);
    let decompressed = be_storage::codec::lz4_decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data.to_vec());
    // Note: small data may not compress (LZ4 overhead can exceed savings)
}

#[test]
fn test_lz4_large_data() {
    let data: Vec<u8> = (0..10000).flat_map(|i| i.to_string().into_bytes()).collect();
    let compressed = be_storage::codec::lz4_compress(&data);
    let decompressed = be_storage::codec::lz4_decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_rle_codec() {
    let values = vec![1i64, 1, 1, 2, 2, 3, 3, 3, 3, 4, 4, 4];
    let encoded = be_storage::codec::rle_encode_i64(&values);
    assert_eq!(encoded, vec![(1, 3), (2, 2), (3, 4), (4, 3)]);
    let decoded = be_storage::codec::rle_decode_i64(&encoded);
    assert_eq!(decoded, values);
}

#[test]
fn test_bit_pack_codec() {
    let values = vec![100i64, 101, 102, 103, 100, 105, 110, 99];
    let (min_val, bits, packed) = be_storage::codec::bit_pack_i64(&values);
    let decoded = be_storage::codec::bit_unpack_i64(min_val, bits, &packed, values.len());
    assert_eq!(decoded, values);
}

#[test]
fn test_dictionary_codec() {
    let strings: Vec<Option<&str>> = vec![Some("hello"), Some("world"), Some("hello"), None, Some("world")];
    let (dict, indices) = be_storage::codec::dictionary_encode(&strings);
    let decoded = be_storage::codec::dictionary_decode(&dict, &indices);
    assert_eq!(decoded[0], Some("hello".to_string()));
    assert_eq!(decoded[1], Some("world".to_string()));
    assert_eq!(decoded[2], Some("hello".to_string()));
    assert_eq!(decoded[3], None);
}

#[test]
fn test_encoding_choice() {
    use be_storage::codec::EncodingType;

    // Low cardinality string -> Dictionary
    assert_eq!(be_storage::codec::choose_encoding(&DataType::String, 0.05, false), EncodingType::Dictionary);
    // High cardinality string -> Raw
    assert_eq!(be_storage::codec::choose_encoding(&DataType::String, 0.8, false), EncodingType::Raw);
    // Sorted int -> RunLength
    assert_eq!(be_storage::codec::choose_encoding(&DataType::Int64, 0.5, true), EncodingType::RunLength);
    // Unsorted int -> BitPacked
    assert_eq!(be_storage::codec::choose_encoding(&DataType::Int64, 0.5, false), EncodingType::BitPacked);
}

// ===========================================================================
// 4.6 Storage Engine CRUD
// ===========================================================================

#[test]
fn test_engine_tablet_crud() {
    let dir = std::env::temp_dir().join("rorisdb_test_engine_crud");
    let _ = std::fs::remove_dir_all(&dir);
    let engine = StorageEngine::open(&dir).unwrap();

    engine.create_tablet(1, test_tablet_schema()).unwrap();
    assert!(engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 1);

    engine.drop_tablet(1).unwrap();
    assert!(!engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 0);
}

#[test]
fn test_engine_multiple_tablets() {
    let dir = std::env::temp_dir().join("rorisdb_test_engine_multi");
    let _ = std::fs::remove_dir_all(&dir);
    let engine = StorageEngine::open(&dir).unwrap();

    for i in 1..=10 {
        engine.create_tablet(i, test_tablet_schema_with_id(i)).unwrap();
    }
    assert_eq!(engine.tablet_count(), 10);
    for i in 1..=10 {
        assert!(engine.get_tablet(i));
    }
}

#[test]
fn test_engine_duplicate_tablet_error() {
    let dir = std::env::temp_dir().join("rorisdb_test_engine_dup");
    let _ = std::fs::remove_dir_all(&dir);
    let engine = StorageEngine::open(&dir).unwrap();

    engine.create_tablet(1, test_tablet_schema()).unwrap();
    let result = engine.create_tablet(1, test_tablet_schema());
    assert!(result.is_err());
}

#[test]
fn test_engine_drop_nonexistent_error() {
    let dir = std::env::temp_dir().join("rorisdb_test_engine_drop_ne");
    let _ = std::fs::remove_dir_all(&dir);
    let engine = StorageEngine::open(&dir).unwrap();

    let result = engine.drop_tablet(999);
    assert!(result.is_err());
}

// ===========================================================================
// 4.7 TabletMeta
// ===========================================================================

#[test]
fn test_tablet_meta() {
    let meta = TabletMeta {
        tablet_id: 1,
        table_id: 100,
        partition_id: 10,
        index_id: 5,
        schema_version: 1,
        min_version: 0,
        max_version: 5,
        persistent_index: false,
        storage_type: StorageType::Local,
    };
    assert_eq!(meta.tablet_id, 1);
    assert_eq!(meta.table_id, 100);
    assert_eq!(meta.storage_type, StorageType::Local);
}
