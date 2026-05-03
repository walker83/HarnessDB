use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use fe_catalog::CatalogManager;
use fe_sql_parser::parse_sql;
use fe_sql_planner::Planner;

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
    let planner = Planner::new(catalog.clone());

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
                                _ => {
                                    match planner.plan(stmt) {
                                        Ok(plan) => println!("{}", plan),
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
