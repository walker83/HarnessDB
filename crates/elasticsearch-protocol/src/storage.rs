//! Elasticsearch index storage backend

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Elasticsearch document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

/// Elasticsearch index
pub struct Index {
    documents: DashMap<String, Document>,
    mapping: HashMap<String, String>, // field -> type
}

impl Index {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            mapping: HashMap::new(),
        }
    }

    pub fn set_mapping(&mut self, field: String, type_name: String) {
        self.mapping.insert(field, type_name);
    }

    pub fn index_document(&self, id: String, doc: Document) {
        self.documents.insert(id, doc);
    }

    pub fn get_document(&self, id: &str) -> Option<Document> {
        self.documents.get(id).map(|d| d.clone())
    }

    pub fn delete_document(&self, id: &str) -> bool {
        self.documents.remove(id).is_some()
    }

    pub fn search(&self, query: &serde_json::Value) -> Vec<(String, Document)> {
        // Simple search - return all documents for now
        // In a real implementation, parse the query DSL
        self.documents
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.documents.len()
    }

    pub fn mapping(&self) -> &HashMap<String, String> {
        &self.mapping
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

/// Elasticsearch cluster
pub struct ElasticsearchStorage {
    indices: DashMap<String, Arc<Index>>,
}

impl ElasticsearchStorage {
    pub fn new() -> Self {
        Self {
            indices: DashMap::new(),
        }
    }

    pub fn create_index(&self, name: &str) {
        self.indices
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Index::new()));
    }

    pub fn get_index(&self, name: &str) -> Option<Arc<Index>> {
        self.indices.get(name).map(|i| i.clone())
    }

    pub fn delete_index(&self, name: &str) -> bool {
        self.indices.remove(name).is_some()
    }

    pub fn list_indices(&self) -> Vec<String> {
        self.indices.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn index_exists(&self, name: &str) -> bool {
        self.indices.contains_key(name)
    }
}

impl Default for ElasticsearchStorage {
    fn default() -> Self {
        Self::new()
    }
}
