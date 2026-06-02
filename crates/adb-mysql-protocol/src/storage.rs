//! AnalyticDB MySQL storage backend

use dashmap::DashMap;
use std::sync::Arc;

/// Column-oriented storage for analytical queries
pub struct AdbMysqlStorage {
    databases: DashMap<String, Arc<AdbMysqlDatabase>>,
}

pub struct AdbMysqlDatabase {
    tables: DashMap<String, Arc<AdbMysqlTable>>,
}

pub struct AdbMysqlTable {
    columns: Vec<String>,
    rows: DashMap<u64, Vec<String>>, // row_id -> column values
}

impl AdbMysqlStorage {
    pub fn new() -> Self {
        Self {
            databases: DashMap::new(),
        }
    }

    pub fn create_database(&self, name: &str) {
        self.databases.insert(name.to_string(), Arc::new(AdbMysqlDatabase::new()));
    }

    pub fn get_database(&self, name: &str) -> Option<Arc<AdbMysqlDatabase>> {
        self.databases.get(name).map(|d| d.value().clone())
    }

    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter().map(|d| d.key().clone()).collect()
    }
}

impl AdbMysqlDatabase {
    pub fn new() -> Self {
        Self {
            tables: DashMap::new(),
        }
    }

    pub fn create_table(&self, name: &str, columns: Vec<String>) {
        self.tables.insert(
            name.to_string(),
            Arc::new(AdbMysqlTable::new(columns)),
        );
    }

    pub fn get_table(&self, name: &str) -> Option<Arc<AdbMysqlTable>> {
        self.tables.get(name).map(|t| t.value().clone())
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.key().clone()).collect()
    }
}

impl AdbMysqlTable {
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: DashMap::new(),
        }
    }

    pub fn insert(&self, row_id: u64, values: Vec<String>) {
        self.rows.insert(row_id, values);
    }

    pub fn select_all(&self) -> Vec<Vec<String>> {
        self.rows.iter().map(|r| r.value().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.rows.len()
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }
}
