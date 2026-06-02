//! MaxCompute Tunnel JSON response models.
//!
//! Tunnel responses use JSON (not XML). These types serialize to the expected
//! JSON structure returned by the MaxCompute Tunnel API.

use serde::Serialize;

use crate::tunnel::schema::TunnelSchema;

// ============================================================================
// Schema serialization for JSON
// ============================================================================

#[derive(Debug, Serialize)]
pub(crate) struct JsonColumn {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nullable: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct JsonSchema {
    columns: Vec<JsonColumn>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    partitionKeys: Vec<JsonColumn>,
}

impl From<&TunnelSchema> for JsonSchema {
    fn from(schema: &TunnelSchema) -> Self {
        let to_json_col = |c: &crate::tunnel::schema::TunnelColumn| JsonColumn {
            name: c.name.clone(),
            col_type: c.odps_type.clone(),
            comment: c.comment.clone(),
            nullable: Some(c.nullable),
        };
        JsonSchema {
            columns: schema.columns.iter().map(to_json_col).collect(),
            partitionKeys: schema.partition_keys.iter().map(to_json_col).collect(),
        }
    }
}

// ============================================================================
// Upload session responses
// ============================================================================

/// Response for POST ...?uploads (Create Upload Session).
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateUploadResponse {
    pub upload_id: String,
    pub status: String,
    #[serde(flatten)]
    pub schema: JsonSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_field_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_name: Option<String>,
}

impl CreateUploadResponse {
    pub fn new(upload_id: String, schema: TunnelSchema) -> Self {
        Self {
            upload_id,
            status: "NORMAL".to_string(),
            schema: JsonSchema::from(&schema),
            max_field_size: Some(0),
            quota_name: None,
        }
    }
}

/// Response for GET ...?uploadid={id} (Reload Upload Session).
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReloadUploadResponse {
    pub upload_id: String,
    pub status: String,
    pub uploaded_block_list: Vec<BlockIdEntry>,
    #[serde(flatten)]
    pub schema: JsonSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockIdEntry {
    pub block_id: u64,
}

impl ReloadUploadResponse {
    pub fn new(upload_id: String, schema: TunnelSchema, block_ids: Vec<u64>) -> Self {
        Self {
            upload_id,
            status: "NORMAL".to_string(),
            uploaded_block_list: block_ids.into_iter().map(|id| BlockIdEntry { block_id: id }).collect(),
            schema: JsonSchema::from(&schema),
            quota_name: None,
        }
    }
}

/// Response for POST ...?uploadid={id} (Commit Upload).
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommitUploadResponse {
    pub status: String,
    pub uploaded_block_list: Vec<BlockIdEntry>,
}

impl CommitUploadResponse {
    pub fn new(block_ids: Vec<u64>) -> Self {
        Self {
            status: "NORMAL".to_string(),
            uploaded_block_list: block_ids.into_iter().map(|id| BlockIdEntry { block_id: id }).collect(),
        }
    }
}

// ============================================================================
// Download session responses
// ============================================================================

/// Response for POST ...?downloads (Create Download Session).
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateDownloadResponse {
    pub download_id: String,
    pub status: String,
    pub record_count: u64,
    #[serde(flatten)]
    pub schema: JsonSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_name: Option<String>,
    pub support_read_by_raw_size: bool,
}

impl CreateDownloadResponse {
    pub fn new(download_id: String, schema: TunnelSchema, record_count: u64) -> Self {
        Self {
            download_id,
            status: "NORMAL".to_string(),
            record_count,
            schema: JsonSchema::from(&schema),
            quota_name: None,
            support_read_by_raw_size: false,
        }
    }
}

/// Response for GET ...?downloadid={id} (Reload Download Session).
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReloadDownloadResponse {
    pub download_id: String,
    pub status: String,
    pub record_count: u64,
    #[serde(flatten)]
    pub schema: JsonSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_name: Option<String>,
    pub support_read_by_raw_size: bool,
}

impl ReloadDownloadResponse {
    pub fn new(download_id: String, schema: TunnelSchema, record_count: u64) -> Self {
        Self {
            download_id,
            status: "NORMAL".to_string(),
            record_count,
            schema: JsonSchema::from(&schema),
            quota_name: None,
            support_read_by_raw_size: false,
        }
    }
}

// ============================================================================
// Tunnel endpoint discovery
// ============================================================================

/// Response for GET /api/projects/{project}/tunnel
/// Returns plain text, not JSON. But this type is here for completeness.
pub fn tunnel_endpoint_response(bind_addr: &str, port: u16) -> String {
    format!("{}:{}", bind_addr, port)
}

// ============================================================================
// Error response
// ============================================================================

/// Tunnel error response (JSON).
#[derive(Debug, Serialize)]
pub struct TunnelError {
    pub code: String,
    pub message: String,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

impl TunnelError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunnel::schema::{TunnelColumn, TunnelSchema};

    fn make_test_schema() -> TunnelSchema {
        TunnelSchema {
            columns: vec![
                TunnelColumn {
                    name: "id".into(),
                    odps_type: "BIGINT".into(),
                    nullable: false,
                    comment: Some("primary key".into()),
                },
                TunnelColumn {
                    name: "name".into(),
                    odps_type: "STRING".into(),
                    nullable: true,
                    comment: None,
                },
            ],
            partition_keys: vec![TunnelColumn {
                name: "ds".into(),
                odps_type: "STRING".into(),
                nullable: true,
                comment: None,
            }],
        }
    }

    #[test]
    fn test_create_upload_response_json() {
        let resp = CreateUploadResponse::new("upload-123".to_string(), make_test_schema());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("upload-123"));
        assert!(json.contains("NORMAL"));
        assert!(json.contains("BIGINT"));
        assert!(json.contains("id"));
    }

    #[test]
    fn test_create_download_response_json() {
        let resp = CreateDownloadResponse::new("dl-456".to_string(), make_test_schema(), 1000);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("dl-456"));
        assert!(json.contains("1000"));
        assert!(json.contains("RecordCount"));
    }

    #[test]
    fn test_commit_upload_response_json() {
        let resp = CommitUploadResponse::new(vec![0, 1, 2]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("NORMAL"));
        assert!(json.contains("0"));
    }

    #[test]
    fn test_reload_upload_response_json() {
        let resp = ReloadUploadResponse::new("upload-789".to_string(), make_test_schema(), vec![0, 1]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("upload-789"));
        assert!(json.contains("BlockList"));
    }

    #[test]
    fn test_tunnel_error_json() {
        let err = TunnelError::new("InvalidArgument", "missing uploadid parameter");
        let json = err.to_json();
        assert!(json.contains("InvalidArgument"));
        assert!(json.contains("missing uploadid"));
        assert!(json.contains("RequestId"));
    }

    #[test]
    fn test_tunnel_endpoint_response() {
        let resp = tunnel_endpoint_response("127.0.0.1", 9031);
        assert_eq!(resp, "127.0.0.1:9031");
    }

    #[test]
    fn test_json_schema_includes_partition_keys() {
        let resp = CreateUploadResponse::new("test".into(), make_test_schema());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("partitionKeys"));
        assert!(json.contains("ds"));
    }

    #[test]
    fn test_json_schema_serialization() {
        let schema = make_test_schema();
        let json_schema = JsonSchema::from(&schema);
        let json = serde_json::to_string(&json_schema).unwrap();
        assert!(json.contains("columns"));
        assert!(json.contains("partitionKeys"));
        // Verify column types
        assert!(json.contains("BIGINT"));
        assert!(json.contains("STRING"));
    }
}
