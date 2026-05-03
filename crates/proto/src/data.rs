use serde::{Deserialize, Serialize};

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
    pub status: super::status::Status,
    pub row_batch: Option<RowBatch>,
    pub query_id: String,
}
