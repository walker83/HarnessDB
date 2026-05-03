use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletMeta {
    pub tablet_id: u64,
    pub table_id: u64,
    pub partition_id: u64,
    pub index_id: u64,
    pub schema_version: u64,
    pub min_version: u64,
    pub max_version: u64,
    pub persistent_index: bool,
    pub storage_type: StorageType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StorageType {
    Local,
    Remote,
    Mixed,
}
