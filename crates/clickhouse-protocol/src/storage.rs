//! ClickHouse column storage backend

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

/// ClickHouse table with columnar storage
pub struct Table {
    columns: HashMap<String, Vec<String>>,
    column_types: HashMap<String, String>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            column_types: HashMap::new(),
        }
    }

    pub fn create_column(&mut self, name: String, type_name: String) {
        self.columns.insert(name.clone(), Vec::new());
        self.column_types.insert(name, type_name);
    }

    pub fn insert_row(&mut self, values: HashMap<String, String>) {
        for (col_name, value) in values {
            if let Some(column) = self.columns.get_mut(&col_name) {
                column.push(value);
            }
        }
    }

    pub fn select_all(&self) -> Vec<HashMap<String, String>> {
        let mut rows = Vec::new();

        if self.columns.is_empty() {
            return rows;
        }

        // Get row count from first column
        let row_count = self.columns.values().next().map(|c| c.len()).unwrap_or(0);

        for i in 0..row_count {
            let mut row = HashMap::new();
            for (col_name, values) in &self.columns {
                if i < values.len() {
                    row.insert(col_name.clone(), values[i].clone());
                }
            }
            rows.push(row);
        }

        rows
    }

    pub fn count(&self) -> usize {
        self.columns.values().next().map(|c| c.len()).unwrap_or(0)
    }

    pub fn columns(&self) -> &HashMap<String, Vec<String>> {
        &self.columns
    }

    pub fn column_types(&self) -> &HashMap<String, String> {
        &self.column_types
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

/// ClickHouse database
pub struct Database {
    tables: DashMap<String, Table>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            tables: DashMap::new(),
        }
    }

    pub fn create_table(&self, name: &str) {
        self.tables.entry(name.to_string()).or_insert_with(Table::new);
    }

    pub fn get_table(&self, name: &str) -> Option<Table> {
        self.tables.get(name).map(|t| {
            let mut new_table = Table::new();
            new_table.columns = t.columns.clone();
            new_table.column_types = t.column_types.clone();
            new_table
        })
    }

    pub fn with_table_mut<F, R>(&self, name: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Table) -> R,
    {
        self.tables.get_mut(name).map(|mut t| f(&mut t))
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn drop_table(&self, name: &str) -> bool {
        self.tables.remove(name).is_some()
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

/// ClickHouse storage backend
pub struct ClickHouseStorage {
    databases: DashMap<String, Arc<Database>>,
}

impl ClickHouseStorage {
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

impl Default for ClickHouseStorage {
    fn default() -> Self {
        Self::new()
    }
}
