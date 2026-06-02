//! Index management for RorisDB
//! Supports: BTree, Hash, Bitmap, Full-text, Vector (HNSW)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    FullText,
    Vector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub unique: bool,
    pub options: HashMap<String, String>,
}

pub struct IndexManager {
    indexes: HashMap<String, IndexDefinition>,
}

impl IndexManager {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
        }
    }

    pub fn create_index(&mut self, index: IndexDefinition) -> Result<(), String> {
        let key = format!("{}.{}", index.table_name, index.name);
        if self.indexes.contains_key(&key) {
            return Err(format!("Index {} already exists", index.name));
        }
        self.indexes.insert(key, index);
        Ok(())
    }

    pub fn drop_index(&mut self, table_name: &str, index_name: &str) -> Result<(), String> {
        let key = format!("{}.{}", table_name, index_name);
        if self.indexes.remove(&key).is_none() {
            return Err(format!("Index {} not found", index_name));
        }
        Ok(())
    }

    pub fn get_indexes(&self, table_name: &str) -> Vec<&IndexDefinition> {
        self.indexes
            .values()
            .filter(|idx| idx.table_name == table_name)
            .collect()
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}
