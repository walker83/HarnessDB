use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

use crate::database::Database;
use crate::materialized_view::MaterializedView;
use crate::table::Table;
use common::{DrorisError, Result, CatalogError};

pub struct CatalogManager {
    databases: DashMap<String, Database>,
    materialized_views: DashMap<String, MaterializedView>,
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
            materialized_views: DashMap::new(),
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

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
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

    pub fn create_materialized_view(&self, mv: MaterializedView) -> common::Result<()> {
        let key = format!("{}.{}", mv.database, mv.name);
        self.materialized_views.insert(key, mv);
        Ok(())
    }

    pub fn drop_materialized_view(&self, db_name: &str, name: &str) -> common::Result<()> {
        let key = format!("{}.{}", db_name, name);
        self.materialized_views.remove(&key)
            .ok_or_else(|| DrorisError::catalog(CatalogError::TableNotFound, format!("materialized view '{}' not found", name)))?;
        Ok(())
    }

    pub fn get_materialized_view(&self, db_name: &str, name: &str) -> Option<MaterializedView> {
        let key = format!("{}.{}", db_name, name);
        self.materialized_views.get(&key).map(|r| r.value().clone())
    }

    pub fn list_materialized_views(&self, db_name: &str) -> Vec<MaterializedView> {
        let prefix = format!("{}.", db_name);
        self.materialized_views.iter()
            .filter(|r| r.key().starts_with(&prefix))
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn all_materialized_views(&self) -> Vec<MaterializedView> {
        self.materialized_views.iter().map(|r| r.value().clone()).collect()
    }

    /// Serialize catalog state to JSON file
    pub fn save(&self) -> common::Result<()> {
        use std::fs;

        let catalog_state = CatalogState {
            databases: self.databases.iter().map(|r| (r.key().clone(), r.value().clone())).collect(),
            materialized_views: self.materialized_views.iter().map(|r| (r.key().clone(), r.value().clone())).collect(),
            next_id: self.next_id.load(Ordering::Relaxed),
        };
        let json = serde_json::to_string(&catalog_state)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        let path = format!("{}/catalog.json", self.catalog_path);
        fs::create_dir_all(&self.catalog_path)?;
        fs::write(&path, json.as_bytes())?;
        Ok(())
    }

    /// Load catalog state from JSON file
    pub fn load(&self) -> common::Result<()> {
        use std::fs;

        let path = format!("{}/catalog.json", self.catalog_path);
        if !std::path::Path::new(&path).exists() {
            return Ok(());
        }
        let contents = fs::read_to_string(&path)?;
        let state: CatalogState = serde_json::from_str(&contents)
            .map_err(|e| DrorisError::Internal(e.to_string()))?;
        for (key, value) in state.databases {
            self.databases.insert(key, value);
        }
        for (key, value) in state.materialized_views {
            self.materialized_views.insert(key, value);
        }
        self.next_id.store(state.next_id, Ordering::SeqCst);
        Ok(())
    }

    /// Replay edit log entries into the catalog
    pub fn replay_edit_log(&self, log: &fe_common::edit_log::EditLog) -> common::Result<()> {
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
                _ => { /* ignore other op types for now */ }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CatalogState {
    databases: HashMap<String, Database>,
    materialized_views: HashMap<String, MaterializedView>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{Table, TableColumn, KeysType};
    use types::DataType;

    fn make_table(id: u64, name: &str) -> Table {
        Table {
            id,
            tablet_id: 0, // TODO: 创建table时分配真实的tablet_id
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
            unique_keys: vec![],
            partition_info: None,
            distribution_info: None,
            replication_num: 1,
            properties: HashMap::new(),
            row_count: 0,
            data_size: 0,
            stats: None,
            view_definition: None,
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