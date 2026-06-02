//! MongoDB document storage backend

use bson::Document;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

/// MongoDB collection storage
pub struct Collection {
    documents: DashMap<String, Document>,
}

impl Collection {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    pub fn insert(&self, id: String, doc: Document) {
        self.documents.insert(id, doc);
    }

    pub fn find(&self, filter: Option<&Document>) -> Vec<Document> {
        self.documents
            .iter()
            .filter(|entry| {
                if let Some(f) = filter {
                    Self::matches_filter(entry.value(), f)
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn update(&self, id: &str, update: &Document) -> bool {
        if let Some(mut entry) = self.documents.get_mut(id) {
            // Apply $set operations
            if let Some(set) = update.get_document("$set").ok() {
                for (key, value) in set {
                    entry.insert(key.clone(), value.clone());
                }
            }
            true
        } else {
            false
        }
    }

    pub fn delete(&self, id: &str) -> bool {
        self.documents.remove(id).is_some()
    }

    pub fn count(&self) -> usize {
        self.documents.len()
    }

    fn matches_filter(doc: &Document, filter: &Document) -> bool {
        for (key, value) in filter {
            match doc.get(key) {
                Some(doc_value) => {
                    if doc_value != value {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

impl Default for Collection {
    fn default() -> Self {
        Self::new()
    }
}

/// MongoDB database
pub struct Database {
    collections: DashMap<String, Arc<Collection>>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
        }
    }

    pub fn get_collection(&self, name: &str) -> Arc<Collection> {
        self.collections
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Collection::new()))
            .clone()
    }

    pub fn list_collections(&self) -> Vec<String> {
        self.collections.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

/// MongoDB storage backend with multiple databases
pub struct MongoDBStorage {
    databases: DashMap<String, Arc<Database>>,
}

impl MongoDBStorage {
    pub fn new() -> Self {
        Self {
            databases: DashMap::new(),
        }
    }

    pub fn get_database(&self, name: &str) -> Arc<Database> {
        self.databases
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Database::new()))
            .clone()
    }

    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for MongoDBStorage {
    fn default() -> Self {
        Self::new()
    }
}
