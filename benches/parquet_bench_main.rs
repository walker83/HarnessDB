// Run with: cargo run --release --bin parquet_bench
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

    println!("Read: {:.4}s, {} rows, {} blocks", read_time, total_rows, blocks);
    println!("Throughput: {:.2} MB/s", 23.0 / read_time);

    println!("\n========================");
}
