use be_execution::exec_node::{AggregateExecNode, SortExecNode, ScanExecNode, ExecNode, ExecutionPlan};
use types::{Block, Schema, Field, DataType, Vector, ScalarValue};

// Helper functions

fn create_block_with_columns(schema: Schema, columns: Vec<Vector>) -> Block {
    Block::new(schema, columns)
}

fn create_scan_node(table_name: &str, columns: &[&str]) -> ScanExecNode {
    ScanExecNode::new(
        table_name.to_string(),
        columns.iter().map(|c| c.to_string()).collect()
    )
}

// ===========================================================================
// P1 Optimization: Vectorized Aggregation Tests
// ===========================================================================

#[test]
fn test_aggregate_sum_batch() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Int64, true),
        Field::new("score", DataType::Float64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30, 40, 50])),
        Vector::Float64(types::vector::Float64Vector::from_vec(vec![1.5, 2.5, 3.5, 4.5, 5.5])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "value", "score"]);
    scan_node.data = Some(block);
    
    let mut agg_node = AggregateExecNode {
        group_by: vec![],
        aggregates: vec![("sum".to_string(), 1)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        returned: false,
    };

    agg_node.open().unwrap();
    let result = agg_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();
    assert_eq!(result_block.num_rows(), 1);
    assert_eq!(result_block.num_columns(), 1);

    let col = result_block.column(0).unwrap();
    assert_eq!(col.scalar_at(0), ScalarValue::Int64(150));
}

#[test]
fn test_aggregate_count_batch() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Int64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30, 40, 50])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "value"]);
    scan_node.data = Some(block);
    
    let mut agg_node = AggregateExecNode {
        group_by: vec![],
        aggregates: vec![("count".to_string(), 1)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        returned: false,
    };

    agg_node.open().unwrap();
    let result = agg_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();
    assert_eq!(result_block.num_rows(), 1);

    let col = result_block.column(0).unwrap();
    assert_eq!(col.scalar_at(0), ScalarValue::Int64(5));
}

#[test]
fn test_aggregate_min_max_batch() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Int64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30, 40, 50])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "value"]);
    scan_node.data = Some(block);
    
    let mut agg_node = AggregateExecNode {
        group_by: vec![],
        aggregates: vec![("min".to_string(), 1), ("max".to_string(), 1)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        returned: false,
    };

    agg_node.open().unwrap();
    let result = agg_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();
    assert_eq!(result_block.num_columns(), 2);

    let col0 = result_block.column(0).unwrap();
    assert_eq!(col0.scalar_at(0), ScalarValue::Int64(10));
    
    let col1 = result_block.column(1).unwrap();
    assert_eq!(col1.scalar_at(0), ScalarValue::Int64(50));
}

#[test]
fn test_aggregate_avg_batch() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Int64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "value"]);
    scan_node.data = Some(block);
    
    let mut agg_node = AggregateExecNode {
        group_by: vec![],
        aggregates: vec![("avg".to_string(), 1)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        returned: false,
    };

    agg_node.open().unwrap();
    let result = agg_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();

    let col = result_block.column(0).unwrap();
    assert_eq!(col.scalar_at(0), ScalarValue::Float64(20.0));
}

#[test]
fn test_aggregate_float_values() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("score", DataType::Float64, true),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::Float64(types::vector::Float64Vector::from_vec(vec![1.5, 2.5, 3.5, 4.5, 5.5])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "score"]);
    scan_node.data = Some(block);
    
    let mut agg_node = AggregateExecNode {
        group_by: vec![],
        aggregates: vec![("sum".to_string(), 1)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        returned: false,
    };

    agg_node.open().unwrap();
    let result = agg_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();

    let col = result_block.column(0).unwrap();
    assert_eq!(col.scalar_at(0), ScalarValue::Float64(17.5));
}

// ===========================================================================
// P1 Optimization: Vectorized Sorting Tests
// ===========================================================================

