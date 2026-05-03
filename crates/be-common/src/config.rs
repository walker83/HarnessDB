use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeConfig {
    pub storage_root_path: String,
    pub mem_limit: String,
    pub http_port: u16,
    pub rpc_port: u16,
    pub heartbeat_port: u16,
    pub tablet_map_shard_size: usize,
    pub compaction_mem_limit: usize,
    pub write_buffer_size: usize,
}

impl Default for BeConfig {
    fn default() -> Self {
        Self {
            storage_root_path: "data/be/storage".into(),
            mem_limit: "80%".into(),
            http_port: 8060,
            rpc_port: 9060,
            heartbeat_port: 9050,
            tablet_map_shard_size: 32,
            compaction_mem_limit: 2 * 1024 * 1024 * 1024,
            write_buffer_size: 64 * 1024 * 1024,
        }
    }
}
