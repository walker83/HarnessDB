use be_storage::compaction::{CompactionManager, CompactionTask, CompactionType};
use be_storage::engine::StorageEngine;
use be_storage::meta::{StorageType, TabletMeta};
use be_storage::rowset::{Rowset, RowsetMeta};
use be_storage::tablet::{Tablet, TabletColumn, TabletSchema};
use types::DataType;

use integration_tests::common;

// ===========================================================================
// Tablet creation tests
// ===========================================================================

#[test]
fn test_tablet_creation() {
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

    let tablet = Tablet::new(1, schema, std::env::temp_dir().join("rovisdb_test"));
    assert_eq!(tablet.tablet_id, 1);
    assert_eq!(tablet.schema.columns.len(), 2);
    assert_eq!(tablet.rowset_count(), 0);
    assert_eq!(tablet.max_version(), 0);
}

#[test]
fn test_tablet_add_rowset() {
    let schema = create_test_tablet_schema();
    let tablet = Tablet::new(1, schema, std::env::temp_dir().join("rovisdb_test"));

    let meta = RowsetMeta::new(1, 1, 1);
    tablet.add_rowset(Rowset::new(meta));
    assert_eq!(tablet.rowset_count(), 1);
    assert_eq!(tablet.max_version(), 1);

    let meta2 = RowsetMeta::new(2, 1, 2);
    tablet.add_rowset(Rowset::new(meta2));
    assert_eq!(tablet.rowset_count(), 2);
    assert_eq!(tablet.max_version(), 2);
}

// ===========================================================================
// StorageEngine tests
// ===========================================================================

fn create_test_engine() -> StorageEngine {
    let temp_dir = std::env::temp_dir().join("rovisdb_test_engine");
    StorageEngine::open(temp_dir).unwrap()
}

#[test]
fn test_engine_create_tablet() {
    let engine = create_test_engine();
    let schema = create_test_tablet_schema();

    engine.create_tablet(1, schema).unwrap();
    assert!(engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 1);
}

#[test]
fn test_engine_create_duplicate_tablet() {
    let engine = create_test_engine();
    let schema = create_test_tablet_schema();

    engine.create_tablet(1, schema.clone()).unwrap();
    let result = engine.create_tablet(1, schema);
    assert!(result.is_err());
}

#[test]
fn test_engine_drop_tablet() {
    let engine = create_test_engine();
    let schema = create_test_tablet_schema();

    engine.create_tablet(1, schema).unwrap();
    engine.drop_tablet(1).unwrap();
    assert!(!engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 0);
}

#[test]
fn test_engine_drop_nonexistent_tablet() {
    let engine = create_test_engine();
    let result = engine.drop_tablet(999);
    assert!(result.is_err());
}

#[test]
fn test_engine_multiple_tablets() {
    let engine = create_test_engine();

    for i in 1..=5 {
        let schema = create_test_tablet_schema_with_id(i);
        engine.create_tablet(i, schema).unwrap();
    }

    assert_eq!(engine.tablet_count(), 5);
    for i in 1..=5 {
        assert!(engine.get_tablet(i));
    }
}

#[test]
fn test_engine_create_and_drop_tablet() {
    let engine = create_test_engine();
    let schema = create_test_tablet_schema();

    engine.create_tablet(1, schema).unwrap();
    assert!(engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 1);

    engine.drop_tablet(1).unwrap();
    assert!(!engine.get_tablet(1));
    assert_eq!(engine.tablet_count(), 0);
}

// ===========================================================================
// Rowset tests
// ===========================================================================

#[test]
fn test_rowset_meta_creation() {
    let meta = RowsetMeta::new(1, 100, 1);
    assert_eq!(meta.rowset_id, 1);
    assert_eq!(meta.tablet_id, 100);
    assert_eq!(meta.version, 1);
    assert_eq!(meta.num_rows, 0);
    assert!(meta.empty);
}

#[test]
fn test_rowset_creation() {
    let meta = RowsetMeta::new(1, 100, 1);
    let rowset = Rowset::new(meta);
    assert_eq!(rowset.num_rows(), 0);
    assert_eq!(rowset.data_size(), 0);
    assert!(rowset.segments.is_empty());
}

#[test]
fn test_rowset_with_segments() {
    use be_storage::rowset::SegmentRef;

    let meta = RowsetMeta::new(1, 100, 1);
    let segments = vec![
        SegmentRef {
            segment_id: 1,
            path: "/tmp/seg_1.dat".to_string(),
            num_rows: 5000,
            size: 250000,
        },
        SegmentRef {
            segment_id: 2,
            path: "/tmp/seg_2.dat".to_string(),
            num_rows: 3000,
            size: 150000,
        },
        SegmentRef {
            segment_id: 3,
            path: "/tmp/seg_3.dat".to_string(),
            num_rows: 2000,
            size: 100000,
        },
    ];
    let rowset = Rowset::with_segments(meta, segments);

    assert_eq!(rowset.num_rows(), 10000);
    assert_eq!(rowset.data_size(), 500000);
    assert_eq!(rowset.segments.len(), 3);
}

