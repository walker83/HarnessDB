//! Partition management for HarnessDB
//! Supports: Range, List, Hash partitioning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionType {
    Range,
    List,
    Hash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionDefinition {
    pub name: String,
    pub partition_type: PartitionType,
    pub column: String,
    pub values: Vec<String>, // For List partition
    pub range_start: Option<String>, // For Range partition
    pub range_end: Option<String>, // For Range partition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePartition {
    pub table_name: String,
    pub partition_type: PartitionType,
    pub partition_column: String,
    pub partitions: Vec<PartitionDefinition>,
}

pub struct PartitionManager {
    table_partitions: HashMap<String, TablePartition>,
}

impl PartitionManager {
    pub fn new() -> Self {
        Self {
            table_partitions: HashMap::new(),
        }
    }

    pub fn create_partition(&mut self, partition: TablePartition) -> Result<(), String> {
        if self.table_partitions.contains_key(&partition.table_name) {
            return Err(format!("Table {} already partitioned", partition.table_name));
        }
        self.table_partitions.insert(partition.table_name.clone(), partition);
        Ok(())
    }

    pub fn get_partition(&self, table_name: &str) -> Option<&TablePartition> {
        self.table_partitions.get(table_name)
    }

    pub fn drop_partition(&mut self, table_name: &str) -> Result<(), String> {
        if self.table_partitions.remove(table_name).is_none() {
            return Err(format!("Table {} not partitioned", table_name));
        }
        Ok(())
    }
}

impl Default for PartitionManager {
    fn default() -> Self {
        Self::new()
    }
}
