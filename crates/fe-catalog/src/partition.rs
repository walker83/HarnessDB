use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub id: u64,
    pub name: String,
    pub visible_version: u64,
    pub visible_version_time: u64,
    pub state: PartitionState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PartitionState {
    Normal,
    Upgrade,
}
