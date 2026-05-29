use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PartitionType {
    Range,
    List,
    Hash,
}

impl PartitionType {
    pub fn as_str(&self) -> &str {
        match self {
            PartitionType::Range => "RANGE",
            PartitionType::List => "LIST",
            PartitionType::Hash => "HASH",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSpec {
    pub partition_type: PartitionType,
    pub columns: Vec<String>,
    pub partitions: Vec<PartitionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionEntry {
    pub id: u64,
    pub name: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    pub list_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMeta {
    pub id: u64,
    pub name: String,
    pub visible_version: u64,
    pub visible_version_time: u64,
    pub state: PartitionState,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PartitionState {
    Normal,
    Upgrade,
    Dropping,
}

impl PartitionSpec {
    pub fn new_range(columns: Vec<String>, partitions: Vec<PartitionEntry>) -> Self {
        Self {
            partition_type: PartitionType::Range,
            columns,
            partitions,
        }
    }

    pub fn new_list(columns: Vec<String>, partitions: Vec<PartitionEntry>) -> Self {
        Self {
            partition_type: PartitionType::List,
            columns,
            partitions,
        }
    }

    pub fn new_hash(columns: Vec<String>, num_partitions: usize) -> Self {
        let partitions: Vec<PartitionEntry> = (0..num_partitions)
            .map(|i| PartitionEntry {
                id: i as u64,
                name: format!("p{}", i),
                range_start: None,
                range_end: None,
                list_values: vec![],
            })
            .collect();
        Self {
            partition_type: PartitionType::Hash,
            columns,
            partitions,
        }
    }

    pub fn partition_names(&self) -> Vec<&str> {
        self.partitions.iter().map(|p| p.name.as_str()).collect()
    }

    pub fn get_partition(&self, name: &str) -> Option<&PartitionEntry> {
        self.partitions.iter().find(|p| p.name == name)
    }

    pub fn to_partition_info(&self) -> crate::table::PartitionInfo {
        crate::table::PartitionInfo {
            partition_type: self.partition_type.as_str().to_string(),
            columns: self.columns.clone(),
            partitions: self
                .partitions
                .iter()
                .map(|p| crate::table::Partition {
                    id: p.id,
                    name: p.name.clone(),
                    range_start: p.range_start.clone(),
                    range_end: p.range_end.clone(),
                })
                .collect(),
        }
    }

    pub fn from_partition_info(info: &crate::table::PartitionInfo) -> Self {
        let partition_type = match info.partition_type.to_uppercase().as_str() {
            "RANGE" => PartitionType::Range,
            "LIST" => PartitionType::List,
            "HASH" => PartitionType::Hash,
            _ => PartitionType::Hash,
        };
        Self {
            partition_type,
            columns: info.columns.clone(),
            partitions: info
                .partitions
                .iter()
                .map(|p| PartitionEntry {
                    id: p.id,
                    name: p.name.clone(),
                    range_start: p.range_start.clone(),
                    range_end: p.range_end.clone(),
                    list_values: vec![],
                })
                .collect(),
        }
    }
}

impl PartitionMeta {
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            visible_version: 0,
            visible_version_time: 0,
            state: PartitionState::Normal,
            properties: HashMap::new(),
        }
    }
}
