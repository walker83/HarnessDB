use std::any::Any;
use std::sync::Arc;

use arrow_schema::Schema as ArrowSchema;
use async_trait::async_trait;
use dashmap::DashMap;
use datafusion::catalog::{CatalogProvider, SchemaProvider, TableProvider};
use datafusion::error::Result as DFResult;

use fe_catalog::CatalogManager;

use crate::ParquetStorage;
use crate::table_provider::ParquetTableProvider;

/// Per-database schema provider backed by ParquetStorage.
pub struct ParquetSchemaProvider {
    db_name: String,
    catalog: Arc<CatalogManager>,
    storage: Arc<ParquetStorage>,
}

impl std::fmt::Debug for ParquetSchemaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParquetSchemaProvider")
            .field("db_name", &self.db_name)
            .finish()
    }
}

impl ParquetSchemaProvider {
    pub fn new(
        db_name: String,
        catalog: Arc<CatalogManager>,
        storage: Arc<ParquetStorage>,
    ) -> Self {
        Self {
            db_name,
            catalog,
            storage,
        }
    }

    fn table_schema(&self, table_name: &str) -> Option<Arc<ArrowSchema>> {
        let table = self.catalog.get_table(&self.db_name, table_name)?;
        let fields: Vec<arrow_schema::Field> = table
            .columns
            .iter()
            .map(|c| {
                arrow_schema::Field::new(
                    &c.name,
                    fe_datafusion::types::to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        Some(Arc::new(ArrowSchema::new(fields)))
    }
}

#[async_trait]
impl SchemaProvider for ParquetSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        self.catalog
            .get_database(&self.db_name)
            .map(|db| {
                db.table_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn table(&self, name: &str) -> DFResult<Option<Arc<dyn TableProvider>>> {
        let schema = match self.table_schema(name) {
            Some(s) => s,
            None => return Ok(None),
        };

        let provider = ParquetTableProvider::new(
            self.storage.clone(),
            self.db_name.clone(),
            name.to_string(),
            schema,
        );
        Ok(Some(Arc::new(provider)))
    }

    fn table_exist(&self, name: &str) -> bool {
        self.catalog.get_table(&self.db_name, name).is_some()
    }
}

/// Top-level catalog provider backed by ParquetStorage.
pub struct ParquetCatalogProvider {
    pub catalog: Arc<CatalogManager>,
    pub storage: Arc<ParquetStorage>,
    pub schemas: DashMap<String, Arc<dyn datafusion::catalog::SchemaProvider>>,
}

impl std::fmt::Debug for ParquetCatalogProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParquetCatalogProvider").finish()
    }
}

impl ParquetCatalogProvider {
    pub fn new(catalog: Arc<CatalogManager>, storage: Arc<ParquetStorage>) -> Self {
        let schemas = DashMap::new();
        let db_names = catalog.list_databases();
        for name in &db_names {
            schemas.insert(
                name.clone(),
                Arc::new(ParquetSchemaProvider::new(
                    name.clone(),
                    catalog.clone(),
                    storage.clone(),
                )) as Arc<dyn datafusion::catalog::SchemaProvider>,
            );
        }
        // Register custom information_schema with MySQL-compatible types
        schemas.insert(
            "information_schema".to_string(),
            Arc::new(crate::information_schema::InformationSchemaProvider::new(
                catalog.clone(),
                storage.clone(),
            )) as Arc<dyn datafusion::catalog::SchemaProvider>,
        );
        Self {
            catalog,
            storage,
            schemas,
        }
    }

    pub fn create_database(&self, name: &str) {
        self.schemas.insert(
            name.to_string(),
            Arc::new(ParquetSchemaProvider::new(
                name.to_string(),
                self.catalog.clone(),
                self.storage.clone(),
            )) as Arc<dyn datafusion::catalog::SchemaProvider>,
        );
    }

    pub fn drop_database(&self, name: &str) {
        self.schemas.remove(name);
    }

    /// Create table: writes empty Parquet with schema.
    pub fn create_table(
        &self,
        db: &str,
        name: &str,
        schema: Arc<ArrowSchema>,
    ) -> Result<(), String> {
        self.storage
            .create_table(db, name, schema)
            .map_err(|e| format!("Failed to create table: {}", e))
    }

    /// Drop table: removes Parquet data.
    pub fn drop_table(&self, db: &str, name: &str) -> Result<(), String> {
        self.storage
            .drop_table(db, name)
            .map_err(|e| format!("Failed to drop table: {}", e))
    }

    /// Get the Arrow schema for a table from catalog metadata.
    pub fn get_table_schema(&self, db: &str, name: &str) -> Option<Arc<ArrowSchema>> {
        let table = self.catalog.get_table(db, name)?;
        let fields: Vec<arrow_schema::Field> = table
            .columns
            .iter()
            .map(|c| {
                arrow_schema::Field::new(
                    &c.name,
                    fe_datafusion::types::to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        Some(Arc::new(ArrowSchema::new(fields)))
    }

    pub fn storage(&self) -> &Arc<ParquetStorage> {
        &self.storage
    }
}

#[async_trait]
impl CatalogProvider for ParquetCatalogProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema_names(&self) -> Vec<String> {
        self.schemas.iter().map(|r| r.key().clone()).collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn datafusion::catalog::SchemaProvider>> {
        self.schemas.get(name).map(|r| r.value().clone())
    }
}