#[test]
fn test_sort_single_column_asc() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
        Field::new("score", DataType::Float64, false),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![3, 1, 4, 2, 5])),
        Vector::String(types::vector::StringVector::from_vec(vec!["charlie", "alice", "delta", "bob", "echo"])),
        Vector::Float64(types::vector::Float64Vector::from_vec(vec![85.0, 95.0, 75.0, 90.0, 80.0])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id", "name", "score"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![(0, true)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();
    assert_eq!(result_block.num_rows(), 5);

    let ids = result_block.column(0).unwrap();
    assert_eq!(ids.scalar_at(0), ScalarValue::Int64(1));
    assert_eq!(ids.scalar_at(1), ScalarValue::Int64(2));
    assert_eq!(ids.scalar_at(2), ScalarValue::Int64(3));
    assert_eq!(ids.scalar_at(3), ScalarValue::Int64(4));
    assert_eq!(ids.scalar_at(4), ScalarValue::Int64(5));
}

#[test]
fn test_sort_single_column_desc() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![3, 1, 4, 2, 5])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![(0, false)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();

    let ids = result_block.column(0).unwrap();
    assert_eq!(ids.scalar_at(0), ScalarValue::Int64(5));
    assert_eq!(ids.scalar_at(1), ScalarValue::Int64(4));
    assert_eq!(ids.scalar_at(2), ScalarValue::Int64(3));
    assert_eq!(ids.scalar_at(3), ScalarValue::Int64(2));
    assert_eq!(ids.scalar_at(4), ScalarValue::Int64(1));
}

#[test]
fn test_sort_float_column() {
    let schema = Schema::new(vec![
        Field::new("score", DataType::Float64, false),
    ]);

    let columns = vec![
        Vector::Float64(types::vector::Float64Vector::from_vec(vec![85.0, 95.0, 75.0, 90.0, 80.0])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["score"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![(0, true)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();

    let scores = result_block.column(0).unwrap();
    assert_eq!(scores.scalar_at(0), ScalarValue::Float64(75.0));
    assert_eq!(scores.scalar_at(1), ScalarValue::Float64(80.0));
    assert_eq!(scores.scalar_at(2), ScalarValue::Float64(85.0));
    assert_eq!(scores.scalar_at(3), ScalarValue::Float64(90.0));
    assert_eq!(scores.scalar_at(4), ScalarValue::Float64(95.0));
}

#[test]
fn test_sort_multi_column() {
    let schema = Schema::new(vec![
        Field::new("group", DataType::Int64, false),
        Field::new("value", DataType::Int64, false),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 1, 2, 1])),
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![30, 20, 10, 40, 50])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["group", "value"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![(0, true), (1, true)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();

    let groups = result_block.column(0).unwrap();
    let values = result_block.column(1).unwrap();

    assert_eq!(groups.scalar_at(0), ScalarValue::Int64(1));
    assert_eq!(values.scalar_at(0), ScalarValue::Int64(10));

    assert_eq!(groups.scalar_at(1), ScalarValue::Int64(1));
    assert_eq!(values.scalar_at(1), ScalarValue::Int64(30));

    assert_eq!(groups.scalar_at(2), ScalarValue::Int64(1));
    assert_eq!(values.scalar_at(2), ScalarValue::Int64(50));

    assert_eq!(groups.scalar_at(3), ScalarValue::Int64(2));
    assert_eq!(values.scalar_at(3), ScalarValue::Int64(20));

    assert_eq!(groups.scalar_at(4), ScalarValue::Int64(2));
    assert_eq!(values.scalar_at(4), ScalarValue::Int64(40));
}

#[test]
fn test_sort_empty_block() {
    let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);
    let block = Block::empty(schema);
    let mut scan_node = create_scan_node("test_table", &["id"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![(0, true)],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();
    // Empty block might return None or an empty block
    if let Some(result_block) = result {
        assert_eq!(result_block.num_rows(), 0);
    }
}

#[test]
fn test_sort_no_order_keys() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);

    let columns = vec![
        Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
    ];

    let block = create_block_with_columns(schema, columns);
    let mut scan_node = create_scan_node("test_table", &["id"]);
    scan_node.data = Some(block);
    
    let mut sort_node = SortExecNode {
        order_by: vec![],
        child: Box::new(ExecutionPlan::Scan(scan_node)),
        opened: false,
        buffered: vec![],
        returned: false,
    };

    sort_node.open().unwrap();
    let result = sort_node.get_next().unwrap();

    assert!(result.is_some());
    let result_block = result.unwrap();
    assert_eq!(result_block.num_rows(), 5);
}