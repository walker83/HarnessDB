use types::{
    Bitmap, Block, DataType, Field, Int32Vector, Int64Vector, Float64Vector,
    BooleanVector, DateVector, StringVector, Schema, ScalarValue, Vector,
    Int8Vector, Int16Vector, Int128Vector, Float32Vector,
};

// ===========================================================================
// Int32Vector tests
// ===========================================================================

#[test]
fn test_int32_vector_new_and_from_vec() {
    let v = Int32Vector::new();
    assert!(v.is_empty());
    assert_eq!(v.len(), 0);

    let v = Int32Vector::from_vec(vec![1, 2, 3, 4, 5]);
    assert_eq!(v.len(), 5);
    assert!(!v.is_empty());
    assert_eq!(v.get(0), Some(1));
    assert_eq!(v.get(4), Some(5));
}

#[test]
fn test_int32_vector_push_and_get() {
    let mut v = Int32Vector::new();
    v.push(Some(10));
    v.push(Some(20));
    v.push(None);
    v.push(Some(40));

    assert_eq!(v.len(), 4);
    assert_eq!(v.get(0), Some(10));
    assert_eq!(v.get(1), Some(20));
    assert_eq!(v.get(2), None);
    assert_eq!(v.get(3), Some(40));
}

#[test]
fn test_int32_vector_null_handling() {
    let v = Int32Vector::from_nullable_vec(vec![Some(1), None, Some(3), None, Some(5)]);
    assert_eq!(v.len(), 5);
    assert_eq!(v.null_count(), 2);
    assert_eq!(v.get(0), Some(1));
    assert_eq!(v.get(1), None);
    assert_eq!(v.get(2), Some(3));
    assert_eq!(v.get(3), None);
    assert_eq!(v.get(4), Some(5));

    // get_checked returns raw data even for nulls (uses default 0)
    assert_eq!(v.get_checked(1), 0);
    assert_eq!(v.get_checked(3), 0);
}

#[test]
fn test_int32_vector_data_and_validity() {
    let v = Int32Vector::from_vec(vec![10, 20, 30]);
    assert_eq!(v.data(), &[10, 20, 30]);
    assert_eq!(v.validity().len(), 3);
    assert_eq!(v.validity().null_count(), 0);
}

#[test]
fn test_int32_vector_filter() {
    let v = Int32Vector::from_vec(vec![10, 20, 30, 40, 50]);
    let selection = Bitmap::from_bools(&[true, false, true, false, true]);
    let filtered = v.filter(&selection);

    assert_eq!(filtered.len(), 3);
    assert_eq!(filtered.get(0), Some(10));
    assert_eq!(filtered.get(1), Some(30));
    assert_eq!(filtered.get(2), Some(50));
}

#[test]
fn test_int32_vector_slice() {
    let v = Int32Vector::from_vec(vec![10, 20, 30, 40, 50]);
    let sliced = v.slice(1, 3);

    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.get(0), Some(20));
    assert_eq!(sliced.get(1), Some(30));
    assert_eq!(sliced.get(2), Some(40));
}

#[test]
fn test_int32_vector_slice_with_nulls() {
    let v = Int32Vector::from_nullable_vec(vec![Some(1), None, Some(3), Some(4), None]);
    let sliced = v.slice(1, 3);

    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.get(0), None);
    assert_eq!(sliced.get(1), Some(3));
    assert_eq!(sliced.get(2), Some(4));
}

#[test]
fn test_int32_vector_append() {
    let mut v1 = Int32Vector::from_vec(vec![1, 2]);
    let v2 = Int32Vector::from_vec(vec![3, 4, 5]);
    v1.append(&v2);

    assert_eq!(v1.len(), 5);
    assert_eq!(v1.get(0), Some(1));
    assert_eq!(v1.get(4), Some(5));
}

#[test]
fn test_int32_vector_slice_overflow() {
    let v = Int32Vector::from_vec(vec![1, 2, 3]);
    // Slice requesting more than available should clamp
    let sliced = v.slice(2, 10);
    assert_eq!(sliced.len(), 1);
    assert_eq!(sliced.get(0), Some(3));
}

// ===========================================================================
// Int64Vector tests
// ===========================================================================

#[test]
fn test_int64_vector_operations() {
    let mut v = Int64Vector::new();
    v.push(Some(100_i64));
    v.push(None);
    v.push(Some(300_i64));

    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some(100));
    assert_eq!(v.get(1), None);
    assert_eq!(v.get(2), Some(300));
}

