use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use datafusion::catalog::{CatalogProvider, SchemaProvider, TableProvider};
use datafusion::datasource::MemTable;
use datafusion::error::{DataFusionError, Result as DFResult};
use datafusion::arrow::datatypes::{Schema as ArrowSchema, SchemaRef};

use fe_catalog::CatalogManager;

use crate::types::to_arrow_data_type;

struct RorisSchemaProvider {
    db_name: String,
    catalog: Arc<std::sync::RwLock<CatalogManager>>,
    tables: DashMap<String, Arc<MemTable>>,
}

impl std::fmt::Debug for RorisSchemaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RorisSchemaProvider")
            .field("db_name", &self.db_name)
            .finish()
    }
}

impl RorisSchemaProvider {
    fn new(db_name: String, catalog: Arc<std::sync::RwLock<CatalogManager>>) -> Self {
        Self {
            db_name,
            catalog,
            tables: DashMap::new(),
        }
    }

    fn ensure_table(&self, table_name: &str) {
        if self.tables.contains_key(table_name) {
            return;
        }
        let catalog = self.catalog.read().unwrap();
        if let Some(table) = catalog.get_table(&self.db_name, table_name) {
            let arrow_fields: Vec<datafusion::arrow::datatypes::Field> = table
                .columns
                .iter()
                .map(|c| datafusion::arrow::datatypes::Field::new(
                    &c.name,
                    to_arrow_data_type(&c.data_type),
                    c.nullable,
                ))
                .collect();
            let schema = Arc::new(ArrowSchema::new(arrow_fields));
            if let Ok(mem_table) = MemTable::try_new(schema, vec![vec![]]) {
                self.tables.insert(table_name.to_string(), Arc::new(mem_table));
            }
        }
    }

    fn create_mem_table(&self, table_name: &str, schema: SchemaRef) {
        if let Ok(mem_table) = MemTable::try_new(schema, vec![vec![]]) {
            self.tables.insert(table_name.to_string(), Arc::new(mem_table));
        }
    }

    fn drop_table(&self, table_name: &str) {
        self.tables.remove(table_name);
    }
}

#[async_trait]
impl SchemaProvider for RorisSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|r| r.key().clone()).collect()
    }

    async fn table(
        &self,
        name: &str,
    ) -> DFResult<Option<Arc<dyn TableProvider>>> {
        self.ensure_table(name);
        if let Some(mem_table) = self.tables.get(name) {
            Ok(Some(mem_table.value().clone() as Arc<dyn TableProvider>))
        } else {
            Ok(None)
        }
    }

    fn table_exist(&self, name: &str) -> bool {
        self.ensure_table(name);
        self.tables.contains_key(name)
    }
}

pub struct RorisCatalogProvider {
    catalog: Arc<std::sync::RwLock<CatalogManager>>,
    schemas: DashMap<String, Arc<RorisSchemaProvider>>,
}

impl std::fmt::Debug for RorisCatalogProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RorisCatalogProvider").finish()
    }
}

impl RorisCatalogProvider {
    pub fn new(catalog: Arc<std::sync::RwLock<CatalogManager>>) -> Self {
        let schemas = DashMap::new();
        let db_names = {
            let cat = catalog.read().unwrap();
            cat.list_databases()
        };
        for name in &db_names {
            schemas.insert(
                name.clone(),
                Arc::new(RorisSchemaProvider::new(
                    name.clone(),
                    catalog.clone(),
                )),
            );
        }
        Self { catalog, schemas }
    }

    pub fn create_database(&self, name: &str) {
        self.schemas.insert(
            name.to_string(),
            Arc::new(RorisSchemaProvider::new(
                name.to_string(),
                self.catalog.clone(),
            )),
        );
    }

    pub fn drop_database(&self, name: &str) {
        self.schemas.remove(name);
    }

    pub fn create_table(&self, db_name: &str, table_name: &str, schema: SchemaRef) {
        if let Some(provider) = self.schemas.get(db_name) {
            provider.create_mem_table(table_name, schema);
        }
    }

    pub fn drop_table(&self, db_name: &str, table_name: &str) {
        if let Some(provider) = self.schemas.get(db_name) {
            provider.drop_table(table_name);
        }
    }

    pub fn get_mem_table(
        &self,
        db_name: &str,
        table_name: &str,
    ) -> Option<Arc<MemTable>> {
        self.schemas.get(db_name).and_then(|provider| {
            provider.ensure_table(table_name);
            provider.tables.get(table_name).map(|r| r.value().clone())
        })
    }
}

impl CatalogProvider for RorisCatalogProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema_names(&self) -> Vec<String> {
        self.schemas.iter().map(|r| r.key().clone()).collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        self.schemas.get(name).map(|r| Arc::clone(r.value()) as Arc<dyn SchemaProvider>)
    }
}
