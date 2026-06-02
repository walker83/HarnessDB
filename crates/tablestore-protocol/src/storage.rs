//! TableStore wide-column storage backend

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// TableStore attribute value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Binary(Vec<u8>),
    Double(f64),
}

/// TableStore row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub primary_key: BTreeMap<String, AttributeValue>,
    pub attributes: HashMap<String, AttributeValue>,
}

/// TableStore table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub primary_key_columns: Vec<String>,
    pub defined_columns: Vec<String>,
    pub time_to_live: i32, // seconds, -1 means no expiration
    pub max_versions: i32,
}

impl TableSchema {
    pub fn new(primary_key_columns: Vec<String>) -> Self {
        Self {
            primary_key_columns,
            defined_columns: Vec::new(),
            time_to_live: -1,
            max_versions: 1,
        }
    }
}

/// TableStore table (wide-column store)
pub struct Table {
    schema: TableSchema,
    rows: DashMap<String, Row>, // key = concatenated primary key
}

impl Table {
    pub fn new(schema: TableSchema) -> Self {
        Self {
            schema,
            rows: DashMap::new(),
        }
    }

    pub fn put_row(&self, row: Row) {
        let key = self.build_key(&row.primary_key);
        self.rows.insert(key, row);
    }

    pub fn get_row(&self, primary_key: &BTreeMap<String, AttributeValue>) -> Option<Row> {
        let key = self.build_key(primary_key);
        self.rows.get(&key).map(|r| r.clone())
    }

    pub fn delete_row(&self, primary_key: &BTreeMap<String, AttributeValue>) -> bool {
        let key = self.build_key(primary_key);
        self.rows.remove(&key).is_some()
    }

    pub fn get_range(
        &self,
        start: &BTreeMap<String, AttributeValue>,
        end: &BTreeMap<String, AttributeValue>,
        limit: usize,
    ) -> Vec<Row> {
        let start_key = self.build_key(start);
        let end_key = self.build_key(end);

        self.rows
            .iter()
            .filter(|entry| {
                let key = entry.key();
                key >= &start_key && key < &end_key
            })
            .take(limit)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.rows.len()
    }

    pub fn schema(&self) -> &TableSchema {
        &self.schema
    }

    fn build_key(&self, primary_key: &BTreeMap<String, AttributeValue>) -> String {
        primary_key
            .iter()
            .map(|(k, v)| format!("{}={:?}", k, v))
            .collect::<Vec<_>>()
            .join("|")
    }
}

/// TableStore instance
pub struct Instance {
    tables: DashMap<String, Arc<Table>>,
}

impl Instance {
    pub fn new() -> Self {
        Self {
            tables: DashMap::new(),
        }
    }

    pub fn create_table(&self, name: &str, schema: TableSchema) {
        self.tables
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Table::new(schema)));
    }

    pub fn get_table(&self, name: &str) -> Option<Arc<Table>> {
        self.tables.get(name).map(|t| t.clone())
    }

    pub fn delete_table(&self, name: &str) -> bool {
        self.tables.remove(name).is_some()
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new()
    }
}

/// TableStore storage backend
pub struct TableStoreStorage {
    instances: DashMap<String, Arc<Instance>>,
}

impl TableStoreStorage {
    pub fn new() -> Self {
        Self {
            instances: DashMap::new(),
        }
    }

    pub fn get_instance(&self, name: &str) -> Arc<Instance> {
        self.instances
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Instance::new()))
            .clone()
    }

    pub fn list_instances(&self) -> Vec<String> {
        self.instances.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for TableStoreStorage {
    fn default() -> Self {
        Self::new()
    }
}
