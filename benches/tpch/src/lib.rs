pub mod data_gen;
pub mod queries;

use std::collections::HashMap;
use std::sync::Arc;

use fe_catalog::table::TableColumn;
use fe_catalog::{CatalogManager, Table};
use fe_sql_planner::{Planner, PlanNode};
use types::Block;

use data_gen::TpchData;

/// Result of running a single TPC-H query.
#[derive(Debug)]
pub struct QueryResult {
    pub query_name: &'static str,
    pub planning_time_us: u64,
    pub plan: Option<PlanNode>,
    pub error: Option<String>,
}

/// The TPC-H benchmark framework.
/// Sets up a catalog with TPC-H data, then runs queries through the planner.
pub struct TpchBenchmark {
    data: TpchData,
    catalog: Arc<CatalogManager>,
}

impl TpchBenchmark {
    /// Create a new benchmark instance with SF 0.01 data.
    pub fn new_sf001() -> Self {
        let data = TpchData::generate_sf001();
        let catalog = Self::build_catalog(&data);
        Self { data, catalog }
    }

    /// Create a new benchmark instance with tiny data for quick tests.
    pub fn new_tiny() -> Self {
        let data = TpchData::generate_tiny();
        let catalog = Self::build_catalog(&data);
        Self { data, catalog }
    }

    /// Get a reference to the generated data.
    pub fn data(&self) -> &TpchData {
        &self.data
    }

    /// Get a reference to the catalog.
    pub fn catalog(&self) -> &Arc<CatalogManager> {
        &self.catalog
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
            };
        }

        let (name, sql) = queries[index - 1];
        self.run_sql(name, sql)
    }

    /// Run a query by name and SQL string.
    pub fn run_sql(&self, name: &'static str, sql: &str) -> QueryResult {
        let mut planner = Planner::new(self.catalog.clone());
        planner.set_database("tpch");

        // Parse the SQL
        let parse_start = std::time::Instant::now();
        let statements = match fe_sql_parser::parse_sql(sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                return QueryResult {
                    query_name: name,
                    planning_time_us: parse_start.elapsed().as_micros() as u64,
                    plan: None,
                    error: Some(format!("Parse error: {:?}", e)),
                };
            }
        };

        let plan = match statements.into_iter().next() {
            Some(stmt) => match planner.plan(stmt) {
                Ok(plan) => {
                    let _elapsed = parse_start.elapsed().as_micros() as u64;
                    Some(plan)
                }
                Err(e) => {
                    return QueryResult {
                        query_name: name,
                        planning_time_us: parse_start.elapsed().as_micros() as u64,
                        plan: None,
                        error: Some(format!("Plan error: {:?}", e)),
                    };
                }
            },
            None => {
                return QueryResult {
                    query_name: name,
                    planning_time_us: parse_start.elapsed().as_micros() as u64,
                    plan: None,
                    error: Some("No statements found".to_string()),
                };
            }
        };

        QueryResult {
            query_name: name,
            planning_time_us: parse_start.elapsed().as_micros() as u64,
            plan,
            error: None,
        }
    }

    /// Run all 22 TPC-H queries and return results.
    pub fn run_all(&self) -> Vec<QueryResult> {
        (1..=22).map(|i| self.run_query(i)).collect()
    }

    /// Build the catalog with TPC-H schema tables registered.
    fn build_catalog(data: &TpchData) -> Arc<CatalogManager> {
        let catalog = Arc::new(CatalogManager::new());

        // Create the tpch database
        catalog.create_database("tpch").unwrap();

        // Register tables in the catalog
        let tables = Self::build_table_definitions(data);
        for table in tables {
            catalog.create_table("tpch", table).unwrap();
        }

        catalog
    }

    /// Build Table definitions for catalog registration from generated data schemas.
    fn build_table_definitions(data: &TpchData) -> Vec<Table> {
        vec![
            make_table(1, "tpch", "nation", &data.nation),
            make_table(2, "tpch", "region", &data.region),
            make_table(3, "tpch", "supplier", &data.supplier),
            make_table(4, "tpch", "part", &data.part),
            make_table(5, "tpch", "partsupp", &data.partsupp),
            make_table(6, "tpch", "customer", &data.customer),
            make_table(7, "tpch", "orders", &data.orders),
            make_table(8, "tpch", "lineitem", &data.lineitem),
        ]
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
