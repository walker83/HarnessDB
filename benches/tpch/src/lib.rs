pub mod data_gen;
pub mod queries;

use std::collections::HashMap;
use std::sync::Arc;

use fe_catalog::table::{TableColumn, Table};
use fe_catalog::{CatalogManager, Table as CatalogTable};
use fe_sql_planner::{Planner, PlanNode};
use types::Block;

use data_gen::TpchData;
use be_storage::{StorageEngine, tablet::TabletSchema};

/// Result of running a single TPC-H query.
#[derive(Debug)]
pub struct QueryResult {
    pub query_name: &'static str,
    pub planning_time_us: u64,
    pub plan: Option<PlanNode>,
    pub error: Option<String>,
    pub blocks: Vec<Block>,
}

impl QueryResult {
    pub fn rows(&self) -> usize {
        self.blocks.iter().map(|b| b.num_rows()).sum()
    }
}

/// Execution result with actual data blocks.
#[derive(Debug)]
pub struct ExecutionResult {
    pub blocks: Vec<Block>,
    pub rows_produced: usize,
    pub planning_time_us: u64,
    pub execution_time_us: u64,
}

/// The TPC-H benchmark framework.
/// Sets up a catalog with TPC-H data, writes it to storage, then runs queries through the planner and executor.
pub struct TpchBenchmark {
    data: TpchData,
    catalog: Arc<CatalogManager>,
    storage: Arc<StorageEngine>,
}

impl TpchBenchmark {
    /// Create a new benchmark instance with SF 0.01 data.
    pub fn new_sf001() -> Self {
        let data = TpchData::generate_sf001();
        Self::new_with_data(data)
    }

    /// Create a new benchmark instance with tiny data for quick tests.
    pub fn new_tiny() -> Self {
        let data = TpchData::generate_tiny();
        Self::new_with_data(data)
    }

    /// Create a new benchmark instance with provided data.
    fn new_with_data(data: TpchData) -> Self {
        let catalog = Arc::new(CatalogManager::new());
        let storage = Arc::new(StorageEngine::open("/tmp/tpch_storage").unwrap());

        // Create the tpch database
        catalog.create_database("tpch").unwrap();

        // Register tables and write to storage
        Self::setup_tables(&data, &catalog, &storage);

        Self { data, catalog, storage }
    }

    /// Setup tables: register in catalog and write to storage.
    fn setup_tables(data: &TpchData, catalog: &Arc<CatalogManager>, storage: &Arc<StorageEngine>) {
        let table_specs = [
            (1, "nation", &data.nation),
            (2, "region", &data.region),
            (3, "supplier", &data.supplier),
            (4, "part", &data.part),
            (5, "partsupp", &data.partsupp),
            (6, "customer", &data.customer),
            (7, "orders", &data.orders),
            (8, "lineitem", &data.lineitem),
        ];

        for (id, name, block) in table_specs {
            // Create table in catalog
            let table = make_table(id, "tpch", name, block);
            let _ = catalog.create_table("tpch", table);

            // Create tablet in storage and write data
            let schema = block.schema().clone();
            let tablet_schema = TabletSchema {
                tablet_id: id,
                columns: schema.fields().iter().map(|f| {
                    be_storage::tablet::TabletColumn {
                        name: f.name.clone(),
                        data_type: f.data_type.clone(),
                        nullable: f.nullable,
                        is_key: false,
                        agg_type: None,
                    }
                }).collect(),
                keys_type: "Duplicate".to_string(),
                num_rows_per_row_block: 1024,
            };

            if let Err(e) = storage.create_tablet(id, tablet_schema) {
                tracing::warn!("Tablet {} may already exist: {}", id, e);
            }

            if let Err(e) = storage.write_batch(id, block) {
                tracing::error!("Failed to write block to tablet {}: {}", id, e);
            }
        }

        tracing::info!("TPC-H tables setup complete");
    }

    /// Get a reference to the generated data.
    pub fn data(&self) -> &TpchData {
        &self.data
    }

    /// Get a reference to the catalog.
    pub fn catalog(&self) -> &Arc<CatalogManager> {
        &self.catalog
    }

    /// Get a reference to the storage engine.
    pub fn storage(&self) -> &Arc<StorageEngine> {
        &self.storage
    }

    /// Run a single TPC-H query by index (1-22).
    pub fn run_query(&self, index: usize) -> QueryResult {
        let queries = queries::all_queries();
        if index == 0 || index > queries.len() {
            return QueryResult {
                query_name: "INVALID",
                planning_time_us: 0,
                plan: None,
                error: Some(format!("Invalid query index: {}", index)),
                blocks: vec![],
            };
        }

        let (name, sql) = queries[index - 1];
        self.run_sql(name, sql)
    }

