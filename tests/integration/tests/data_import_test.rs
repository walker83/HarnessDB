use std::fs::{self, File};
use std::io::{BufReader, Write};

use integration_tests::common;
use types::{DataType, ScalarValue};

// ===========================================================================
// Helper: create temp files
// ===========================================================================

fn create_test_csv(content: &str, filename: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("rorisdb_test_import");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(filename);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn create_test_json(content: &str, filename: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("rorisdb_test_import");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(filename);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

// ===========================================================================
// 5.1 CSV Import
// ===========================================================================

#[test]
fn test_csv_import_basic() {
    let csv_content = "id,name,age,salary\n1,Alice,30,95000.0\n2,Bob,25,75000.0\n3,Charlie,35,110000.0\n";
    let path = create_test_csv(csv_content, "basic.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader).with_header();

    let batch = csv_reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.num_columns(), 4);
    assert!(batch.num_rows() > 0);

    let headers = csv_reader.headers();
    assert_eq!(headers.len(), 4);
    assert_eq!(headers[0], "id");
    assert_eq!(headers[1], "name");
    assert_eq!(headers[2], "age");
    assert_eq!(headers[3], "salary");
}

#[test]
fn test_csv_import_custom_delimiter() {
    let csv_content = "id|name|value\n1|Alice|100\n2|Bob|200\n";
    let path = create_test_csv(csv_content, "pipe.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader)
        .with_header()
        .with_delimiter(b'|');

    let batch = csv_reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.num_columns(), 3);
    assert!(batch.num_rows() > 0);
}

#[test]
fn test_csv_import_null_values() {
    let csv_content = "id,name,value\n1,Alice,100\n2,,200\n3,Charlie,\n";
    let path = create_test_csv(csv_content, "nulls.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader).with_header();

    let batch = csv_reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 3);

    let headers = csv_reader.headers();
    assert_eq!(headers[1], "name");
    assert_eq!(headers[2], "value");
}

#[test]
fn test_csv_import_many_rows() {
    let mut csv_content = String::from("id,value\n");
    for i in 0..500 {
        csv_content.push_str(&format!("{},val_{}\n", i, i));
    }
    let path = create_test_csv(&csv_content, "many_rows.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader).with_header();

    let mut total_rows = 0;
    while let Some(batch) = csv_reader.next_batch().unwrap() {
        total_rows += batch.num_rows();
    }
    assert_eq!(total_rows, 500);
}

#[test]
fn test_csv_import_then_query_plan() {
    let csv_content = "id,name,department,salary\n1,Alice,Engineering,95000\n2,Bob,Marketing,75000\n";
    let path = create_test_csv(csv_content, "query_test.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader).with_header();
    let batch = csv_reader.next_batch().unwrap().unwrap();

    assert_eq!(batch.num_columns(), 4);

    let catalog = common::create_test_catalog();
    let plan = common::plan_sql(catalog, "test_db",
        "SELECT id, name, department, salary FROM employees WHERE salary > 3000");
    let node_types = common::collect_node_types(&plan);
    assert!(node_types.contains(&"Scan".to_string()));
}

// ===========================================================================
// 5.2 JSON Import
// ===========================================================================

#[test]
fn test_json_lines_import() {
    let json_content = "{\"id\": 1, \"name\": \"Alice\", \"salary\": 95000.0}\n{\"id\": 2, \"name\": \"Bob\", \"salary\": 75000.0}\n";
    let path = create_test_json(json_content, "data.jsonl");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut json_reader = data_io::json_reader::JsonReader::new(reader);

    let batch = json_reader.next_batch().unwrap().unwrap();
    assert!(batch.num_rows() > 0);
    assert!(batch.num_columns() > 0);
}

#[test]
fn test_json_lines_many_rows() {
    let mut json_content = String::new();
    for i in 0..200 {
        json_content.push_str(&format!("{{\"id\": {}, \"name\": \"user_{}\", \"score\": {}}}\n", i, i, i as f64 * 1.5));
    }
    let path = create_test_json(&json_content, "many.jsonl");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut json_reader = data_io::json_reader::JsonReader::new(reader);

    let mut total_rows = 0;
    while let Some(batch) = json_reader.next_batch().unwrap() {
        total_rows += batch.num_rows();
    }
    assert_eq!(total_rows, 200);
}

#[test]
fn test_json_import_with_nulls() {
    let json_content = "{\"id\": 1, \"name\": \"Alice\"}\n{\"id\": 2, \"name\": null}\n";
    let path = create_test_json(json_content, "nulls.jsonl");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut json_reader = data_io::json_reader::JsonReader::new(reader);

    let batch = json_reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 2);
}

// ===========================================================================
// 5.3 Parquet Import (if test file exists)
// ===========================================================================

#[test]
fn test_parquet_import_zclawbench() {
    let parquet_path = "/tmp/ZClawBench/train.parquet";
    if !std::path::Path::new(parquet_path).exists() {
        eprintln!("Skipping parquet test: {} not found", parquet_path);
        return;
    }

    use data_io::parquet_reader::ParquetReader;

    let mut reader = ParquetReader::open(parquet_path).unwrap();
    assert_eq!(reader.num_rows(), 696);
    assert_eq!(reader.num_columns(), 4);

    let schema = reader.schema();
    assert_eq!(schema.num_fields(), 4);

    let mut total_rows = 0;
    let mut blocks = 0;
    while let Some(batch) = reader.next_batch().unwrap() {
        total_rows += batch.num_rows();
        blocks += 1;
    }
    assert_eq!(total_rows, 696);
    assert!(blocks > 0);
}

#[test]
fn test_parquet_schema_inference() {
    let parquet_path = "/tmp/ZClawBench/train.parquet";
    if !std::path::Path::new(parquet_path).exists() {
        eprintln!("Skipping parquet schema test: {} not found", parquet_path);
        return;
    }

    use data_io::parquet_reader::ParquetReader;

    let reader = ParquetReader::open(parquet_path).unwrap();
    let schema = reader.schema();
    let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"task_id"));
    assert!(field_names.contains(&"model_name"));
}

// ===========================================================================
// 5.4 Stream Load
// ===========================================================================

#[test]
fn test_stream_load_csv_format() {
    use data_io::stream_load::{StreamLoad, LoadFormat};

    let loader = StreamLoad::new("test_db", "employees");
    let csv_data = b"id,name,salary\n1,Alice,95000\n2,Bob,75000\n".to_vec();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(loader.load(csv_data, LoadFormat::Csv)).unwrap();
    assert!(result.is_success());
    assert!(result.rows_loaded > 0);
}

#[test]
fn test_stream_load_json_format() {
    use data_io::stream_load::{StreamLoad, LoadFormat};

    let loader = StreamLoad::new("test_db", "employees");
    let json_data = b"{\"id\": 1, \"name\": \"Alice\"}\n{\"id\": 2, \"name\": \"Bob\"}\n".to_vec();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(loader.load(json_data, LoadFormat::Json)).unwrap();
    assert!(result.is_success());
    assert!(result.rows_loaded > 0);
}

#[test]
fn test_stream_load_builder() {
    use data_io::stream_load::{StreamLoadBuilder, LoadFormat};

    let loader = StreamLoadBuilder::new("test_db", "orders")
        .with_timeout(7200)
        .with_format(LoadFormat::Csv)
        .with_header("Authorization", "Bearer token")
        .build();

    assert_eq!(loader.db_name(), "test_db");
    assert_eq!(loader.table_name(), "orders");
    assert_eq!(loader.timeout_secs(), 7200);
}

#[test]
fn test_load_format_conversion() {
    use data_io::stream_load::LoadFormat;

    assert_eq!(LoadFormat::from_str("csv"), Some(LoadFormat::Csv));
    assert_eq!(LoadFormat::from_str("CSV"), Some(LoadFormat::Csv));
    assert_eq!(LoadFormat::from_str("json"), Some(LoadFormat::Json));
    assert_eq!(LoadFormat::from_str("unknown"), None);
    assert_eq!(LoadFormat::Csv.as_str(), "csv");
    assert_eq!(LoadFormat::Json.as_str(), "json");
}

#[test]
fn test_load_result_success_failure() {
    use data_io::stream_load::LoadResult;

    let success = LoadResult::success(100);
    assert!(success.is_success());
    assert_eq!(success.rows_loaded, 100);
    assert_eq!(success.errors, 0);

    let failure = LoadResult::failure("test error".into());
    assert!(!failure.is_success());
    assert_eq!(failure.rows_loaded, 0);
    assert_eq!(failure.errors, 1);
}

// ===========================================================================
// Import + Block operation integration
// ===========================================================================

#[test]
fn test_csv_import_filter_and_aggregate() {
    let mut csv_content = String::from("id,department,salary\n");
    for i in 0..100 {
        let dept = match i % 3 { 0 => "Engineering", 1 => "Marketing", _ => "Sales" };
        let salary = 50000.0 + (i as f64 * 500.0);
        csv_content.push_str(&format!("{},{},{:.1}\n", i, dept, salary));
    }
    let path = create_test_csv(&csv_content, "agg_test.csv");

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = data_io::csv_reader::CsvReader::new(reader).with_header();

    let batch = csv_reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 100);

    // Filter salary > 80000
    let salary_col = batch.column_by_name("salary").unwrap().1;
    let mut sel = types::Bitmap::with_capacity(batch.num_rows());
    for i in 0..batch.num_rows() {
        let pass = matches!(salary_col.scalar_at(i), ScalarValue::Float64(v) if v > 80000.0);
        sel.push(pass);
    }
    let filtered = batch.filter(&sel);
    assert!(filtered.num_rows() > 0);
}
