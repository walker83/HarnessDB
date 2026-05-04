use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogType {
    Internal,
    Iceberg,
    Hive,
    Hudi,
}

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FileFormat {
    Parquet,
    Orc,
    Avro,
    Iceberg,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub database: String,
    pub catalog_name: String,
    pub columns: Vec<ColumnInfo>,
    pub location: Option<String>,
    pub file_format: FileFormat,
    pub partition_keys: Vec<ColumnInfo>,
    pub properties: HashMap<String, String>,
}

#[async_trait]
pub trait Catalog: Send + Sync {
    fn get_type(&self) -> CatalogType;
    fn get_name(&self) -> &str;

    async fn list_databases(&self) -> common::Result<Vec<String>>;
    async fn get_database(&self, name: &str) -> common::Result<Option<DatabaseInfo>>;
    async fn list_tables(&self, database: &str) -> common::Result<Vec<String>>;
    async fn get_table(&self, database: &str, name: &str) -> common::Result<Option<TableInfo>>;

    async fn refresh(&self) -> common::Result<()>;
}

pub struct CatalogCache {
    pub databases: HashMap<String, DatabaseInfo>,
    pub tables: HashMap<String, TableInfo>,
}

impl Default for CatalogCache {
    fn default() -> Self {
        Self {
            databases: HashMap::new(),
            tables: HashMap::new(),
        }
    }
}