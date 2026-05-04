use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatalogType {
    Internal,
    Hive,
    Iceberg,
    Hudi,
    JDBC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub database: String,
    pub catalog_name: String,
    pub columns: Vec<ColumnInfo>,
    pub location: Option<String>,
    pub file_format: Option<FileFormat>,
    pub partition_keys: Vec<ColumnInfo>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileFormat {
    Parquet,
    Orc,
    Avro,
    Iceberg,
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

#[async_trait::async_trait]
pub trait Catalog: Send + Sync {
    fn catalog_type(&self) -> CatalogType;
    fn get_name(&self) -> &str;
    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, String>;
    async fn get_database(&self, name: &str) -> Result<Option<DatabaseInfo>, String>;
    async fn list_tables(&self, database: &str) -> Result<Vec<String>, String>;
    async fn get_table(&self, database: &str, table: &str) -> Result<Option<TableInfo>, String>;
    async fn refresh(&self) -> Result<(), String>;
}

pub struct InternalCatalog {
    name: String,
    manager: std::sync::Arc<crate::CatalogManager>,
}

impl InternalCatalog {
    pub fn new(name: &str, manager: std::sync::Arc<crate::CatalogManager>) -> Self {
        Self {
            name: name.to_string(),
            manager,
        }
    }
}

#[async_trait::async_trait]
impl Catalog for InternalCatalog {
    fn catalog_type(&self) -> CatalogType {
        CatalogType::Internal
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, String> {
        Ok(self.manager.list_databases().into_iter().map(|name| DatabaseInfo {
            name,
            properties: HashMap::new(),
        }).collect())
    }

    async fn get_database(&self, name: &str) -> Result<Option<DatabaseInfo>, String> {
        Ok(self.manager.get_database(name).map(|db| DatabaseInfo {
            name: db.name,
            properties: HashMap::new(),
        }))
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<String>, String> {
        self.manager.list_tables(database)
            .ok_or(format!("Database {} not found", database))
    }

    async fn get_table(&self, database: &str, table: &str) -> Result<Option<TableInfo>, String> {
        Ok(self.manager.get_table(database, table).map(|t| TableInfo {
            name: t.name.clone(),
            database: t.database.clone(),
            catalog_name: self.name.clone(),
            columns: t.columns.iter().map(|c| ColumnInfo {
                name: c.name.clone(),
                data_type: format!("{:?}", c.data_type),
                nullable: c.nullable,
                comment: Some(c.comment.clone()),
            }).collect(),
            location: None,
            file_format: Some(FileFormat::Parquet),
            partition_keys: vec![],
            properties: t.properties.clone(),
        }))
    }

    async fn refresh(&self) -> Result<(), String> {
        Ok(())
    }
}

pub mod iceberg {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub struct IcebergCatalogConfig {
        pub uri: String,
        pub warehouse: String,
        pub auth_token: Option<String>,
    }

    impl IcebergCatalogConfig {
        pub fn new(uri: impl Into<String>, warehouse: impl Into<String>) -> Self {
            Self {
                uri: uri.into(),
                warehouse: warehouse.into(),
                auth_token: None,
            }
        }

        pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
            self.auth_token = Some(token.into());
            self
        }
    }

    pub struct IcebergCatalog {
        name: String,
        config: IcebergCatalogConfig,
        rest_client: reqwest::Client,
        cache: RwLock<CatalogCache>,
    }

    impl IcebergCatalog {
        pub fn new(name: &str, config: IcebergCatalogConfig) -> Self {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to create http client");

            Self {
                name: name.to_string(),
                config,
                rest_client: client,
                cache: RwLock::new(CatalogCache::default()),
            }
        }

        pub fn mock(name: &str, uri: &str) -> Self {
            Self::new(name, IcebergCatalogConfig::new(uri, "mock_warehouse"))
        }

        async fn request<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
            let url = format!("{}{}", self.config.uri.trim_end_matches('/'), path);

            let mut builder = self.rest_client.get(&url);

            if let Some(ref token) = self.config.auth_token {
                builder = builder.header("Authorization", format!("Bearer {}", token));
            }

            let response = builder.send().await
                .map_err(|e| format!("Iceberg REST request failed: {}", e))?;

            let status = response.status();
            if !status.is_success() {
                return Err(format!(
                    "Iceberg REST API error: {} {}",
                    status.as_u16(),
                    response.text().await.unwrap_or_default()
                ));
            }

            response.json().await
                .map_err(|e| format!("Failed to parse Iceberg response: {}", e))
        }
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct Namespace {
        pub name: Vec<String>,
    }

    impl Namespace {
        pub fn full_name(&self) -> String {
            self.name.join(".")
        }
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct TableIdentifier {
        pub namespace: Vec<String>,
        pub name: String,
    }

    #[async_trait::async_trait]
    impl Catalog for IcebergCatalog {
        fn catalog_type(&self) -> CatalogType {
            CatalogType::Iceberg
        }

        fn get_name(&self) -> &str {
            &self.name
        }

        async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, String> {
            #[derive(serde::Deserialize)]
            struct Response { namespaces: Vec<Namespace> }

            let resp: Response = self.request("/v1/namespaces").await?;
            Ok(resp.namespaces.into_iter().map(|n| DatabaseInfo {
                name: n.full_name(),
                properties: HashMap::new(),
            }).collect())
        }

        async fn get_database(&self, name: &str) -> Result<Option<DatabaseInfo>, String> {
            Ok(Some(DatabaseInfo {
                name: name.to_string(),
                properties: HashMap::new(),
            }))
        }

        async fn list_tables(&self, _database: &str) -> Result<Vec<String>, String> {
            Ok(vec![])
        }

        async fn get_table(&self, _database: &str, _table: &str) -> Result<Option<TableInfo>, String> {
            Ok(None)
        }

        async fn refresh(&self) -> Result<(), String> {
            let mut cache = self.cache.write().await;
            *cache = CatalogCache::default();
            Ok(())
        }
    }
}