#[test]
fn test_int64_vector_from_nullable_vec() {
    let v = Int64Vector::from_nullable_vec(vec![Some(1), None, Some(3)]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
}

// ===========================================================================
// Float64Vector tests
// ===========================================================================

#[test]
fn test_float64_vector_operations() {
    let mut v = Float64Vector::new();
    v.push(Some(1.5));
    v.push(Some(2.5));
    v.push(None);

    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some(1.5));
    assert_eq!(v.get(1), Some(2.5));
    assert_eq!(v.get(2), None);
}

#[test]
fn test_float64_vector_filter() {
    let v = Float64Vector::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let sel = Bitmap::from_bools(&[false, true, false, true, false]);
    let filtered = v.filter(&sel);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered.get(0), Some(2.0));
    assert_eq!(filtered.get(1), Some(4.0));
}

// ===========================================================================
// BooleanVector tests
// ===========================================================================

#[test]
fn test_boolean_vector_operations() {
    let mut v = BooleanVector::new();
    v.push(Some(true));
    v.push(Some(false));
    v.push(None);
    v.push(Some(true));

    assert_eq!(v.len(), 4);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some(true));
    assert_eq!(v.get(1), Some(false));
    assert_eq!(v.get(2), None);
    assert_eq!(v.get(3), Some(true));
}

#[test]
fn test_boolean_vector_from_vec() {
    let v = BooleanVector::from_vec(vec![true, false, true]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 0);
}

// ===========================================================================
// DateVector tests
// ===========================================================================

#[test]
fn test_date_vector_operations() {
    // Dates stored as i32 (days since epoch)
    let mut v = DateVector::new();
    v.push(Some(0));       // epoch
    v.push(Some(18628));   // ~51 years
    v.push(None);

    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some(0));
    assert_eq!(v.get(1), Some(18628));
    assert_eq!(v.get(2), None);
}

// ===========================================================================
// StringVector tests
// ===========================================================================

#[test]
fn test_string_vector_from_vec() {
    let v = StringVector::from_vec(vec!["hello", "world", "test"]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.get(0), Some("hello"));
    assert_eq!(v.get(1), Some("world"));
    assert_eq!(v.get(2), Some("test"));
}

#[test]
fn test_string_vector_push() {
    let mut v = StringVector::new();
    v.push(Some("foo"));
    v.push(None);
    v.push(Some("bar"));

    assert_eq!(v.len(), 3);
    assert_eq!(v.get(0), Some("foo"));
    assert_eq!(v.get(1), None);
    assert_eq!(v.get(2), Some("bar"));
}

#[test]
fn test_string_vector_null_handling() {
    let v = StringVector::from_option_vec(vec![
        Some("a".to_string()),
        None,
        Some("c".to_string()),
    ]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some("a"));
    assert_eq!(v.get(1), None);
    assert_eq!(v.get(2), Some("c"));
}

#[test]
fn test_string_vector_filter() {
    let v = StringVector::from_vec(vec!["a", "b", "c", "d"]);
    let sel = Bitmap::from_bools(&[true, false, true, false]);
    let filtered = v.filter(&sel);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered.get(0), Some("a"));
    assert_eq!(filtered.get(1), Some("c"));
}

#[test]
fn test_string_vector_slice() {
    let v = StringVector::from_vec(vec!["alpha", "beta", "gamma", "delta"]);
    let sliced = v.slice(1, 2);
    assert_eq!(sliced.len(), 2);
    assert_eq!(sliced.get(0), Some("beta"));
    assert_eq!(sliced.get(1), Some("gamma"));
}

#[test]
fn test_string_vector_empty_string() {
    let v = StringVector::from_vec(vec!["", "hello", ""]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.get(0), Some(""));
    assert_eq!(v.get(1), Some("hello"));
    assert_eq!(v.get(2), Some(""));
}

// ===========================================================================
// Bitmap tests
// ===========================================================================

#[test]
fn test_bitmap_new_and_push() {
    let mut bm = Bitmap::new();
    assert!(bm.is_empty());
    assert_eq!(bm.len(), 0);

    bm.push(true);
    bm.push(false);
    bm.push(true);

    assert_eq!(bm.len(), 3);
    assert!(!bm.is_empty());
    assert!(bm.get(0));
    assert!(!bm.get(1));
    assert!(bm.get(2));
}

