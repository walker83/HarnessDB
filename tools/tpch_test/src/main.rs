use std::time::Instant;
use tpch_bench::{TpchBenchmark, queries};

fn main() {
    println!("=== RorisDB TPC-H Performance Test (Execution Time) ===\n");

    let iterations = 5;
    println!("Running {} iterations for stable timing...\n", iterations);

    let all_queries = queries::all_queries();

    // Warm up
    let bench = TpchBenchmark::new_tiny();
    for idx in 1..=all_queries.len() {
        let _ = bench.execute_query(idx);
    }

    // Header
    println!("| Q# | Query Name          | Status  | Avg (μs) | Min (μs) | Max (μs) | Rows |");
    println!("|----|---------------------|---------|----------|----------|----------|------|");

    let mut total_ok = 0;
    let mut total_err = 0;
    let mut total_min = u64::MAX;
    let mut total_max = 0u64;
    let mut total_avg = 0u64;

    for (idx, (name, _sql)) in all_queries.iter().enumerate() {
        let mut times: Vec<u64> = Vec::with_capacity(iterations);
        let mut total_rows = 0;

        for _ in 0..iterations {
            let bench = TpchBenchmark::new_tiny();
            let start = Instant::now();
            let result = bench.execute_query(idx + 1);
            let elapsed = start.elapsed().as_micros() as u64;

            if result.blocks.is_empty() {
                times.push(u64::MAX);
            } else {
                times.push(elapsed);
                total_rows += result.rows_produced;
            }
        }

        let q_min = times.iter().filter(|&&t| t != u64::MAX).min().unwrap_or(&0);
        let q_max = times.iter().filter(|&&t| t != u64::MAX).max().unwrap_or(&0);
        let q_avg = if times.iter().any(|&t| t != u64::MAX) {
            times.iter().filter(|&&t| t != u64::MAX).sum::<u64>() / times.iter().filter(|&&t| t != u64::MAX).count() as u64
        } else {
            0
        };

        let status = if times.iter().any(|&t| t == u64::MAX) { "FAIL" } else { "OK" };
        if times.iter().any(|&t| t == u64::MAX) {
            total_err += 1;
        } else {
            total_ok += 1;
            total_min = total_min.min(*q_min);
            total_max = total_max.max(*q_max);
            total_avg += q_avg;
        }

        let short_name = name.replace("q", "Q");
        let avg_rows = if total_rows > 0 { total_rows / iterations } else { 0 };
        
        if status == "OK" {
            println!("| {:2} | {:18} | {:7} | {:8} | {:8} | {:8} | {:4} |",
                     idx + 1, short_name, status, q_avg, q_min, q_max, avg_rows);
        } else {
            println!("| {:2} | {:18} | {:7} | {:8} | {:8} | {:8} | {:4} |",
                     idx + 1, short_name, status, "-", "-", "-", "-");
        }
    }

    println!("\n=== Summary ===");
    println!("Total queries: {}", all_queries.len());
    println!("Passed: {}", total_ok);
    println!("Failed: {}", total_err);
    
    if total_ok > 0 {
        println!("\nOverall execution time (avg of all queries): {} μs", total_avg / total_ok);
        println!("Overall min time: {} μs", total_min);
        println!("Overall max time: {} μs", total_max);
    }
    
    println!("\n=== Note ===");
    println!("This benchmark measures TOTAL execution time (parsing + planning + execution).");
    println!("Use this for fair comparison with other databases like DuckDB.");
}
