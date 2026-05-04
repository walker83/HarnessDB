use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use types::DataType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: u64,
    pub name: String,
    pub database: String,
    pub columns: Vec<TableColumn>,
    pub keys_type: KeysType,
    pub partition_info: Option<PartitionInfo>,
    pub distribution_info: Option<DistributionInfo>,
    pub replication_num: u32,
    pub properties: HashMap<String, String>,
    pub row_count: u64,
    pub data_size: u64,
    /// Collected statistics for CBO (NULL if never analyzed).
    pub stats: Option<crate::stats::TableStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub agg_type: Option<String>,
    pub comment: String,
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
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub id: u64,
    pub name: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionInfo {
    pub dist_type: String,
    pub columns: Vec<String>,
    pub buckets: u32,
}

impl Table {
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    pub fn get_column(&self, name: &str) -> Option<&TableColumn> {
        self.columns.iter().find(|c| c.name == name)
    }
}