#[test]
fn test_bitmap_from_bools() {
    let bm = Bitmap::from_bools(&[true, false, true, true, false]);
    assert_eq!(bm.len(), 5);
    assert_eq!(bm.set_count(), 3);
    assert_eq!(bm.null_count(), 2);
}

#[test]
fn test_bitmap_all_set() {
    let bm = Bitmap::all_set(10);
    assert_eq!(bm.len(), 10);
    assert_eq!(bm.set_count(), 10);
    assert_eq!(bm.null_count(), 0);
    for i in 0..10 {
        assert!(bm.get(i));
    }
}

#[test]
fn test_bitmap_all_set_non_word_aligned() {
    let bm = Bitmap::all_set(65);
    assert_eq!(bm.len(), 65);
    assert_eq!(bm.set_count(), 65);
    // Bit 65 should not be set (out of bounds returns false)
    assert!(!bm.get(65));
}

#[test]
fn test_bitmap_set_and_get() {
    let mut bm = Bitmap::with_capacity(10);
    for i in 0..10 {
        bm.push(false);
    }
    assert_eq!(bm.set_count(), 0);

    bm.set(3, true);
    bm.set(7, true);
    assert!(bm.get(3));
    assert!(bm.get(7));
    assert!(!bm.get(0));
    assert!(!bm.get(9));

    bm.set(3, false);
    assert!(!bm.get(3));
}

#[test]
fn test_bitmap_is_valid() {
    let bm = Bitmap::from_bools(&[true, false, true]);
    assert!(bm.is_valid(0));
    assert!(!bm.is_valid(1));
    assert!(bm.is_valid(2));
}

#[test]
fn test_bitmap_get_out_of_bounds() {
    let bm = Bitmap::from_bools(&[true]);
    assert!(!bm.get(100));
}

#[test]
fn test_bitmap_bitand() {
    let bm1 = Bitmap::from_bools(&[true, true, false, true]);
    let bm2 = Bitmap::from_bools(&[true, false, false, true]);
    let result = &bm1 & &bm2;

    assert_eq!(result.len(), 4);
    assert!(result.get(0));
    assert!(!result.get(1));
    assert!(!result.get(2));
    assert!(result.get(3));
}

#[test]
fn test_bitmap_bitor() {
    let bm1 = Bitmap::from_bools(&[true, false, false, false]);
    let bm2 = Bitmap::from_bools(&[false, true, false, true]);
    let result = &bm1 | &bm2;

    assert_eq!(result.len(), 4);
    assert!(result.get(0));
    assert!(result.get(1));
    assert!(!result.get(2));
    assert!(result.get(3));
}

#[test]
fn test_bitmap_not() {
    let bm = Bitmap::from_bools(&[true, false, true]);
    let result = !&bm;

    assert!(!result.get(0));
    assert!(result.get(1));
    assert!(!result.get(2));
}

#[test]
fn test_bitmap_large() {
    // Test with more than 64 bits to exercise multi-word logic
    let bools: Vec<bool> = (0..200).map(|i| i % 3 == 0).collect();
    let bm = Bitmap::from_bools(&bools);

    assert_eq!(bm.len(), 200);
    for i in 0..200 {
        assert_eq!(bm.get(i), i % 3 == 0);
    }
}

#[test]
fn test_bitmap_with_capacity() {
    let bm = Bitmap::with_capacity(1000);
    assert_eq!(bm.len(), 0);
    assert!(bm.is_empty());
}

// ===========================================================================
// Schema tests
// ===========================================================================

#[test]
fn test_schema_new_and_fields() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
        Field::new("score", DataType::Float64, true),
    ]);

    assert_eq!(schema.num_fields(), 3);
    assert_eq!(schema.names(), vec!["id", "name", "score"]);
}

#[test]
fn test_schema_field_lookup() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
    ]);

    assert_eq!(schema.index_of("id"), Some(0));
    assert_eq!(schema.index_of("name"), Some(1));
    assert_eq!(schema.index_of("missing"), None);

    let field = schema.field(0).unwrap();
    assert_eq!(field.name, "id");
    assert_eq!(field.data_type, DataType::Int64);
    assert!(!field.nullable);
}

#[test]
fn test_schema_projection() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
        Field::new("score", DataType::Float64, true),
        Field::new("age", DataType::Int32, true),
    ]);

    let projected = schema.project(&[0, 2]);
    assert_eq!(projected.num_fields(), 2);
    assert_eq!(projected.names(), vec!["id", "score"]);
}

