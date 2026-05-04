use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

use crate::database::Database;
use crate::table::Table;
use common::{DrorisError, Result, CatalogError};

pub struct CatalogManager {
    databases: DashMap<String, Database>,
    next_id: AtomicU64,
    catalog_path: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
enum CatalogOp {
    CreateDatabase(String),
    DropDatabase(String),
    CreateTable { db: String, table: Table },
    DropTable { db: String, table: String },
    AlterDatabase { name: String },
    AlterTable { db: String, table: String },
    UpdateStats { db: String, table: String },
}

pub struct CatalogWriter {
    catalog: Arc<AsyncRwLock<CatalogManager>>,
    edit_log: Arc<AsyncRwLock<fe_common::edit_log::EditLog>>,
}

impl CatalogManager {
    pub fn new() -> Self {
        Self::with_path("data/fe/doris-meta")
    }

    pub fn with_path(path: impl Into<String>) -> Self {
        let dbs = DashMap::new();
        dbs.insert("information_schema".into(), Database::new(0, "information_schema"));

        Self {
            databases: dbs,
            next_id: AtomicU64::new(1),
            catalog_path: path.into(),
        }
    }

    pub fn create_database(&self, name: &str) -> Result<()> {
        if self.databases.contains_key(name) {
            return Err(DrorisError::catalog(CatalogError::DatabaseAlreadyExists, format!("database '{}' already exists", name)));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.databases.insert(name.to_string(), Database::new(id, name));
        Ok(())
    }

    pub fn drop_database(&self, name: &str) -> Result<()> {
        self.databases.remove(name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::DatabaseNotFound, format!("database '{}' not found", name)))?;
        Ok(())
    }

    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter().map(|r| r.key().clone()).collect()
    }

    pub fn get_database(&self, name: &str) -> Option<Database> {
        self.databases.get(name).map(|r| r.value().clone())
    }

