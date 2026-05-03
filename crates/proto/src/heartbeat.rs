use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub be_host: String,
    pub be_port: i32,
    pub http_port: i32,
    pub be_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub status: super::status::Status,
    pub master_info: Option<String>,
}
