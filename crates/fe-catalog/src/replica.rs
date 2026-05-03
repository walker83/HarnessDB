use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    pub id: u64,
    pub backend_id: u64,
    pub version: u64,
    pub version_hash: u64,
    pub data_size: u64,
    pub row_count: u64,
    pub state: ReplicaState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReplicaState {
    Normal,
    Clone,
    Alter,
    Decommission,
}
