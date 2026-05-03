use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub db_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
    pub keys_type: KeysType,
    pub partition_info: Option<PartitionInfo>,
    pub distribution_info: DistributionInfo,
    pub replication_num: i32,
    pub properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub agg_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeysType {
    Duplicate,
    Aggregate,
    Unique,
    Primary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition_type: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionInfo {
    pub dist_type: String,
    pub columns: Vec<String>,
    pub buckets: i32,
}
