use std::time::Instant;
use data_io::parquet_reader::ParquetReader;

fn main() {
    println!("HarnessDB Parquet Benchmark");
    println!("========================\n");

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

    println!("Read: {:.4f}s, {} rows, {} blocks", read_time, total_rows, blocks);
    println!("Throughput: {:.2f} MB/s", 23.0 / read_time);

    // Re-read for aggregation test
    let start = Instant::now();
    let mut reader = ParquetReader::open("/tmp/ZClawBench/train.parquet").unwrap();
    let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    while let Some(batch) = reader.next_batch().unwrap() {
        let schema = batch.schema();
        let cat_idx = schema.fields().iter().position(|f| f.name() == "task_category");

        if let Some(idx) = cat_idx {
            let col = batch.column(idx).unwrap();
            for i in 0..batch.num_rows() {
                if let types::ScalarValue::String(s) = col.scalar_at(i) {
                    *category_counts.entry(s.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    let agg_time = start.elapsed().as_secs_f64();
    println!("\nAggregation: {:.4f}s", agg_time);
    for (cat, cnt) in &category_counts {
        println!("  {}: {}", cat, cnt);
    }

    // Filter test
    let start = Instant::now();
    let mut reader = ParquetReader::open("/tmp/ZClawBench/train.parquet").unwrap();
    let mut count = 0;

    while let Some(batch) = reader.next_batch().unwrap() {
        let schema = batch.schema();
        let cat_idx = schema.fields().iter().position(|f| f.name() == "task_category");

        if let Some(idx) = cat_idx {
            let col = batch.column(idx).unwrap();
            for i in 0..batch.num_rows() {
                if let types::ScalarValue::String(s) = col.scalar_at(i) {
                    if s == "Data Analysis" {
                        count += 1;
                    }
                }
            }
        }
    }
    let filter_time = start.elapsed().as_secs_f64();
    println!("\nFilter: {:.4f}s, count={}", filter_time, count);

    println!("\n========================");
    println!("Total time: {:.4f}s", read_time + agg_time + filter_time);
}
