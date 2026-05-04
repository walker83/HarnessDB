use super::Status;

#[derive(Debug, Clone)]
pub struct HeartbeatRequest {
    pub be_host: String,
    pub be_port: i32,
    pub http_port: i32,
    pub be_version: String,
}

#[derive(Debug, Clone)]
pub struct HeartbeatResponse {
    pub status: Status,
    pub master_info: Option<String>,
}