// ===========================================================================
// Memtable flush simulation
// ===========================================================================

#[test]
fn test_memtable_flush_to_segment() {
    // Simulate: create tablet -> write data -> flush to rowset
    let schema = create_test_tablet_schema();
    let tablet = Tablet::new(1, schema, std::env::temp_dir().join("rovisdb_test"));

    // Initial state: no rowsets
    assert_eq!(tablet.rowset_count(), 0);

    // Simulate a flush: add a rowset
    tablet.add_rowset(Rowset::new(RowsetMeta::new(1, 1, 1)));
    assert_eq!(tablet.rowset_count(), 1);

    // Simulate more data and another flush
    tablet.add_rowset(Rowset::new(RowsetMeta::new(2, 1, 2)));
    assert_eq!(tablet.rowset_count(), 2);
    assert_eq!(tablet.max_version(), 2);
}

// ===========================================================================
// Segment read with column projection
// ===========================================================================

#[test]
fn test_segment_projection_with_block() {
    // Use Block operations to simulate segment column projection
    let block = common::create_employees_block();

    // Project only id and salary columns
    let projected = block.project(&[0, 3]);
    assert_eq!(projected.num_columns(), 2);
    assert_eq!(projected.schema().field(0).unwrap().name, "id");
    assert_eq!(projected.schema().field(1).unwrap().name, "salary");

    let row = projected.row(0);
    assert_eq!(row[0], types::ScalarValue::Int64(1));
    assert_eq!(row[1], types::ScalarValue::Float64(95000.0));
}

#[test]
fn test_segment_read_with_filter() {
    // Use Block operations to simulate segment read with predicate pushdown
    let block = common::create_employees_block();

    // Filter: id >= 3
    let id_col = block.column(0).unwrap();
    let mut selection = types::Bitmap::with_capacity(block.num_rows());
    for i in 0..block.num_rows() {
        let val = id_col.scalar_at(i);
        let pass = matches!(val, types::ScalarValue::Int64(v) if v >= 3);
        selection.push(pass);
    }

    let filtered = block.filter(&selection);
    assert_eq!(filtered.num_rows(), 3); // ids 3, 4, 5
}

// ===========================================================================
// Compaction tests
// ===========================================================================

#[test]
fn test_compaction_manager_creation() {
    let mut mgr = CompactionManager::new(4);
    // No tasks pending
    assert!(mgr.poll_task().is_none());
}

#[test]
fn test_compaction_submit_and_poll() {
    let mut mgr = CompactionManager::new(2);

    let task = CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1, 2, 3],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 1000000,
    };

    mgr.submit(task);
    let polled = mgr.poll_task();
    assert!(polled.is_some());
    assert_eq!(polled.unwrap().tablet_id, 1);
}

#[test]
fn test_compaction_max_concurrent() {
    let mut mgr = CompactionManager::new(2);

    mgr.submit(CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 100,
    });
    mgr.submit(CompactionTask {
        tablet_id: 2,
        rowset_ids: vec![2],
        compaction_type: CompactionType::Base,
        estimated_size: 200,
    });
    mgr.submit(CompactionTask {
        tablet_id: 3,
        rowset_ids: vec![3],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 300,
    });

    // Should be able to poll 2 (max_concurrent)
    assert!(mgr.poll_task().is_some());
    assert!(mgr.poll_task().is_some());
    // Third should fail (running == max_concurrent)
    assert!(mgr.poll_task().is_none());

    // Complete one, should be able to poll again
    mgr.complete();
    let task = mgr.poll_task();
    assert!(task.is_some());
    // Any of the remaining tasks could be returned (1 or 3 since 2 was polled first)
    let task_id = task.unwrap().tablet_id;
    assert!(task_id == 1 || task_id == 3, "expected 1 or 3, got {}", task_id);
}

#[test]
fn test_compaction_priority_ordering() {
    let mut mgr = CompactionManager::new(4);

    // Submit tasks with different sizes (larger = higher priority in BinaryHeap)
    mgr.submit(CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 100,
    });
    mgr.submit(CompactionTask {
        tablet_id: 2,
        rowset_ids: vec![2],
        compaction_type: CompactionType::Base,
        estimated_size: 500,
    });
    mgr.submit(CompactionTask {
        tablet_id: 3,
        rowset_ids: vec![3],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 300,
    });

    // Should get the largest first (priority queue)
    let task = mgr.poll_task().unwrap();
    assert_eq!(task.tablet_id, 2); // size 500
    assert_eq!(task.estimated_size, 500);
}

#[test]
fn test_compaction_task_types() {
    let mut mgr = CompactionManager::new(2);

    mgr.submit(CompactionTask {
        tablet_id: 1,
        rowset_ids: vec![1, 2],
        compaction_type: CompactionType::Cumulative,
        estimated_size: 100,
    });
    mgr.submit(CompactionTask {
        tablet_id: 2,
        rowset_ids: vec![3, 4, 5],
        compaction_type: CompactionType::Base,
        estimated_size: 200,
    });

    let task1 = mgr.poll_task().unwrap();
    assert_eq!(task1.compaction_type, CompactionType::Base);

    let task2 = mgr.poll_task().unwrap();
    assert_eq!(task2.compaction_type, CompactionType::Cumulative);
}

