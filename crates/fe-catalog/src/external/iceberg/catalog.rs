use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::external::catalog::{Catalog, CatalogCache, CatalogType, ColumnInfo, DatabaseInfo, FileFormat, TableInfo};

pub struct IcebergCatalogConfig {
    pub uri: String,
    pub warehouse: String,
    pub auth_token: Option<String>,
    pub skip_signature: bool,
}

impl IcebergCatalogConfig {
    pub fn new(uri: impl Into<String>, warehouse: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            warehouse: warehouse.into(),
            auth_token: None,
            skip_signature: false,
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
        let config = IcebergCatalogConfig::new(uri, "mock_warehouse");
        Self {
            name: name.to_string(),
            config,
            rest_client: reqwest::Client::new(),
            cache: RwLock::new(CatalogCache::default()),
        }
    }

    async fn request<T: serde::de::DeserializeOwned>(&self, path: &str) -> common::Result<T> {
        let url = format!("{}{}", self.config.uri.trim_end_matches('/'), path);

        let mut builder = self.rest_client.get(&url);

        if let Some(ref token) = self.config.auth_token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        let response = builder.send().await
            .map_err(|e| common::DrorisError::Internal(format!("Iceberg REST request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(common::DrorisError::Internal(format!(
                "Iceberg REST API error: {} {}",
                status.as_u16(),
                response.text().await.unwrap_or_default()
            )));
        }

        response.json().await
            .map_err(|e| common::DrorisError::Internal(format!("Failed to parse Iceberg response: {}", e)))
    }

    async fn load_table_metadata(&self, namespace: &str, table: &str) -> common::Result<TableMetadata> {
        let path = format!("/v1/namespaces/{}/tables/{}", namespace, table);
        self.request::<TableLoadResponse>(&path).await
            .map(|r| r.metadata)
    }

    async fn list_namespaces(&self) -> common::Result<Vec<Namespace>> {
        #[derive(serde::Deserialize)]
        struct Response { namespaces: Vec<Namespace> }

        let resp: Response = self.request("/v1/namespaces").await?;
        Ok(resp.namespaces)
    }

    async fn list_tables_in_namespace(&self, namespace: &str) -> common::Result<Vec<String>> {
        #[derive(serde::Deserialize)]
        struct Response { identifiers: Vec<TableIdentifier> }

        let resp: Response = self.request(&format!("/v1/namespaces/{}/tables", namespace)).await?;
        Ok(resp.identifiers.into_iter().map(|t| t.name).collect())
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableLoadResponse {
    pub name: String,
    pub metadata: TableMetadata,
    pub metadata_location: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableMetadata {
    format_version: Option<i32>,
    table_uuid: Option<String>,
    location: Option<String>,
    schema: IcebergSchema,
    partition_specs: Vec<PartitionSpec>,
    current_snapshot_id: Option<i64>,
    snapshots: Option<Vec<Snapshot>>,
    manifest_list: Option<String>,
    properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcebergSchema {
    schema_id: Option<i32>,
    #[serde(default)]
    columns: Vec<IcebergColumn>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcebergColumn {
    id: Option<i32>,
    name: String,
    #[serde(rename = "type")]
    col_type: String,
    doc: Option<String>,
    required: Option<bool>,
}

impl IcebergColumn {
    fn to_column_info(&self) -> ColumnInfo {
        ColumnInfo {
            name: self.name.clone(),
            data_type: self.col_type.clone(),
            nullable: !self.required.unwrap_or(false),
            comment: self.doc.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionSpec {
    spec_id: Option<i32>,
    fields: Vec<PartitionField>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionField {
    source_id: Option<i32>,
    field_id: Option<i32>,
    name: String,
    transform: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    snapshot_id: Option<i64>,
    schema_id: Option<i32>,
    sequence_number: Option<i64>,
    manifest_list: Option<String>,
    summary: Option<HashMap<String, String>>,
    parent_snapshot_id: Option<i64>,
}

#[async_trait::async_trait]
impl Catalog for IcebergCatalog {
    fn get_type(&self) -> CatalogType {
        CatalogType::Iceberg
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn list_databases(&self) -> common::Result<Vec<String>> {
        let namespaces = self.list_namespaces().await?;
        Ok(namespaces.into_iter().map(|n| n.full_name()).collect())
    }

    async fn get_database(&self, name: &str) -> common::Result<Option<DatabaseInfo>> {
        Ok(Some(DatabaseInfo {
            name: name.to_string(),
            properties: HashMap::new(),
        }))
    }

    async fn list_tables(&self, database: &str) -> common::Result<Vec<String>> {
        self.list_tables_in_namespace(database).await
    }

    async fn get_table(&self, database: &str, name: &str) -> common::Result<Option<TableInfo>> {
        let metadata = match self.load_table_metadata(database, name).await {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let columns: Vec<ColumnInfo> = metadata.schema.columns
            .iter()
            .map(|c| c.to_column_info())
            .collect();

        let partition_keys: Vec<ColumnInfo> = metadata.partition_specs
            .first()
            .map(|spec| {
                spec.fields.iter()
                    .filter_map(|f| {
                        metadata.schema.columns.iter()
                            .find(|col| col.id == f.source_id)
                            .map(|col| col.to_column_info())
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some(TableInfo {
            name: name.to_string(),
            database: database.to_string(),
            catalog_name: self.name.clone(),
            columns,
            location: metadata.location,
            file_format: FileFormat::Parquet,
            partition_keys,
            properties: metadata.properties.unwrap_or_default(),
        }))
    }

    async fn refresh(&self) -> common::Result<()> {
        let mut cache = self.cache.write().await;
        *cache = CatalogCache::default();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_catalog() {
        let catalog = IcebergCatalog::mock("iceberg_test", "http://localhost:8080");

        assert_eq!(catalog.get_name(), "iceberg_test");
        assert_eq!(catalog.get_type(), CatalogType::Iceberg);
    }

    #[tokio::test]
    async fn test_namespace_full_name() {
        let ns = Namespace { name: vec!["db".to_string(), "schema".to_string()] };
        assert_eq!(ns.full_name(), "db.schema");
    }

    #[tokio::test]
    fn test_iceberg_column_to_column_info() {
        let col = IcebergColumn {
            id: Some(1),
            name: "id".to_string(),
            col_type: "long".to_string(),
            doc: Some("primary key".to_string()),
            required: Some(true),
        };

        let info = col.to_column_info();
        assert_eq!(info.name, "id");
        assert_eq!(info.data_type, "long");
        assert!(!info.nullable);
        assert_eq!(info.comment, Some("primary key".to_string()));
    }
}