#[test]
fn test_schema_data_types() {
    let schema = Schema::new(vec![
        Field::new("a", DataType::Int64, false),
        Field::new("b", DataType::String, false),
    ]);

    let types = schema.data_types();
    assert_eq!(*types[0], DataType::Int64);
    assert_eq!(*types[1], DataType::String);
}

#[test]
fn test_schema_empty() {
    let schema = Schema::empty();
    assert_eq!(schema.num_fields(), 0);
    assert!(schema.fields().is_empty());
}

#[test]
fn test_schema_from_iterator() {
    let schema: Schema = vec![
        Field::new("x", DataType::Int32, false),
        Field::new("y", DataType::Float64, false),
    ].into_iter().collect();

    assert_eq!(schema.num_fields(), 2);
}

// ===========================================================================
// Field tests
// ===========================================================================

#[test]
fn test_field_creation() {
    let f = Field::new("col", DataType::Int64, true);
    assert_eq!(f.name, "col");
    assert_eq!(f.data_type, DataType::Int64);
    assert!(f.nullable);

    let f2 = Field::not_null("col2", DataType::String);
    assert!(!f2.nullable);
}

// ===========================================================================
// DataType tests
// ===========================================================================

#[test]
fn test_data_type_size() {
    assert_eq!(DataType::Null.size(), 0);
    assert_eq!(DataType::Boolean.size(), 1);
    assert_eq!(DataType::Int8.size(), 1);
    assert_eq!(DataType::Int16.size(), 2);
    assert_eq!(DataType::Int32.size(), 4);
    assert_eq!(DataType::Int64.size(), 8);
    assert_eq!(DataType::Int128.size(), 16);
    assert_eq!(DataType::Float32.size(), 4);
    assert_eq!(DataType::Float64.size(), 8);
    assert_eq!(DataType::Date.size(), 4);
    assert_eq!(DataType::DateTime.size(), 8);
    assert_eq!(DataType::String.size(), 16);
}

#[test]
fn test_data_type_is_numeric() {
    assert!(DataType::Int8.is_numeric());
    assert!(DataType::Int16.is_numeric());
    assert!(DataType::Int32.is_numeric());
    assert!(DataType::Int64.is_numeric());
    assert!(DataType::Int128.is_numeric());
    assert!(DataType::Float32.is_numeric());
    assert!(DataType::Float64.is_numeric());
    assert!(!DataType::Boolean.is_numeric());
    assert!(!DataType::String.is_numeric());
    assert!(!DataType::Date.is_numeric());
}

// ===========================================================================
// ScalarValue tests
// ===========================================================================

#[test]
fn test_scalar_value_data_type() {
    assert_eq!(ScalarValue::Int64(42).data_type(), DataType::Int64);
    assert_eq!(ScalarValue::Float64(3.14).data_type(), DataType::Float64);
    assert_eq!(ScalarValue::Boolean(true).data_type(), DataType::Boolean);
    assert_eq!(ScalarValue::String("hello".to_string()).data_type(), DataType::String);
    assert_eq!(ScalarValue::Date(0).data_type(), DataType::Date);
    assert_eq!(ScalarValue::Null.data_type(), DataType::Null);
}

#[test]
fn test_scalar_value_is_null() {
    assert!(ScalarValue::Null.is_null());
    assert!(!ScalarValue::Int64(0).is_null());
    assert!(!ScalarValue::Boolean(false).is_null());
}

// ===========================================================================
// Block tests
// ===========================================================================

#[test]
fn test_block_creation() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, true),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::String(StringVector::from_vec(vec!["a", "b", "c"])),
    ];

    let block = Block::new(schema, columns);
    assert_eq!(block.num_rows(), 3);
    assert_eq!(block.num_columns(), 2);
    assert!(!block.is_empty());
}

#[test]
fn test_block_empty() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);
    let block = Block::empty(schema);
    assert_eq!(block.num_rows(), 0);
    assert!(block.is_empty());
}

#[test]
fn test_block_column_access() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Float64, true),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![10, 20])),
        Vector::Float64(Float64Vector::from_vec(vec![1.5, 2.5])),
    ];

    let block = Block::new(schema, columns);

    // Access by index
    let col0 = block.column(0).unwrap();
    assert_eq!(col0.len(), 2);

    // Access by name
    let (idx, col) = block.column_by_name("value").unwrap();
    assert_eq!(idx, 1);
    assert_eq!(col.len(), 2);

    // Missing column
    assert!(block.column_by_name("missing").is_none());
}