// ===========================================================================
// TabletMeta tests
// ===========================================================================

#[test]
fn test_tablet_meta_creation() {
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

// ===========================================================================
// Codec tests (from be-storage)
// ===========================================================================

#[test]
fn test_storage_codec_rle() {
    let values = vec![1i64, 1, 1, 2, 2, 3, 3, 3, 3];
    let encoded = be_storage::codec::rle_encode_i64(&values);
    assert_eq!(encoded, vec![(1, 3), (2, 2), (3, 4)]);

    let decoded = be_storage::codec::rle_decode_i64(&encoded);
    assert_eq!(decoded, values);
}

#[test]
fn test_storage_codec_bit_pack() {
    let values = vec![100i64, 101, 102, 103, 100, 105];
    let (min_val, bits, packed) = be_storage::codec::bit_pack_i64(&values);
    let decoded = be_storage::codec::bit_unpack_i64(min_val, bits, &packed, values.len());
    assert_eq!(decoded, values);
}

#[test]
fn test_storage_codec_dictionary() {
    let strings: Vec<Option<&str>> = vec![Some("hello"), Some("world"), Some("hello"), None];
    let (dict, indices) = be_storage::codec::dictionary_encode(&strings);
    let decoded = be_storage::codec::dictionary_decode(&dict, &indices);
    assert_eq!(decoded[0], Some("hello".to_string()));
    assert_eq!(decoded[1], Some("world".to_string()));
    assert_eq!(decoded[2], Some("hello".to_string()));
    assert_eq!(decoded[3], None);
}

#[test]
fn test_storage_codec_lz4() {
    let data = b"hello world this is a test of lz4 compression in rovisdb storage engine";
    let compressed = be_storage::codec::lz4_compress(data);
    let decompressed = be_storage::codec::lz4_decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data.to_vec());
}

#[test]
fn test_storage_encoding_choice() {
    use be_storage::codec::EncodingType;

    // String with low cardinality -> Dictionary
    assert_eq!(
        be_storage::codec::choose_encoding(&DataType::String, 0.05, false),
        EncodingType::Dictionary
    );

    // String with high cardinality -> Raw
    assert_eq!(
        be_storage::codec::choose_encoding(&DataType::String, 0.5, false),
        EncodingType::Raw
    );

    // Sorted -> RunLength
    assert_eq!(
        be_storage::codec::choose_encoding(&DataType::Int64, 0.5, true),
        EncodingType::RunLength
    );

    // Int64 unsorted -> BitPacked
    assert_eq!(
        be_storage::codec::choose_encoding(&DataType::Int64, 0.5, false),
        EncodingType::BitPacked
    );
}

// ===========================================================================
// Index tests (ZoneMap, BloomFilter)
// ===========================================================================

#[test]
fn test_zone_map_build() {
    let values = vec![
        types::ScalarValue::Int64(10),
        types::ScalarValue::Int64(20),
        types::ScalarValue::Null,
        types::ScalarValue::Int64(5),
    ];
    let zm = be_storage::index::ZoneMap::build(&values);
    assert_eq!(zm.null_count, 1);
    assert_eq!(zm.num_rows, 4);
}

#[test]
fn test_bloom_filter_basic() {
    let mut bf = be_storage::index::BloomFilter::new(100, 0.01);
    bf.insert(b"hello");
    bf.insert(b"world");
    assert!(bf.may_contain(b"hello"));
    assert!(bf.may_contain(b"world"));
    assert!(!bf.may_contain(b"xyzzy_not_present_at_all"));
    assert_eq!(bf.len(), 2);
}

#[test]
fn test_predicate_eval() {
    use be_storage::index::{eval_predicate, PredicateOp};

    let val = types::ScalarValue::Int64(50);
    assert!(eval_predicate(&PredicateOp::Eq, &val, &types::ScalarValue::Int64(50)));
    assert!(!eval_predicate(&PredicateOp::Eq, &val, &types::ScalarValue::Int64(51)));
    assert!(eval_predicate(&PredicateOp::Lt, &val, &types::ScalarValue::Int64(100)));
    assert!(eval_predicate(&PredicateOp::Gt, &val, &types::ScalarValue::Int64(25)));
    assert!(eval_predicate(&PredicateOp::Le, &val, &types::ScalarValue::Int64(50)));
    assert!(eval_predicate(&PredicateOp::Ge, &val, &types::ScalarValue::Int64(50)));

    // Null values never match
    assert!(!eval_predicate(&PredicateOp::Eq, &types::ScalarValue::Null, &types::ScalarValue::Int64(50)));
}

// ===========================================================================
// Helpers
// ===========================================================================

fn create_test_tablet_schema() -> TabletSchema {
    TabletSchema {
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
    }
}

fn create_test_tablet_schema_with_id(id: u64) -> TabletSchema {
    let mut schema = create_test_tablet_schema();
    schema.tablet_id = id;
    schema
}