    /// Run a query by name and SQL string.
    pub fn run_sql(&self, name: &'static str, sql: &str) -> QueryResult {
        let mut planner = Planner::new(self.catalog.clone());
        planner.set_database("tpch");

        let parse_start = std::time::Instant::now();

        // Parse the SQL
        let statements = match fe_sql_parser::parse_sql(sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                return QueryResult {
                    query_name: name,
                    planning_time_us: parse_start.elapsed().as_micros() as u64,
                    plan: None,
                    error: Some(format!("Parse error: {:?}", e)),
                    blocks: vec![],
                };
            }
        };

        // Plan the query
        let plan = match statements.into_iter().next() {
            Some(stmt) => match planner.plan(stmt) {
                Ok(plan) => Some(plan),
                Err(e) => {
                    return QueryResult {
                        query_name: name,
                        planning_time_us: parse_start.elapsed().as_micros() as u64,
                        plan: None,
                        error: Some(format!("Plan error: {:?}", e)),
                        blocks: vec![],
                    };
                }
            },
            None => {
                return QueryResult {
                    query_name: name,
                    planning_time_us: parse_start.elapsed().as_micros() as u64,
                    plan: None,
                    error: Some("No statements found".to_string()),
                    blocks: vec![],
                };
            }
        };

        QueryResult {
            query_name: name,
            planning_time_us: parse_start.elapsed().as_micros() as u64,
            plan,
            error: None,
            blocks: vec![],
        }
    }

    /// Execute a query and return actual results.
    /// This uses the storage engine and execution context to actually run the query.
    pub fn execute_query(&self, index: usize) -> ExecutionResult {
        let queries = queries::all_queries();
        if index == 0 || index > queries.len() {
            return ExecutionResult {
                blocks: vec![],
                rows_produced: 0,
                planning_time_us: 0,
                execution_time_us: 0,
            };
        }

        let (name, sql) = queries[index - 1];
        self.execute_sql(name, sql)
    }

    /// Execute a query by name and SQL string (sync wrapper).
    pub fn execute_sql(&self, name: &'static str, sql: &str) -> ExecutionResult {
        let planning_start = std::time::Instant::now();
        let exec_start = std::time::Instant::now();

        // Use a separate thread with its own runtime to avoid nested runtime issues
        let blocks = std::thread::scope(|s| {
            s.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    let mut planner = Planner::new(self.catalog.clone());
                    planner.set_database("tpch");

                    // Parse and plan
                    let statements = match fe_sql_parser::parse_sql(sql) {
                        Ok(stmts) => stmts,
                        Err(e) => {
                            tracing::warn!("Parse error for {}: {:?}", name, e);
                            return vec![];
                        }
                    };

                    let plan = match statements.into_iter().next() {
                        Some(stmt) => match planner.plan(stmt) {
                            Ok(plan) => plan,
                            Err(e) => {
                                tracing::warn!("Plan error for {}: {:?}", name, e);
                                return vec![];
                            }
                        },
                        None => return vec![],
                    };

                    // Create execution context and execute
                    let context = be_execution::ExecutionContext::new(self.storage.clone(), self.catalog.clone());

                    match be_execution::execute_plan(&plan, &context).await {
                        Ok(blocks) => blocks,
                        Err(e) => {
                            tracing::warn!("Execution error for {}: {}", name, e);
                            vec![]
                        }
                    }
                })
            }).join().unwrap()
        });

        let execution_time_us = exec_start.elapsed().as_micros() as u64;
        let rows_produced: usize = blocks.iter().map(|b| b.num_rows()).sum();

        ExecutionResult {
            planning_time_us: planning_start.elapsed().as_micros() as u64,
            execution_time_us,
            rows_produced,
            blocks,
        }
    }

    /// Run all 22 TPC-H queries and return results.
    pub fn run_all(&self) -> Vec<QueryResult> {
        (1..=22).map(|i| self.run_query(i)).collect()
    }
}

/// Helper to create a Table definition from a Block's schema.
fn make_table(id: u64, database: &str, name: &str, block: &Block) -> Table {
    let columns: Vec<TableColumn> = block
        .schema()
        .fields()
        .iter()
        .map(|f| TableColumn {
            name: f.name.clone(),
            data_type: f.data_type.clone(),
            nullable: f.nullable,
            default_value: None,
            agg_type: None,
            comment: String::new(),
        })
        .collect();

    Table {
        id,
        name: name.to_string(),
        database: database.to_string(),
        columns,
        keys_type: fe_catalog::table::KeysType::Duplicate,
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: block.num_rows() as u64,
        data_size: 0,
    }
}