#[test]
fn test_block_row_access() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2])),
        Vector::String(StringVector::from_vec(vec!["alice", "bob"])),
    ];

    let block = Block::new(schema, columns);
    let row0 = block.row(0);
    assert_eq!(row0[0], ScalarValue::Int64(1));
    assert_eq!(row0[1], ScalarValue::String("alice".to_string()));

    let row1 = block.row(1);
    assert_eq!(row1[0], ScalarValue::Int64(2));
    assert_eq!(row1[1], ScalarValue::String("bob".to_string()));
}

#[test]
fn test_block_projection() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::String, false),
        Field::new("score", DataType::Float64, false),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3])),
        Vector::String(StringVector::from_vec(vec!["a", "b", "c"])),
        Vector::Float64(Float64Vector::from_vec(vec![90.0, 85.0, 95.0])),
    ];

    let block = Block::new(schema, columns);
    let projected = block.project(&[2, 0]);

    assert_eq!(projected.num_columns(), 2);
    assert_eq!(projected.schema().names(), vec!["score", "id"]);

    // Verify data is correct in projected order
    let row0 = projected.row(0);
    assert_eq!(row0[0], ScalarValue::Float64(90.0));
    assert_eq!(row0[1], ScalarValue::Int64(1));
}

#[test]
fn test_block_filter() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Int32, false),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
        Vector::Int32(Int32Vector::from_vec(vec![10, 20, 30, 40, 50])),
    ];

    let block = Block::new(schema, columns);
    let selection = Bitmap::from_bools(&[true, false, true, false, true]);
    let filtered = block.filter(&selection);

    assert_eq!(filtered.num_rows(), 3);

    let row0 = filtered.row(0);
    assert_eq!(row0[0], ScalarValue::Int64(1));
    assert_eq!(row0[1], ScalarValue::Int32(10));

    let row1 = filtered.row(1);
    assert_eq!(row1[0], ScalarValue::Int64(3));
    assert_eq!(row1[1], ScalarValue::Int32(30));
}

#[test]
fn test_block_slice() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
    ];

    let block = Block::new(schema, columns);
    let sliced = block.slice(1, 3);

    assert_eq!(sliced.num_rows(), 3);
    assert_eq!(sliced.row(0)[0], ScalarValue::Int64(2));
    assert_eq!(sliced.row(1)[0], ScalarValue::Int64(3));
    assert_eq!(sliced.row(2)[0], ScalarValue::Int64(4));
}

#[test]
fn test_block_concat() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);

    let make_block = |ids: Vec<i64>| -> Block {
        let s = schema.clone();
        Block::new(s, vec![Vector::Int64(Int64Vector::from_vec(ids))])
    };

    let b1 = make_block(vec![1, 2]);
    let b2 = make_block(vec![3, 4]);
    let b3 = make_block(vec![5]);

    let result = Block::concat(&[b1, b2, b3]).unwrap();
    assert_eq!(result.num_rows(), 5);
    assert_eq!(result.row(0)[0], ScalarValue::Int64(1));
    assert_eq!(result.row(4)[0], ScalarValue::Int64(5));
}

#[test]
fn test_block_concat_empty() {
    let result = Block::concat(&[]);
    assert!(result.is_none());
}

#[test]
fn test_block_append() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ]);

    let mut block = Block::new(
        schema.clone(),
        vec![Vector::Int64(Int64Vector::from_vec(vec![1, 2]))],
    );

    let other = Block::new(
        schema,
        vec![Vector::Int64(Int64Vector::from_vec(vec![3, 4]))],
    );

    block.append_block(&other);
    assert_eq!(block.num_rows(), 4);
}

#[test]
fn test_block_schema() {
    let schema = Schema::new(vec![
        Field::new("a", DataType::Int64, false),
        Field::new("b", DataType::String, true),
    ]);
    let block = Block::empty(schema.clone());
    assert_eq!(block.schema().names(), schema.names());
}

// ===========================================================================
// Vector enum dispatch tests
// ===========================================================================

#[test]
fn test_vector_data_type() {
    assert_eq!(Vector::Int64(Int64Vector::new()).data_type(), DataType::Int64);
    assert_eq!(Vector::Int32(Int32Vector::new()).data_type(), DataType::Int32);
    assert_eq!(Vector::Float64(Float64Vector::new()).data_type(), DataType::Float64);
    assert_eq!(Vector::Boolean(BooleanVector::new()).data_type(), DataType::Boolean);
    assert_eq!(Vector::Date(DateVector::new()).data_type(), DataType::Date);
    assert_eq!(Vector::String(StringVector::new()).data_type(), DataType::String);
}

