use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::database::Database;
use crate::table::Table;
use common::{DrorisError, Result};

pub struct CatalogManager {
    databases: RwLock<HashMap<String, Database>>,
    next_id: AtomicU64,
}

impl CatalogManager {
    pub fn new() -> Self {
        let mut dbs = HashMap::new();
        dbs.insert("information_schema".into(), Database::new(0, "information_schema"));

        Self {
            databases: RwLock::new(dbs),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn create_database(&self, name: &str) -> Result<()> {
        let mut dbs = self.databases.write();
        if dbs.contains_key(name) {
            return Err(DrorisError::Catalog(format!("database '{}' already exists", name)));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        dbs.insert(name.to_string(), Database::new(id, name));
        Ok(())
    }

    pub fn drop_database(&self, name: &str) -> Result<()> {
        let mut dbs = self.databases.write();
        dbs.remove(name)
            .ok_or_else(|| DrorisError::Catalog(format!("database '{}' not found", name)))?;
        Ok(())
    }

    pub fn list_databases(&self) -> Vec<String> {
        self.databases.read().keys().cloned().collect()
    }

    pub fn get_database(&self, name: &str) -> Option<Database> {
        self.databases.read().get(name).cloned()
    }

    pub fn create_table(&self, db_name: &str, table: Table) -> Result<()> {
        let mut dbs = self.databases.write();
        let db = dbs.get_mut(db_name)
            .ok_or_else(|| DrorisError::Catalog(format!("database '{}' not found", db_name)))?;
        db.add_table(table);
        Ok(())
    }

    pub fn drop_table(&self, db_name: &str, table_name: &str) -> Result<()> {
        let mut dbs = self.databases.write();
        let db = dbs.get_mut(db_name)
            .ok_or_else(|| DrorisError::Catalog(format!("database '{}' not found", db_name)))?;
        db.drop_table(table_name)
            .ok_or_else(|| DrorisError::Catalog(format!("table '{}' not found", table_name)))?;
        Ok(())
    }

    pub fn get_table(&self, db_name: &str, table_name: &str) -> Option<Table> {
        self.databases.read()
            .get(db_name)
            .and_then(|db| db.get_table(table_name).cloned())
    }

    pub fn list_tables(&self, db_name: &str) -> Option<Vec<String>> {
        self.databases.read()
            .get(db_name)
            .map(|db| db.table_names().into_iter().map(|s| s.to_string()).collect())
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}
