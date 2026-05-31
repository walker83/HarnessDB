pub mod data_gen;
pub mod queries;

use std::collections::HashMap;
use std::sync::Arc;

use arrow_schema::{Field, Schema as ArrowSchema};
use fe_catalog::CatalogManager;
use fe_catalog::table::{Table, TableColumn};
use fe_datafusion::block_convert;
use fe_storage::{ParquetCatalogProvider, ParquetStorage};
use types::Block;

use data_gen::TpchData;
use datafusion::prelude::SessionContext;

/// Result of running a single TPC-H query via DataFusion.
#[derive(Debug)]
pub struct QueryResult {
    pub query_name: &'static str,
    pub planning_time_us: u64,
    pub execution_time_us: u64,
    pub rows: usize,
    pub error: Option<String>,
}

impl QueryResult {
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// The TPC-H benchmark framework.
pub struct TpchBenchmark {
    data: TpchData,
    catalog: Arc<CatalogManager>,
    storage: Arc<ParquetStorage>,
}

impl TpchBenchmark {
    pub fn new_sf001() -> Self {
        let data = TpchData::generate_sf001();
        Self::new_with_data(data)
    }

    pub fn new_tiny() -> Self {
        let data = TpchData::generate_tiny();
        Self::new_with_data(data)
    }

    fn new_with_data(data: TpchData) -> Self {
        let catalog = Arc::new(CatalogManager::new());
        let storage = Arc::new(ParquetStorage::open("/tmp/tpch_storage").unwrap());

        catalog.create_database("tpch").unwrap();
        Self::setup_tables(&data, &catalog, &storage);

        Self {
            data,
            catalog,
            storage,
        }
    }

    fn setup_tables(data: &TpchData, catalog: &Arc<CatalogManager>, storage: &Arc<ParquetStorage>) {
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
            let table = make_table(id, "tpch", name, block);
            let _ = catalog.create_table("tpch", table);

            // Build Arrow schema and write to Parquet
            let arrow_fields: Vec<Field> = block
                .schema()
                .fields()
                .iter()
                .map(|f| {
                    Field::new(
                        &f.name,
                        fe_datafusion::types::to_arrow_data_type(&f.data_type),
                        f.nullable,
                    )
                })
                .collect();
            let arrow_schema = Arc::new(ArrowSchema::new(arrow_fields));

            let batch = block_convert::block_to_record_batch(block).unwrap();
            storage.create_table("tpch", name, arrow_schema).unwrap();
            storage.insert("tpch", name, batch).unwrap();
        }

        tracing::info!("TPC-H tables setup complete");
    }

    pub fn data(&self) -> &TpchData {
        &self.data
    }

    pub fn catalog(&self) -> &Arc<CatalogManager> {
        &self.catalog
    }

    pub fn storage(&self) -> &Arc<ParquetStorage> {
        &self.storage
    }

    pub fn datafusion_ctx(&self) -> SessionContext {
        use datafusion::common::config::ConfigOptions;

        let catalog_provider =
            ParquetCatalogProvider::new(self.catalog.clone(), self.storage.clone());

        let mut config = ConfigOptions::new();
        config.catalog.default_catalog = "tpch".to_string();
        config.catalog.default_schema = "tpch".to_string();

        let ctx = SessionContext::new_with_config(config.into());
        ctx.register_catalog("tpch", Arc::new(catalog_provider));
        ctx
    }

    pub fn run_query(&self, index: usize) -> QueryResult {
        let queries = queries::all_queries();
        if index == 0 || index > queries.len() {
            return QueryResult {
                query_name: "INVALID",
                planning_time_us: 0,
                execution_time_us: 0,
                rows: 0,
                error: Some(format!("Invalid query index: {}", index)),
            };
        }

        let (name, sql) = queries[index - 1];
        self.run_sql(name, sql)
    }

    pub fn run_sql(&self, name: &'static str, sql: &str) -> QueryResult {
        let start = std::time::Instant::now();

        let result = std::thread::scope(|s| {
            s.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    let ctx = self.datafusion_ctx();
                    match ctx.sql(sql).await {
                        Ok(df) => match df.collect().await {
                            Ok(batches) => {
                                let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                                Ok(rows)
                            }
                            Err(e) => Err(format!("Execution error: {}", e)),
                        },
                        Err(e) => Err(format!("SQL error: {}", e)),
                    }
                })
            })
            .join()
            .unwrap()
        });

        let elapsed = start.elapsed().as_micros() as u64;

        match result {
            Ok(rows) => QueryResult {
                query_name: name,
                planning_time_us: elapsed / 2,
                execution_time_us: elapsed / 2,
                rows,
                error: None,
            },
            Err(e) => QueryResult {
                query_name: name,
                planning_time_us: elapsed,
                execution_time_us: 0,
                rows: 0,
                error: Some(e),
            },
        }
    }

    pub fn run_all(&self) -> Vec<QueryResult> {
        (1..=22).map(|i| self.run_query(i)).collect()
    }
}

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
        tablet_id: id,
        id,
        name: name.to_string(),
        database: database.to_string(),
        columns,
        keys_type: fe_catalog::table::KeysType::Duplicate,
        unique_keys: vec![],
        partition_info: None,
        distribution_info: None,
        replication_num: 1,
        properties: HashMap::new(),
        row_count: block.num_rows() as u64,
        data_size: 0,
        stats: None,
        view_definition: None,
    }
}