    pub fn create_table(&self, db_name: &str, table: Table) -> Result<()> {
        let mut db_ref = self.databases.get_mut(db_name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::DatabaseNotFound, format!("database '{}' not found", db_name)))?;
        db_ref.add_table(table);
        Ok(())
    }

    pub fn drop_table(&self, db_name: &str, table_name: &str) -> Result<()> {
        let mut db_ref = self.databases.get_mut(db_name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::DatabaseNotFound, format!("database '{}' not found", db_name)))?;
        db_ref.drop_table(table_name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::TableNotFound, format!("table '{}' not found", table_name)))?;
        Ok(())
    }

    pub fn get_table(&self, db_name: &str, table_name: &str) -> Option<Table> {
        self.databases.get(db_name)
            .and_then(|db| db.get_table(table_name).cloned())
    }

    pub fn list_tables(&self, db_name: &str) -> Option<Vec<String>> {
        self.databases.get(db_name)
            .map(|db| db.table_names().into_iter().map(|s| s.to_string()).collect())
    }

    /// Update statistics for a table.
    pub fn update_table_stats(
        &self,
        db_name: &str,
        table_name: &str,
        stats: crate::stats::TableStats,
    ) -> Result<()> {
        let mut db_ref = self.databases.get_mut(db_name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::DatabaseNotFound,
                format!("database '{}' not found", db_name)))?;
        let table = db_ref.get_table_mut(table_name)
            .ok_or_else(|| DrorisError::catalog(CatalogError::TableNotFound,
                format!("table '{}' not found", table_name)))?;
        table.stats = Some(stats);
        Ok(())
    }

    /// Get statistics for a table.
    pub fn get_table_stats(&self, db_name: &str, table_name: &str) -> Option<crate::stats::TableStats> {
        self.get_table(db_name, table_name).and_then(|t| t.stats.clone())
    }

    /// Get all tables with their stats for a given database.
    pub fn get_all_table_stats(&self, db_name: &str) -> Vec<(String, Option<crate::stats::TableStats>)> {
        self.databases.get(db_name)
            .map(|db| {
                db.tables.iter()
                    .map(|(name, table)| (name.clone(), table.stats.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Serialize catalog state to JSON file
    pub fn save(&self) -> common::Result<()> {
        use tokio::fs;
        use tokio::io::AsyncWriteExt;

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let catalog_state = CatalogState {
                databases: self.databases.iter().map(|r| (r.key().clone(), r.value().clone())).collect(),
                next_id: self.next_id.load(Ordering::Relaxed),
            };
            let json = serde_json::to_string(&catalog_state)
                .map_err(|e| DrorisError::Internal(e.to_string()))?;
            let path = format!("{}/catalog.json", self.catalog_path);
            fs::create_dir_all(&self.catalog_path).await?;
            let mut file = fs::File::create(&path).await?;
            file.write_all(json.as_bytes()).await?;
            Ok(())
        })
    }

    /// Load catalog state from JSON file
    pub fn load(&mut self) -> common::Result<()> {
        use tokio::fs;
        use tokio::io::AsyncReadExt;

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let path = format!("{}/catalog.json", self.catalog_path);
            if !std::path::Path::new(&path).exists() {
                return Ok(());
            }
            let mut file = fs::File::open(&path).await?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            let state: CatalogState = serde_json::from_str(&contents)
                .map_err(|e| DrorisError::Internal(e.to_string()))?;
            for (key, value) in state.databases {
                self.databases.insert(key, value);
            }
            self.next_id = AtomicU64::new(state.next_id);
            Ok(())
        })
    }

    /// Replay edit log entries into the catalog
    pub fn replay_edit_log(&mut self, log: &fe_common::edit_log::EditLog) -> common::Result<()> {
        use fe_common::edit_log::OpType;

        for entry in log.entries() {
            match entry.op_type {
                OpType::CreateDatabase => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data)
                        && let CatalogOp::CreateDatabase(name) = op {
                            self.create_database(&name)?;
                        }
                }
                OpType::DropDatabase => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data)
                        && let CatalogOp::DropDatabase(name) = op {
                            self.drop_database(&name)?;
                        }
                }
                OpType::CreateTable => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data)
                        && let CatalogOp::CreateTable { db, table } = op {
                            self.create_table(&db, table)?;
                        }
                }
                OpType::DropTable => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data)
                        && let CatalogOp::DropTable { db, table } = op {
                            self.drop_table(&db, &table)?;
                        }
                }
                OpType::UpdateStats => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data)
                        && let CatalogOp::UpdateStats { db: _, table: _ } = op {
                            // Stats are stored directly on the Table struct, already persisted via catalog save.
                        }
                }
                _ => { /* ignore other op types for now */ }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogState {
    databases: HashMap<String, Database>,
    next_id: u64,
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogWriter {
    pub fn new(catalog: Arc<AsyncRwLock<CatalogManager>>, edit_log: Arc<AsyncRwLock<fe_common::edit_log::EditLog>>) -> Self {
        Self { catalog, edit_log }
    }

    pub async fn create_database(&self, name: &str) -> common::Result<()> {
        let op = CatalogOp::CreateDatabase(name.to_string());
        let data = serde_json::to_vec(&op)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let _index = self.edit_log.write().await.append(fe_common::edit_log::OpType::CreateDatabase, data);
        self.catalog.write().await.create_database(name)?;
        Ok(())
    }

    pub async fn drop_database(&self, name: &str) -> common::Result<()> {
        let op = CatalogOp::DropDatabase(name.to_string());
        let data = serde_json::to_vec(&op)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let _index = self.edit_log.write().await.append(fe_common::edit_log::OpType::DropDatabase, data);
        self.catalog.write().await.drop_database(name)?;
        Ok(())
    }

    pub async fn create_table(&self, db_name: &str, table: Table) -> common::Result<()> {
        let op = CatalogOp::CreateTable { db: db_name.to_string(), table: table.clone() };
        let data = serde_json::to_vec(&op)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let _index = self.edit_log.write().await.append(fe_common::edit_log::OpType::CreateTable, data);
        self.catalog.write().await.create_table(db_name, table)?;
        Ok(())
    }

    pub async fn drop_table(&self, db_name: &str, table_name: &str) -> common::Result<()> {
        let op = CatalogOp::DropTable { db: db_name.to_string(), table: table_name.to_string() };
        let data = serde_json::to_vec(&op)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let _index = self.edit_log.write().await.append(fe_common::edit_log::OpType::DropTable, data);
        self.catalog.write().await.drop_table(db_name, table_name)?;
        Ok(())
    }

    pub async fn update_table_stats(
        &self,
        db_name: &str,
        table_name: &str,
        stats: crate::stats::TableStats,
    ) -> common::Result<()> {
        let op = CatalogOp::UpdateStats { db: db_name.to_string(), table: table_name.to_string() };
        let data = serde_json::to_vec(&op)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let _index = self.edit_log.write().await.append(fe_common::edit_log::OpType::UpdateStats, data);
        self.catalog.write().await.update_table_stats(db_name, table_name, stats)?;
        Ok(())
    }
}

/// Statistics provider backed by the catalog's persisted stats.
pub struct CatalogStatsProvider {
    catalog: std::sync::Arc<CatalogManager>,
}

impl CatalogStatsProvider {
    pub fn new(catalog: std::sync::Arc<CatalogManager>) -> Self {
        Self { catalog }
    }
}

impl crate::stats::StatisticsProvider for CatalogStatsProvider {
    fn get_table_stats(&self, database: &str, table: &str) -> Option<crate::stats::TableStats> {
        self.catalog.get_table_stats(database, table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{Table, TableColumn, KeysType};
    use types::DataType;

    fn make_table(id: u64, name: &str) -> Table {
        Table {
            id,
            name: name.to_string(),
            database: "testdb".to_string(),
            columns: vec![
                TableColumn {
                    name: "id".into(),
                    data_type: DataType::Int64,
                    nullable: false,
                    default_value: None,
                    agg_type: None,
                    comment: String::new(),
                },
            ],
            keys_type: KeysType::Duplicate,
            partition_info: None,
            distribution_info: None,
            replication_num: 1,
            properties: HashMap::new(),
            row_count: 0,
            data_size: 0,
            stats: None,
        }
    }

    #[test]
    fn test_create_database() {
        let mgr = CatalogManager::new();
        assert!(mgr.create_database("db1").is_ok());
        assert!(mgr.list_databases().contains(&"db1".to_string()));
    }

    #[test]
    fn test_create_database_duplicate() {
        let mgr = CatalogManager::new();
        mgr.create_database("db1").unwrap();
        let result = mgr.create_database("db1");
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_database() {
        let mgr = CatalogManager::new();
        mgr.create_database("db1").unwrap();
        assert!(mgr.drop_database("db1").is_ok());
        assert!(!mgr.list_databases().contains(&"db1".to_string()));
    }

    #[test]
    fn test_drop_database_nonexistent() {
        let mgr = CatalogManager::new();
        assert!(mgr.drop_database("no_such_db").is_err());
    }

    #[test]
    fn test_create_table() {
        let mgr = CatalogManager::new();
        mgr.create_database("mydb").unwrap();
        let table = make_table(1, "users");
        assert!(mgr.create_table("mydb", table).is_ok());
        let t = mgr.get_table("mydb", "users");
        assert!(t.is_some());
        assert_eq!(t.unwrap().name, "users");
    }

    #[test]
    fn test_create_table_wrong_db() {
        let mgr = CatalogManager::new();
        let table = make_table(1, "users");
        assert!(mgr.create_table("no_db", table).is_err());
    }

    #[test]
    fn test_drop_table() {
        let mgr = CatalogManager::new();
        mgr.create_database("mydb").unwrap();
        let table = make_table(1, "users");
        mgr.create_table("mydb", table).unwrap();
        assert!(mgr.drop_table("mydb", "users").is_ok());
        assert!(mgr.get_table("mydb", "users").is_none());
    }

    #[test]
    fn test_list_tables() {
        let mgr = CatalogManager::new();
        mgr.create_database("mydb").unwrap();
        mgr.create_table("mydb", make_table(1, "t1")).unwrap();
        mgr.create_table("mydb", make_table(2, "t2")).unwrap();
        let tables = mgr.list_tables("mydb").unwrap();
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"t1".to_string()));
        assert!(tables.contains(&"t2".to_string()));
    }

    #[test]
    fn test_information_schema_exists() {
        let mgr = CatalogManager::new();
        assert!(mgr.list_databases().contains(&"information_schema".to_string()));
    }

    #[test]
    fn test_catalog_save_and_load() {
        let dir = format!("/tmp/rovisdb_test_catalog_{}", std::process::id());
        let mgr = CatalogManager::with_path(&dir);
        mgr.create_database("saved_db").unwrap();
        mgr.create_table("saved_db", make_table(1, "saved_table")).unwrap();
        mgr.save().unwrap();

        let mut mgr2 = CatalogManager::with_path(&dir);
        mgr2.load().unwrap();
        assert!(mgr2.get_database("saved_db").is_some());
        assert!(mgr2.get_table("saved_db", "saved_table").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }
}