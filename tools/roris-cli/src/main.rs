use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use fe_catalog::CatalogManager;
use fe_catalog::CatalogStatsProvider;
use fe_sql_parser::parse_sql;
use fe_sql_planner::Planner;
use fe_sql_planner::Optimizer;

#[derive(Parser)]
#[command(name = "roris-cli", about = "Roris command line client")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "9020")]
    port: u16,

    #[arg(long, default_value = "root")]
    user: String,
}

fn main() -> Result<()> {
    let _args = Args::parse();
    println!("roris-cli v0.1.0");
    println!("Type 'help' for usage, 'quit' to exit.\n");

    let catalog = Arc::new(CatalogManager::new());
    let stats_provider = Arc::new(CatalogStatsProvider::new(catalog.clone()));
    let planner = Planner::new(catalog.clone());
    let optimizer = Optimizer::new().with_stats_provider(stats_provider);

    let mut rl = rustyline::DefaultEditor::new()?;
    loop {
        let readline = rl.readline("roris> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line)?;

                match line {
                    "quit" | "exit" => break,
                    "help" => {
                        println!("Available commands:");
                        println!("  SHOW DATABASES;         - list databases");
                        println!("  CREATE DATABASE <name>; - create a database");
                        println!("  USE <db>;              - switch database");
                        println!("  SHOW TABLES;            - list tables");
                        println!("  ANALYZE TABLE <table>;  - collect statistics");
                        println!("  SHOW STATS [FROM <t>];  - show statistics");
                        println!("  <SQL statement>;        - execute SQL");
                        println!("  quit / exit             - exit client");
                        continue;
                    }
                    _ => {}
                }

                match parse_sql(line) {
                    Ok(stmts) => {
                        for stmt in stmts {
                            match &stmt {
                                fe_sql_parser::Statement::ShowDatabases => {
                                    let dbs = catalog.list_databases();
                                    println!("{:?}", dbs);
                                }
                                fe_sql_parser::Statement::AnalyzeTable { database, table } => {
                                    let db = database.as_deref().unwrap_or("information_schema");
                                    match handle_analyze_table(&catalog, db, table) {
                                        Ok(()) => println!("OK - Statistics collected for {}.{}", db, table),
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                                fe_sql_parser::Statement::ShowStats { database, table } => {
                                    let db = database.as_deref().unwrap_or("information_schema");
                                    handle_show_stats(&catalog, db, table.as_deref());
                                }
                                _ => {
                                    match planner.plan(stmt) {
                                        Ok(plan) => {
                                            let optimized = optimizer.optimize(plan);
                                            println!("{}", optimized);
                                        }
                                        Err(e) => eprintln!("Plan error: {}", e),
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Parse error: {}", e),
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}

fn handle_analyze_table(
    catalog: &CatalogManager,
    database: &str,
    table: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use fe_catalog::stats::{TableStats, ColumnStats};

    let tbl = catalog.get_table(database, table)
        .ok_or_else(|| format!("Table {}.{} not found", database, table))?;

    // Build stats from catalog metadata
    let mut stats = TableStats::with_row_count(tbl.row_count);
    stats.data_size = tbl.data_size;
    stats.updated_at = Some(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?.as_secs());

    for col in &tbl.columns {
        let col_stats = ColumnStats {
            ndv: if tbl.row_count > 0 { (tbl.row_count * 2 / 3).max(1) } else { 0 },
            null_count: 0,
            min_value: None,
            max_value: None,
            avg_width: match col.data_type {
                types::DataType::Int32 | types::DataType::Float32 => 4,
                types::DataType::Int64 | types::DataType::Float64 => 8,
                types::DataType::String => 50,
                types::DataType::Date => 4,
                _ => 8,
            },
            histogram: None,
        };
        stats.column_stats.insert(col.name.clone(), col_stats);
    }

    catalog.update_table_stats(database, table, stats)?;
    Ok(())
}

fn handle_show_stats(
    catalog: &CatalogManager,
    database: &str,
    table: Option<&str>,
) {
    if let Some(table_name) = table {
        match catalog.get_table_stats(database, table_name) {
            Some(stats) => {
                println!("Table: {}.{}", database, table_name);
                println!("  Row count: {}", stats.row_count);
                println!("  Data size: {} bytes", stats.data_size);
                if let Some(ts) = stats.updated_at {
                    println!("  Updated: {} (epoch)", ts);
                }
                if stats.column_stats.is_empty() {
                    println!("  No column statistics");
                } else {
                    println!("  Column statistics:");
                    println!("  {:20} {:>8} {:>8} {:>8} {:>10} {:>10}",
                             "Column", "NDV", "Nulls", "AvgWd", "Min", "Max");
                    for (name, col) in &stats.column_stats {
                        println!("  {:20} {:>8} {:>8} {:>8} {:>10} {:>10}",
                                 name, col.ndv, col.null_count, col.avg_width,
                                 col.min_value.as_deref().unwrap_or("?"),
                                 col.max_value.as_deref().unwrap_or("?"));
                    }
                }
            }
            None => println!("No statistics for {}.{}", database, table_name),
        }
    } else {
        let all_stats = catalog.get_all_table_stats(database);
        if all_stats.is_empty() {
            println!("No tables found in {}", database);
        } else {
            println!("Statistics for database '{}':", database);
            println!("  {:20} {:>10} {:>12} {:>10}", "Table", "Rows", "DataSize", "HasStats");
            for (name, stats) in &all_stats {
                match stats {
                    Some(s) => {
                        println!("  {:20} {:>10} {:>12} {:>10}",
                                 name, s.row_count, s.data_size, "Yes");
                    }
                    None => {
                        println!("  {:20} {:>10} {:>12} {:>10}",
                                 name, "-", "-", "No");
                    }
                }
            }
        }
    }
}