#[test]
fn test_vector_len_and_empty() {
    let v = Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3]));
    assert_eq!(v.len(), 3);
    assert!(!v.is_empty());

    let v2 = Vector::Int64(Int64Vector::new());
    assert_eq!(v2.len(), 0);
    assert!(v2.is_empty());
}

#[test]
fn test_vector_scalar_at() {
    let v = Vector::Int64(Int64Vector::from_nullable_vec(vec![
        Some(42),
        None,
        Some(100),
    ]));

    assert_eq!(v.scalar_at(0), ScalarValue::Int64(42));
    assert_eq!(v.scalar_at(1), ScalarValue::Null);
    assert_eq!(v.scalar_at(2), ScalarValue::Int64(100));
}

#[test]
fn test_vector_filter_dispatch() {
    let v = Vector::Int64(Int64Vector::from_vec(vec![1, 2, 3, 4]));
    let sel = Bitmap::from_bools(&[true, false, true, false]);
    let filtered = v.filter(&sel);

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered.scalar_at(0), ScalarValue::Int64(1));
    assert_eq!(filtered.scalar_at(1), ScalarValue::Int64(3));
}

#[test]
fn test_vector_slice_dispatch() {
    let v = Vector::String(StringVector::from_vec(vec!["a", "b", "c", "d"]));
    let sliced = v.slice(1, 2);
    assert_eq!(sliced.len(), 2);
    assert_eq!(sliced.scalar_at(0), ScalarValue::String("b".to_string()));
    assert_eq!(sliced.scalar_at(1), ScalarValue::String("c".to_string()));
}

#[test]
fn test_vector_null_count() {
    let v = Vector::Int32(Int32Vector::from_nullable_vec(vec![
        Some(1), None, Some(3), None,
    ]));
    assert_eq!(v.null_count(), 2);
}

#[test]
fn test_vector_append_vector() {
    let mut v1 = Vector::Int64(Int64Vector::from_vec(vec![1, 2]));
    let v2 = Vector::Int64(Int64Vector::from_vec(vec![3, 4]));
    v1.append_vector(&v2);
    assert_eq!(v1.len(), 4);
}

#[test]
fn test_vector_from_scalar() {
    let v = Vector::from_scalar(&ScalarValue::Int64(42), 5);
    assert_eq!(v.len(), 5);
    for i in 0..5 {
        assert_eq!(v.scalar_at(i), ScalarValue::Int64(42));
    }
}

#[test]
fn test_vector_from_scalar_string() {
    let v = Vector::from_scalar(&ScalarValue::String("hello".to_string()), 3);
    assert_eq!(v.len(), 3);
    for i in 0..3 {
        assert_eq!(v.scalar_at(i), ScalarValue::String("hello".to_string()));
    }
}

// ===========================================================================
// Additional typed vector tests
// ===========================================================================

#[test]
fn test_int8_vector() {
    let mut v = Int8Vector::new();
    v.push(Some(1_i8));
    v.push(None);
    v.push(Some(-1_i8));
    assert_eq!(v.len(), 3);
    assert_eq!(v.null_count(), 1);
    assert_eq!(v.get(0), Some(1));
    assert_eq!(v.get(1), None);
    assert_eq!(v.get(2), Some(-1));
}

#[test]
fn test_int16_vector() {
    let v = Int16Vector::from_vec(vec![100_i16, 200, 300]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.get(1), Some(200));
}

#[test]
fn test_int128_vector() {
    let v = Int128Vector::from_vec(vec![1_i128 << 64, 0]);
    assert_eq!(v.len(), 2);
    assert_eq!(v.get(0), Some(1_i128 << 64));
}

#[test]
fn test_float32_vector() {
    let v = Float32Vector::from_vec(vec![1.0_f32, 2.0, 3.0]);
    assert_eq!(v.len(), 3);
    assert_eq!(v.get(1), Some(2.0_f32));
}

#[test]
fn test_float64_vector_filter_with_nulls() {
    let v = Float64Vector::from_nullable_vec(vec![
        Some(1.0), None, Some(3.0), Some(4.0), None,
    ]);
    let sel = Bitmap::from_bools(&[true, true, false, true, false]);
    let filtered = v.filter(&sel);
    assert_eq!(filtered.len(), 3);
    assert_eq!(filtered.get(0), Some(1.0));
    assert_eq!(filtered.get(1), None);
    assert_eq!(filtered.get(2), Some(4.0));
}
