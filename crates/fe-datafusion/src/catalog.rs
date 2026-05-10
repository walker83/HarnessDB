use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use datafusion::arrow::datatypes::Schema as ArrowSchema;
use datafusion::catalog::{CatalogProvider, SchemaProvider, TableProvider};
use datafusion::datasource::MemTable;
use datafusion::error::Result as DFResult;

use fe_catalog::CatalogManager;
use be_storage::{StorageEngine, tablet::{TabletSchema, TabletColumn}};
use types::DataType;

use crate::table_provider::RorisTableProvider;

// ---------------------------------------------------------------------------
// RorisSchemaProvider — per-database schema, backed by StorageEngine
// ---------------------------------------------------------------------------

pub struct RorisSchemaProvider {
    db_name: String,
    catalog: Arc<CatalogManager>,
    storage: Arc<StorageEngine>,
    mem_tables: Arc<DashMap<String, Arc<MemTable>>>,
}

impl std::fmt::Debug for RorisSchemaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RorisSchemaProvider")
            .field("db_name", &self.db_name)
            .finish()
    }
}

impl RorisSchemaProvider {
    pub fn new(
        db_name: String,
        catalog: Arc<CatalogManager>,
        storage: Arc<StorageEngine>,
        mem_tables: Arc<DashMap<String, Arc<MemTable>>>,
    ) -> Self {
        Self { db_name, catalog, storage, mem_tables }
    }

    fn get_table_schema(&self, table_name: &str) -> Option<arrow_schema::SchemaRef> {
        let table = self.catalog.get_table(&self.db_name, table_name)?;
        let fields: Vec<arrow_schema::Field> = table
            .columns
            .iter()
            .map(|c| {
                arrow_schema::Field::new(
                    &c.name,
                    crate::types::to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        Some(Arc::new(arrow_schema::Schema::new(fields)))
    }

    fn get_tablet_id(&self, table_name: &str) -> Option<u64> {
        let table = self.catalog.get_table(&self.db_name, table_name)?;
        Some(table.tablet_id)
    }
}

#[async_trait]
impl SchemaProvider for RorisSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        self.catalog
            .get_database(&self.db_name)
            .map(|db| db.table_names().into_iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    async fn table(
        &self,
        name: &str,
    ) -> DFResult<Option<Arc<dyn TableProvider>>> {
        // First check if there's a MemTable (for INSERT data)
        let mem_table_key = format!("{}.{}", self.db_name, name);
        if let Some(mem_table) = self.mem_tables.get(&mem_table_key) {
            return Ok(Some(mem_table.value().clone() as Arc<dyn TableProvider>));
        }

        let schema_ref = self.get_table_schema(name);
        let tablet_id = self.get_tablet_id(name);

        match (schema_ref, tablet_id) {
            (Some(schema), Some(tid)) => {
                let provider = RorisTableProvider::new(
                    self.storage.clone(),
                    tid,
                    schema,
                );
                Ok(Some(Arc::new(provider)))
            }
            _ => Ok(None),
        }
    }

    fn table_exist(&self, name: &str) -> bool {
        self.catalog.get_table(&self.db_name, name).is_some()
    }
}

// ---------------------------------------------------------------------------
// RorisCatalogProvider — top-level catalog
// ---------------------------------------------------------------------------

pub struct RorisCatalogProvider {
    pub catalog: Arc<CatalogManager>,
    pub storage: Arc<StorageEngine>,
    pub schemas: DashMap<String, Arc<RorisSchemaProvider>>,
    pub mem_tables: Arc<DashMap<String, Arc<MemTable>>>,
}

impl std::fmt::Debug for RorisCatalogProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RorisCatalogProvider").finish()
    }
}

impl RorisCatalogProvider {
    pub fn new(
        catalog: Arc<CatalogManager>,
        storage: Arc<StorageEngine>,
    ) -> Self {
        let schemas = DashMap::new();
        let mem_tables = Arc::new(DashMap::new());
        let db_names = catalog.list_databases();
        for name in &db_names {
            schemas.insert(
                name.clone(),
                Arc::new(RorisSchemaProvider::new(
                    name.clone(),
                    catalog.clone(),
                    storage.clone(),
                    mem_tables.clone(),
                )),
            );
        }
        Self {
            catalog,
            storage,
            schemas,
            mem_tables,
        }
    }

    pub fn create_database(&self, name: &str) {
        self.schemas.insert(
            name.to_string(),
            Arc::new(RorisSchemaProvider::new(
                name.to_string(),
                self.catalog.clone(),
                self.storage.clone(),
                self.mem_tables.clone(),
            )),
        );
    }

    pub fn drop_database(&self, name: &str) {
        self.schemas.remove(name);
    }

    pub fn create_table(&self, db: &str, name: &str, schema: Arc<ArrowSchema>) {
        let key = format!("{}.{}", db, name);
        let mem_table = MemTable::try_new(schema, vec![vec![]]).unwrap();
        self.mem_tables.insert(key, Arc::new(mem_table));
    }

    pub fn create_table_with_id(&self, tablet_id: u64, tablet_schema: TabletSchema) -> Result<(), String> {
        self.storage.create_tablet(tablet_id, tablet_schema)
            .map_err(|e| format!("Failed to create tablet: {}", e))
    }

    pub fn drop_table(&self, db: &str, name: &str) {
        let key = format!("{}.{}", db, name);
        self.mem_tables.remove(&key);
    }

    pub fn get_table_schema(&self, db: &str, name: &str) -> Option<Arc<ArrowSchema>> {
        let key = format!("{}.{}", db, name);
        self.mem_tables.get(&key).map(|r| r.value().schema())
    }

    pub fn get_mem_table(&self, db: &str, name: &str) -> Option<Arc<MemTable>> {
        let key = format!("{}.{}", db, name);
        self.mem_tables.get(&key).map(|r| r.value().clone())
    }
}

#[async_trait]
impl CatalogProvider for RorisCatalogProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema_names(&self) -> Vec<String> {
        self.schemas.iter().map(|r| r.key().clone()).collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        self.schemas
            .get(name)
            .map(|r| r.value().clone() as Arc<dyn SchemaProvider>)
    }
}
