//! Oracle storage backend

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Oracle table
pub struct Table {
    rows: DashMap<String, HashMap<String, String>>, // primary_key -> columns
}

impl Table {
    pub fn new() -> Self {
        Self {
            rows: DashMap::new(),
        }
    }

    pub fn insert(&self, pk: String, columns: HashMap<String, String>) {
        self.rows.insert(pk, columns);
    }

    pub fn select(&self, pk: Option<&str>) -> Vec<HashMap<String, String>> {
        if let Some(key) = pk {
            self.rows.get(key).map(|r| vec![r.clone()]).unwrap_or_default()
        } else {
            self.rows.iter().map(|r| r.value().clone()).collect()
        }
    }

    pub fn update(&self, pk: &str, columns: HashMap<String, String>) -> bool {
        if let Some(mut row) = self.rows.get_mut(pk) {
            for (k, v) in columns {
                row.insert(k, v);
            }
            true
        } else {
            false
        }
    }

    pub fn delete(&self, pk: &str) -> bool {
        self.rows.remove(pk).is_some()
    }

    pub fn count(&self) -> usize {
        self.rows.len()
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

/// Oracle schema (user)
pub struct Schema {
    tables: DashMap<String, Arc<Table>>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            tables: DashMap::new(),
        }
    }

    pub fn create_table(&self, name: &str) {
        self.tables.entry(name.to_string()).or_insert_with(|| Arc::new(Table::new()));
    }

    pub fn get_table(&self, name: &str) -> Option<Arc<Table>> {
        self.tables.get(name).map(|t| t.clone())
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// Oracle database instance
pub struct OracleStorage {
    schemas: DashMap<String, Arc<Schema>>,
}

impl OracleStorage {
    pub fn new() -> Self {
        let storage = Self {
            schemas: DashMap::new(),
        };

        // Create default schemas
        storage.schemas.insert("SYS".to_string(), Arc::new(Schema::new()));
        storage.schemas.insert("SYSTEM".to_string(), Arc::new(Schema::new()));
        storage.schemas.insert("HR".to_string(), Arc::new(Schema::new()));

        storage
    }

    pub fn get_schema(&self, name: &str) -> Arc<Schema> {
        self.schemas
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Schema::new()))
            .clone()
    }

    pub fn list_schemas(&self) -> Vec<String> {
        self.schemas.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for OracleStorage {
    fn default() -> Self {
        Self::new()
    }
}
