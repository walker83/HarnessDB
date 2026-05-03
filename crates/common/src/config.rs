use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeConfig {
    pub http_port: u16,
    pub rpc_port: u16,
    pub edit_log_port: u16,
    pub meta_dir: String,
    pub log_dir: String,
}

impl Default for FeConfig {
    fn default() -> Self {
        Self {
            http_port: 8030,
            rpc_port: 9020,
            edit_log_port: 9010,
            meta_dir: "data/fe/doris-meta".into(),
            log_dir: "log".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeConfig {
    pub http_port: u16,
    pub rpc_port: u16,
    pub heartbeat_port: u16,
    pub storage_root_path: String,
    pub log_dir: String,
    pub mem_limit: String,
}

impl Default for BeConfig {
    fn default() -> Self {
        Self {
            http_port: 8060,
            rpc_port: 9060,
            heartbeat_port: 9050,
            storage_root_path: "data/be/storage".into(),
            log_dir: "log".into(),
            mem_limit: "80%".into(),
        }
    }
}
