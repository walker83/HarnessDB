use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::external_trait::{Catalog, CatalogCache, CatalogType, ColumnInfo, DatabaseInfo, FileFormat, TableInfo};

/// Iceberg REST Catalog configuration
#[derive(Debug, Clone)]
pub struct IcebergCatalogConfig {
    pub uri: String,
    pub warehouse: String,
    pub auth_token: Option<String>,
    pub prefix: Option<String>,
}

impl IcebergCatalogConfig {
    pub fn new(uri: impl Into<String>, warehouse: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            warehouse: warehouse.into(),
            auth_token: None,
            prefix: None,
        }
    }

    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }
}

/// Iceberg REST API response types
#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergNamespace {
    pub name: Vec<String>,
}

impl IcebergNamespace {
    pub fn full_name(&self) -> String {
        self.name.join(".")
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergTable {
    pub namespace: Vec<String>,
    pub name: String,
    #[serde(rename = "format-version")]
    pub format_version: Option<i32>,
    #[serde(rename = "table-uuid")]
    pub table_uuid: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergTableMetadata {
    #[serde(rename = "format-version")]
    pub format_version: i32,
    #[serde(rename = "table-uuid")]
    pub table_uuid: String,
    pub location: String,
    #[serde(rename = "last-sequence-number")]
    pub last_sequence_number: i64,
    pub schemas: Vec<IcebergSchema>,
    #[serde(rename = "current-schema-id")]
    pub current_schema_id: i32,
    pub partitions: Option<Vec<IcebergPartitionSpec>>,
    #[serde(rename = "default-sort-order")]
    pub default_sort_order: Option<IcebergSortOrder>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergSchema {
    pub schema_id: i32,
    pub fields: Vec<IcebergField>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergField {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: IcebergFieldType,
    pub required: bool,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum IcebergFieldType {
    Primitive {
        #[serde(rename = "type")]
        prim_type: String,
        #[serde(rename = "element-id")]
        element_id: Option<i32>,
        #[serde(rename = "key-id")]
        key_id: Option<i32>,
    },
    Struct {
        r#struct: Vec<IcebergField>,
        #[serde(rename = "struct-id")]
        struct_id: Option<i32>,
    },
    List {
        element: Box<IcebergFieldType>,
        #[serde(rename = "element-id")]
        element_id: Option<i32>,
    },
    Map {
        key: Box<IcebergFieldType>,
        value: Box<IcebergFieldType>,
        #[serde(rename = "key-id")]
        key_id: Option<i32>,
        #[serde(rename = "value-id")]
        value_id: Option<i32>,
    },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergPartitionSpec {
    pub spec_id: i32,
    pub fields: Vec<IcebergPartitionField>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergPartitionField {
    pub name: String,
    pub transform: String,
    pub source_id: i32,
    pub field_id: i32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergSortOrder {
    pub order_id: i32,
    pub fields: Vec<IcebergSortField>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergSortField {
    pub source_id: i32,
    pub transform: String,
    pub direction: Option<String>,
    pub null_order: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IcebergSnapshot {
    pub snapshot_id: i64,
    pub parent_snapshot_id: Option<i64>,
    pub sequence_number: i64,
    pub timestamp_ms: i64,
    pub manifest_list: String,
    pub summary: Option<HashMap<String, String>>,
}

/// Iceberg REST Catalog client
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
            .expect("failed to create HTTP client");

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

    async fn request<T: serde::de::DeserializerOwned>(&self, path: &str) -> Result<T, String> {
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

    async fn post<T: serde::de::DeserializerOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> Result<T, String> {
        let url = format!("{}{}", self.config.uri.trim_end_matches('/'), path);

        let mut builder = self.rest_client.post(&url);

        if let Some(ref token) = self.config.auth_token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        let response = builder
            .json(body)
            .send()
            .await
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

    fn map_iceberg_type(field_type: &IcebergFieldType) -> String {
        match field_type {
            IcebergFieldType::Primitive { prim_type, .. } => {
                match prim_type.as_str() {
                    "int" | "integer" => "Int32".to_string(),
                    "long" => "Int64".to_string(),
                    "float" => "Float32".to_string(),
                    "double" => "Float64".to_string(),
                    "decimal" => "Float64".to_string(),
                    "boolean" => "Boolean".to_string(),
                    "string" | "uuid" => "String".to_string(),
                    "date" => "String".to_string(),
                    "time" => "String".to_string(),
                    "timestamp" | "timestamptz" => "String".to_string(),
                    "binary" | "fixed" => "String".to_string(),
                    _ => "String".to_string(),
                }
            }
            IcebergFieldType::List { element, .. } => {
                format!("Array<{}>", Self::map_iceberg_type(element))
            }
            IcebergFieldType::Map { key, value, .. } => {
                format!("Map<{}, {}>", Self::map_iceberg_type(key), Self::map_iceberg_type(value))
            }
            IcebergFieldType::Struct { r#struct, .. } => {
                let fields: Vec<String> = r#struct.iter()
                    .map(|f| format!("{}: {}", f.name, Self::map_iceberg_type(&f.field_type)))
                    .collect();
                format!("Struct<{}>", fields.join(", "))
            }
        }
    }
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
        struct Response { namespaces: Vec<IcebergNamespace> }

        let resp: Response = self.request("/v1/namespaces").await?;
        Ok(resp.namespaces.into_iter().map(|n| DatabaseInfo {
            name: n.full_name(),
            properties: HashMap::new(),
        }).collect())
    }

    async fn get_database(&self, name: &str) -> Result<Option<DatabaseInfo>, String> {
        let path = if name.contains('.') {
            format!("/v1/namespaces/{}", name.replace('.', "/"))
        } else {
            format!("/v1/namespaces/{}", name)
        };

        #[derive(serde::Deserialize)]
        struct Response {
            namespace: Vec<String>,
            properties: HashMap<String, String>,
        }

        match self.request::<Response>(&path).await {
            Ok(resp) => Ok(Some(DatabaseInfo {
                name: resp.namespace.join("."),
                properties: resp.properties,
            })),
            Err(_) => Ok(None),
        }
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<String>, String> {
        let path = if database.contains('.') {
            format!("/v1/namespaces/{}/tables", database.replace('.', "/"))
        } else {
            format!("/v1/namespaces/{}/tables", database)
        };

        #[derive(serde::Deserialize)]
        struct Response { identifiers: Vec<IcebergTable> }

        let resp: Response = self.request(&path).await?;
        Ok(resp.identifiers.into_iter().map(|t| t.name).collect())
    }

    async fn get_table(&self, database: &str, table: &str) -> Result<Option<TableInfo>, String> {
        let path = if database.contains('.') {
            format!("/v1/namespaces/{}/tables/{}", database.replace('.', "/"), table)
        } else {
            format!("/v1/namespaces/{}/tables/{}", database, table)
        };

        #[derive(serde::Deserialize)]
        struct Response {
            name: String,
            namespace: Vec<String>,
            #[serde(rename = "table-uuid")]
            table_uuid: Option<String>,
            location: Option<String>,
            #[serde(rename = "current-schema-id")]
            current_schema_id: Option<i32>,
            schemas: Option<Vec<IcebergSchema>>,
            #[serde(rename = "partition-specs")]
            partition_specs: Option<Vec<IcebergPartitionSpec>>,
        }

        let resp: Response = self.request(&path).await?;

        // Get the table metadata for column info
        let metadata_path = format!("{}/metadata", path);
        let metadata: Option<IcebergTableMetadata> = self.request(&metadata_path).await.ok();

        let columns = if let Some(ref meta) = metadata {
            if let Some(current_schema) = meta.schemas.iter().find(|s| s.schema_id == meta.current_schema_id) {
                current_schema.fields.iter().map(|f| ColumnInfo {
                    name: f.name.clone(),
                    data_type: Self::map_iceberg_type(&f.field_type),
                    nullable: !f.required,
                    comment: f.doc.clone(),
                }).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let partition_keys = if let Some(ref meta) = metadata {
            meta.partitions.as_ref().map(|specs| {
                specs.iter().flat_map(|spec| {
                    spec.fields.iter().map(|f| {
                        let col_name = if let Some(ref schema) = meta.schemas.iter().find(|s| s.schema_id == meta.current_schema_id) {
                            schema.fields.iter().find(|field| field.id == f.source_id)
                                .map(|field| field.name.clone())
                                .unwrap_or_else(|| f.name.clone())
                        } else {
                            f.name.clone()
                        };
                        ColumnInfo {
                            name: col_name,
                            data_type: "".to_string(),
                            nullable: false,
                            comment: None,
                        }
                    }).collect::<Vec<_>>()
                }).collect()
            }).unwrap_or_default()
        } else {
            vec![]
        };

        Ok(Some(TableInfo {
            name: table.to_string(),
            database: database.to_string(),
            catalog_name: self.name.clone(),
            columns,
            location: metadata.as_ref().map(|m| m.location.clone()).or(resp.location),
            file_format: Some(FileFormat::Iceberg),
            partition_keys,
            properties: HashMap::new(),
        }))
    }

    async fn refresh(&self) -> Result<(), String> {
        let mut cache = self.cache.write().await;
        *cache = CatalogCache::default();
        Ok(())
    }
}

/// Iceberg Time Travel support
pub mod time_travel {
    use super::*;

    /// Options for Iceberg time travel queries
    #[derive(Debug, Clone)]
    pub struct TimeTravelOptions {
        /// Snapshot ID to use
        pub snapshot_id: Option<i64>,
        /// Timestamp in milliseconds
        pub timestamp_ms: Option<i64>,
        /// Version string (e.g., "version-hint")
        pub version: Option<String>,
    }

    impl TimeTravelOptions {
        pub fn as_of_snapshot(snapshot_id: i64) -> Self {
            Self {
                snapshot_id: Some(snapshot_id),
                timestamp_ms: None,
                version: None,
            }
        }

        pub fn as_of_timestamp(timestamp_ms: i64) -> Self {
            Self {
                snapshot_id: None,
                timestamp_ms: Some(timestamp_ms),
                version: None,
            }
        }

        pub fn as_of_version(version: impl Into<String>) -> Self {
            Self {
                snapshot_id: None,
                timestamp_ms: None,
                version: Some(version.into()),
            }
        }
    }

    /// Query Iceberg table at a specific point in time
    pub async fn query_at_snapshot(
        catalog: &IcebergCatalog,
        namespace: &str,
        table: &str,
        options: &TimeTravelOptions,
    ) -> Result<Vec<IcebergSnapshot>, String> {
        let base_path = format!("/v1/namespaces/{}/tables/{}", namespace.replace('.', "/"), table);

        #[derive(serde::Deserialize)]
        struct SnapshotsResponse {
            snapshots: Vec<IcebergSnapshot>,
        }

        let path = format!("{}/snapshots", base_path);
        let resp: SnapshotsResponse = catalog.request(&path).await?;

        let filtered: Vec<IcebergSnapshot> = if let Some(snapshot_id) = options.snapshot_id {
            resp.snapshots.into_iter()
                .filter(|s| s.snapshot_id == snapshot_id)
                .collect()
        } else if let Some(ts) = options.timestamp_ms {
            resp.snapshots.into_iter()
                .filter(|s| s.timestamp_ms <= ts)
                .collect()
        } else {
            resp.snapshots
        };

        Ok(filtered)
    }

    /// Get the manifest file list for a snapshot
    pub async fn get_manifest_list(
        catalog: &IcebergCatalog,
        namespace: &str,
        table: &str,
        snapshot_id: i64,
    ) -> Result<Vec<String>, String> {
        let snapshots = query_at_snapshot(
            catalog,
            namespace,
            table,
            &TimeTravelOptions::as_of_snapshot(snapshot_id),
        ).await?;

        if let Some(snapshot) = snapshots.first() {
            Ok(vec![snapshot.manifest_list.clone()])
        } else {
            Err(format!("Snapshot {} not found", snapshot_id))
        }
    }
}