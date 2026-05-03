use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

use crate::database::Database;
use crate::table::Table;
use common::{DrorisError, Result};

pub struct CatalogManager {
    databases: RwLock<HashMap<String, Database>>,
    next_id: AtomicU64,
    catalog_path: String,
}

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
        let mut dbs = HashMap::new();
        dbs.insert("information_schema".into(), Database::new(0, "information_schema"));

        Self {
            databases: RwLock::new(dbs),
            next_id: AtomicU64::new(1),
            catalog_path: path.into(),
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

    /// Serialize catalog state to JSON file
    pub fn save(&self) -> common::Result<()> {
        use tokio::fs;
        use tokio::io::AsyncWriteExt;

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let catalog_state = CatalogState {
                databases: self.databases.read().clone(),
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
            *self.databases.write() = state.databases;
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
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data) {
                        if let CatalogOp::CreateDatabase(name) = op {
                            self.create_database(&name)?;
                        }
                    }
                }
                OpType::DropDatabase => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data) {
                        if let CatalogOp::DropDatabase(name) = op {
                            self.drop_database(&name)?;
                        }
                    }
                }
                OpType::CreateTable => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data) {
                        if let CatalogOp::CreateTable { db, table } = op {
                            self.create_table(&db, table)?;
                        }
                    }
                }
                OpType::DropTable => {
                    if let Ok(op) = serde_json::from_slice::<CatalogOp>(&entry.data) {
                        if let CatalogOp::DropTable { db, table } = op {
                            self.drop_table(&db, &table)?;
                        }
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
}