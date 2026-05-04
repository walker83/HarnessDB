use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::catalog::CatalogManager;
use crate::external::catalog::{Catalog, CatalogType, DatabaseInfo, TableInfo};

pub struct InternalCatalog {
    name: String,
    catalog: Arc<RwLock<CatalogManager>>,
}

impl InternalCatalog {
    pub fn new(name: &str, catalog: Arc<RwLock<CatalogManager>>) -> Self {
        Self {
            name: name.to_string(),
            catalog,
        }
    }
}

#[async_trait]
impl Catalog for InternalCatalog {
    fn get_type(&self) -> CatalogType {
        CatalogType::Internal
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn list_databases(&self) -> common::Result<Vec<String>> {
        let catalog = self.catalog.read().await;
        Ok(catalog.list_databases())
    }

    async fn get_database(&self, name: &str) -> common::Result<Option<DatabaseInfo>> {
        let catalog = self.catalog.read().await;
        Ok(catalog.get_database(name).map(|db| DatabaseInfo {
            name: db.name,
            properties: std::collections::HashMap::new(),
        }))
    }

    async fn list_tables(&self, database: &str) -> common::Result<Vec<String>> {
        let catalog = self.catalog.read().await;
        Ok(catalog.list_tables(database).unwrap_or_default())
    }

    async fn get_table(&self, database: &str, name: &str) -> common::Result<Option<TableInfo>> {
        let catalog = self.catalog.read().await;
        let table = catalog.get_table(database, name);

        Ok(table.map(|t| TableInfo {
            name: t.name,
            database: database.to_string(),
            catalog_name: self.name.clone(),
            columns: t.columns.into_iter().map(|c| crate::external::catalog::ColumnInfo {
                name: c.name,
                data_type: format!("{:?}", c.data_type),
                nullable: c.nullable,
                comment: Some(c.comment),
            }).collect(),
            location: None,
            file_format: crate::external::catalog::FileFormat::Parquet,
            partition_keys: vec![],
            properties: t.properties,
        }))
    }

    async fn refresh(&self) -> common::Result<()> {
        Ok(())
    }
}