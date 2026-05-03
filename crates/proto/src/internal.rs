use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PExecPlanFragmentRequest {
    pub fragment_instance_id: String,
    pub plan: Vec<u8>,
    pub desc_tbl: Vec<u8>,
    pub params: Option<PQueryOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQueryOptions {
    pub query_timeout: i64,
    pub mem_limit: i64,
    pub query_type: String,
}
