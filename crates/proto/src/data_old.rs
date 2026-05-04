use serde::{Deserialize, Serialize};

// Status type for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OldStatus {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowBatch {
    pub rows: Vec<RowData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowData {
    pub values: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub status: OldStatus,
    pub row_batch: Option<RowBatch>,
    pub query_id: String,
}
