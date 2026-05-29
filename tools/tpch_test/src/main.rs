use std::time::Instant;

fn main() {
    println!("========================================");
    println!("  TPC-H via DataFusion Integration");
    println!("========================================\n");

    let _ = std::fs::remove_dir_all("/tmp/tpch_storage");
    let bench = tpch_bench::TpchBenchmark::new_tiny();

    // -------------------------------------------------------
    // Storage verification
    // -------------------------------------------------------
    println!("--- Storage Verification ---\n");

    let table_specs = [
        ("nation", 15),
        ("region", 5),
        ("supplier", 10),
        ("part", 20),
        ("partsupp", 80),
        ("customer", 15),
        ("orders", 150),
        ("lineitem", 600),
    ];

    let storage = bench.storage();
    let mut all_ok = true;
    for (name, expected) in &table_specs {
        let read_rows = storage
            .read("tpch", name)
            .map(|b| b.num_rows())
            .unwrap_or(0);
        let read_status = if read_rows == *expected {
            "OK"
        } else {
            "MISMATCH"
        };
        if read_rows != *expected {
            all_ok = false;
        }
        println!(
            "  {} table={} rows={}/{}",
            read_status, name, read_rows, expected
        );
    }

    if !all_ok {
        println!("\nStorage issues detected!\n");
    }

    // -------------------------------------------------------
    // DataFusion execution: all 22 queries
    // -------------------------------------------------------
    println!("\n--- DataFusion Execution: All 22 TPC-H Queries ---\n");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let ctx = bench.datafusion_ctx();

        // Verify table resolution
        println!("  Registered schemas: {:?}", ctx.catalog_names());
        for name in &["nation", "region", "supplier", "part", "partsupp", "customer", "orders", "lineitem"] {
            match ctx.table(*name).await {
                Ok(t) => println!("  Table '{}' resolved: {} columns", name, t.schema().fields().len()),
                Err(e) => println!("  Table '{}' NOT FOUND: {}", name, e),
            }
        }
        println!();

        // Quick sanity queries
        exec_df(&ctx, "SELECT * FROM nation", "Simple scan (nation)").await;
        exec_df(&ctx, "SELECT * FROM lineitem LIMIT 10", "Scan with LIMIT").await;
        exec_df(&ctx, "SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag", "Simple GROUP BY").await;
        exec_df(&ctx, "SELECT COUNT(*) FROM lineitem WHERE l_returnflag = 'R'", "WHERE + COUNT").await;

        // Q1 with date expression — DataFusion handles DATE/INTERVAL natively
        exec_df(&ctx,
            "SELECT l_returnflag, l_linestatus, COUNT(*) FROM lineitem WHERE l_shipdate <= DATE '1998-12-01' - INTERVAL '90' DAY GROUP BY l_returnflag, l_linestatus",
            "Q1 with DATE/INTERVAL").await;

        // Run all 22 TPC-H queries
        println!("\n  --- All 22 TPC-H Queries via DataFusion ---\n");
        println!("{:<5} {:<10} {:<15} {}", "Q", "Rows", "Time(ms)", "Status");
        println!("{}", "-".repeat(60));

        let queries = tpch_bench::queries::all_queries();
        let mut passed = 0;
        for (i, (name, sql)) in queries.iter().enumerate() {
            let start = Instant::now();
            match ctx.sql(sql).await {
                Ok(df) => {
                    match df.collect().await {
                        Ok(batches) => {
                            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                            let elapsed = start.elapsed().as_millis();
                            println!("{:<5} {:<10} {:<15} {} ({})", i + 1, rows, elapsed, "OK", name);
                            passed += 1;
                        }
                        Err(e) => {
                            let elapsed = start.elapsed().as_millis();
                            println!("{:<5} {:<10} {:<15} {} ({}) - collect error: {:.80}", i + 1, 0, elapsed, "FAIL", name, e);
                        }
                    }
                }
                Err(e) => {
                    let elapsed = start.elapsed().as_millis();
                    println!("{:<5} {:<10} {:<15} {} ({}) - plan error: {:.80}", i + 1, 0, elapsed, "FAIL", name, e);
                }
            }
        }
        println!("  => {}/22 passed\n", passed);
    });

    println!("========================================");
    println!("  Done");
    println!("========================================");
}

async fn exec_df(ctx: &datafusion::prelude::SessionContext, sql: &str, label: &str) {
    let start = Instant::now();
    match ctx.sql(sql).await {
        Ok(df) => match df.collect().await {
            Ok(batches) => {
                let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                let elapsed = start.elapsed().as_millis();
                if rows > 0 {
                    println!("  OK  {} => {} rows ({} ms)", label, rows, elapsed);
                    if let Some(b) = batches.first() {
                        let cols: Vec<String> = b
                            .schema()
                            .fields()
                            .iter()
                            .take(5)
                            .map(|f| format!("{}", f.name()))
                            .collect();
                        println!("       columns: [{}]", cols.join(", "));
                    }
                } else {
                    println!("  EMPTY  {} => 0 rows ({} ms)", label, elapsed);
                }
            }
            Err(e) => println!("  FAIL  {} => collect error: {:.60}", label, e),
        },
        Err(e) => println!("  FAIL  {} => plan error: {:.60}", label, e),
    }
}
