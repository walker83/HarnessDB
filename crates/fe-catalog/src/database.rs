use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::table::Table;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub id: u64,
    pub name: String,
    pub tables: HashMap<String, Table>,
    pub properties: HashMap<String, String>,
    pub create_sql: Option<String>,
}

impl Database {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tables: HashMap::new(),
            properties: HashMap::new(),
            create_sql: None,
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }

    pub fn drop_table(&mut self, name: &str) -> Option<Table> {
        self.tables.remove(name)
    }

    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }
}
