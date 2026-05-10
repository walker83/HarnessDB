//! Catalog store for database and table metadata.
//!
//! Key schema:
//! - `db:{name}` → Database JSON
//! - `db:{name}:table:{tbl}` → Table JSON
//! - `next_id` → Atomic ID counter (u64)

use crate::meta_store::{MetaStore, CF_CATALOG, KEY_DB, KEY_TABLE, KEY_NEXT_ID, Result, RocksStoreError};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use tracing::debug;
use std::collections::HashMap;
use std::sync::Arc;

/// Database metadata (mirrors fe-catalog::Database).
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
}

/// Table column definition (mirrors fe-catalog::TableColumn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: types::DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub agg_type: Option<String>,
    pub comment: String,
}

/// Keys type (mirrors fe-catalog::KeysType).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeysType {
    Duplicate,
    Aggregate,
    Unique,
    Primary,
}

/// Table metadata (mirrors fe-catalog::Table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: u64,
    pub tablet_id: u64,
    pub name: String,
    pub database: String,
    pub columns: Vec<TableColumn>,
    pub keys_type: KeysType,
    pub unique_keys: Vec<UniqueKeyDef>,
    pub partition_info: Option<PartitionInfo>,
    pub distribution_info: Option<DistributionInfo>,
    pub replication_num: u32,
    pub properties: HashMap<String, String>,
    pub row_count: u64,
    pub data_size: u64,
    pub stats: Option<TableStats>,
    pub view_definition: Option<String>,
}

impl Default for Table {
    fn default() -> Self {
        Table {
            id: 0,
            tablet_id: 0,
            name: String::new(),
            database: String::new(),
            columns: Vec::new(),
            keys_type: KeysType::Duplicate,
            unique_keys: Vec::new(),
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
}

/// Unique key definition (mirrors fe-catalog::UniqueKeyDef).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueKeyDef {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Partition info (mirrors fe-catalog::PartitionInfo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition_type: String,
    pub columns: Vec<String>,
    pub partitions: Vec<Partition>,
}

/// Partition (mirrors fe-catalog::Partition).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub id: u64,
    pub name: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
}

/// Distribution info (mirrors fe-catalog::DistributionInfo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionInfo {
    pub dist_type: String,
    pub columns: Vec<String>,
    pub buckets: u32,
}

/// Table statistics (mirrors fe-catalog::TableStats).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub row_count: u64,
    pub data_size: u64,
    pub column_stats: HashMap<String, ColumnStats>,
}

/// Column statistics (mirrors fe-catalog::ColumnStats).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub null_count: u64,
    pub distinct_count: u64,
}

/// Catalog store for managing database and table metadata in RocksDB.
pub struct CatalogStore {
    store: Arc<MetaStore>,
}

impl CatalogStore {
    /// Create a new CatalogStore.
    pub fn new(store: Arc<MetaStore>) -> Self {
        Self { store }
    }

    /// Put a database (typed, uses be-rocks Database type).
    pub fn put_database(&self, name: &str, db: &Database) -> Result<()> {
        let key = format!("{}{}", KEY_DB, name);
        let value = serialize(db)?;
        self.store.put_cf(CF_CATALOG, key.as_bytes(), &value)?;
        debug!("Put database: {}", name);
        Ok(())
    }

    /// Put a database as raw bytes (for cross-crate compatibility).
    pub fn put_database_raw(&self, name: &str, data: &[u8]) -> Result<()> {
        let key = format!("{}{}", KEY_DB, name);
        self.store.put_cf(CF_CATALOG, key.as_bytes(), data)?;
        debug!("Put database raw: {}", name);
        Ok(())
    }

    /// Get a database by name (typed).
    pub fn get_database(&self, name: &str) -> Result<Option<Database>> {
        let key = format!("{}{}", KEY_DB, name);
        let value = self.store.get_cf(CF_CATALOG, key.as_bytes())?;
        value.map(|v| deserialize(&v)).transpose()
    }

    /// Get a database as raw bytes (for cross-crate compatibility).
    pub fn get_database_raw(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let key = format!("{}{}", KEY_DB, name);
        self.store.get_cf(CF_CATALOG, key.as_bytes())
    }

    /// Delete a database by name.
    pub fn delete_database(&self, name: &str) -> Result<()> {
        let key = format!("{}{}", KEY_DB, name);
        self.store.delete_cf(CF_CATALOG, key.as_bytes())?;

        // Also delete all tables in this database
        let prefix = format!("{}{}{}", KEY_DB, name, KEY_TABLE);
        self.delete_keys_with_prefix(CF_CATALOG, prefix.as_bytes())?;

        debug!("Deleted database: {}", name);
        Ok(())
    }

    /// List all database names.
    pub fn list_databases(&self) -> Result<Vec<String>> {
        let prefix = KEY_DB.as_bytes();
        let keys = self.list_keys_with_prefix(CF_CATALOG, prefix)?;
        keys.iter()
            .map(|k| {
                String::from_utf8(k[prefix.len()..].to_vec())
                    .map_err(|e| RocksStoreError::InvalidKey(e.to_string()))
            })
            .collect()
    }

    /// Put a table (typed).
    pub fn put_table(&self, db_name: &str, table_name: &str, table: &Table) -> Result<()> {
        let key = format!("{}{}{}{}", KEY_DB, db_name, KEY_TABLE, table_name);
        let value = serialize(table)?;
        self.store.put_cf(CF_CATALOG, key.as_bytes(), &value)?;
        debug!("Put table: {}.{}", db_name, table_name);
        Ok(())
    }

    /// Put a table as raw bytes (for cross-crate compatibility).
    pub fn put_table_raw(&self, db_name: &str, table_name: &str, data: &[u8]) -> Result<()> {
        let key = format!("{}{}{}{}", KEY_DB, db_name, KEY_TABLE, table_name);
        self.store.put_cf(CF_CATALOG, key.as_bytes(), data)?;
        debug!("Put table raw: {}.{}", db_name, table_name);
        Ok(())
    }

    /// Get a table by database and name (typed).
    pub fn get_table(&self, db_name: &str, table_name: &str) -> Result<Option<Table>> {
        let key = format!("{}{}{}{}", KEY_DB, db_name, KEY_TABLE, table_name);
        let value = self.store.get_cf(CF_CATALOG, key.as_bytes())?;
        value.map(|v| deserialize(&v)).transpose()
    }

    /// Get a table as raw bytes (for cross-crate compatibility).
    pub fn get_table_raw(&self, db_name: &str, table_name: &str) -> Result<Option<Vec<u8>>> {
        let key = format!("{}{}{}{}", KEY_DB, db_name, KEY_TABLE, table_name);
        self.store.get_cf(CF_CATALOG, key.as_bytes())
    }

    /// Put arbitrary key-value pair (for materialized views, custom metadata).
    pub fn put_raw(&self, key: &str, data: &[u8]) -> Result<()> {
        self.store.put_cf(CF_CATALOG, key.as_bytes(), data)?;
        debug!("Put raw key: {}", key);
        Ok(())
    }

    /// Get arbitrary key-value pair.
    pub fn get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.store.get_cf(CF_CATALOG, key.as_bytes())
    }

    /// Delete arbitrary key.
    pub fn delete_raw(&self, key: &str) -> Result<()> {
        self.store.delete_cf(CF_CATALOG, key.as_bytes())?;
        debug!("Delete raw key: {}", key);
        Ok(())
    }

    /// List keys with a given prefix (for custom metadata).
    pub fn list_keys_with_prefix_str(&self, prefix: &str) -> Result<Vec<String>> {
        let keys = self.list_keys_with_prefix(CF_CATALOG, prefix.as_bytes())?;
        keys.iter()
            .map(|k| {
                String::from_utf8(k.clone())
                    .map_err(|e| RocksStoreError::InvalidKey(e.to_string()))
            })
            .collect()
    }

    /// Delete a table by database and name.
    pub fn delete_table(&self, db_name: &str, table_name: &str) -> Result<()> {
        let key = format!("{}{}{}{}", KEY_DB, db_name, KEY_TABLE, table_name);
        self.store.delete_cf(CF_CATALOG, key.as_bytes())?;
        debug!("Deleted table: {}.{}", db_name, table_name);
        Ok(())
    }

    /// List all table names in a database.
    pub fn list_tables(&self, db_name: &str) -> Result<Vec<String>> {
        let prefix = format!("{}{}{}", KEY_DB, db_name, KEY_TABLE);
        let keys = self.list_keys_with_prefix(CF_CATALOG, prefix.as_bytes())?;
        keys.iter()
            .map(|k| {
                String::from_utf8(k[prefix.len()..].to_vec())
                    .map_err(|e| RocksStoreError::InvalidKey(e.to_string()))
            })
            .collect()
    }

    /// Get the next unique ID and increment the counter atomically.
    pub fn next_id(&self) -> Result<u64> {
        self.store.increment_counter(CF_CATALOG, KEY_NEXT_ID.as_bytes())
    }

    /// Get current ID counter value.
    pub fn get_next_id(&self) -> Result<u64> {
        self.store.get_counter(CF_CATALOG, KEY_NEXT_ID.as_bytes())
    }

    /// Set the ID counter value (for migration/recovery).
    pub fn set_next_id(&self, value: u64) -> Result<()> {
        self.store.set_counter(CF_CATALOG, KEY_NEXT_ID.as_bytes(), value)
    }

    /// List all tables across all databases.
    pub fn list_all_tables(&self) -> Result<Vec<(String, Table)>> {
        let db_names = self.list_databases()?;
        let mut tables = Vec::new();
        for db_name in db_names {
            let table_names = self.list_tables(&db_name)?;
            for table_name in table_names {
                if let Some(table) = self.get_table(&db_name, &table_name)? {
                    tables.push((format!("{}.{}", db_name, table_name), table));
                }
            }
        }
        Ok(tables)
    }

    // Internal helpers

    fn list_keys_with_prefix(&self, cf_name: &str, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let cf = self.store.db().cf_handle(cf_name)
            .ok_or_else(|| RocksStoreError::InvalidKey(format!("CF {} not found", cf_name)))?;

        let mut iter = self.store.db().raw_iterator_cf(&cf);
        iter.seek(prefix);

        let mut keys = Vec::new();
        while iter.valid() {
            let key = iter.key().unwrap_or_default();
            if key.starts_with(prefix) {
                keys.push(key.to_vec());
                iter.next();
            } else {
                break;
            }
        }
        Ok(keys)
    }

    fn delete_keys_with_prefix(&self, cf_name: &str, prefix: &[u8]) -> Result<()> {
        let keys = self.list_keys_with_prefix(cf_name, prefix)?;
        for key in keys {
            self.store.delete_cf(cf_name, &key)?;
        }
        Ok(())
    }

    /// Flush to disk.
    pub fn flush(&self) -> Result<()> {
        self.store.flush()
    }
}

fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value)
        .map_err(|e| RocksStoreError::SerializeError(e.to_string()))
}

fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|e| RocksStoreError::SerializeError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use types::DataType;

    fn make_test_database(name: &str, id: u64) -> Database {
        Database::new(id, name)
    }

    fn make_test_table(name: &str, id: u64) -> Table {
        Table {
            id,
            tablet_id: id + 1000,
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
    fn test_put_and_get_database() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        let db = make_test_database("mydb", 1);
        store.put_database("mydb", &db).unwrap();

        let retrieved = store.get_database("mydb").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "mydb");
        assert_eq!(retrieved.id, 1);
    }

    #[test]
    fn test_delete_database() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        let db = make_test_database("mydb", 1);
        store.put_database("mydb", &db).unwrap();
        store.delete_database("mydb").unwrap();

        let retrieved = store.get_database("mydb").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_databases() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_database("db1", &make_test_database("db1", 1)).unwrap();
        store.put_database("db2", &make_test_database("db2", 2)).unwrap();
        store.put_database("db3", &make_test_database("db3", 3)).unwrap();

        let dbs = store.list_databases().unwrap();
        assert_eq!(dbs.len(), 3);
        assert!(dbs.contains(&"db1".to_string()));
        assert!(dbs.contains(&"db2".to_string()));
        assert!(dbs.contains(&"db3".to_string()));
    }

    #[test]
    fn test_put_and_get_table() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_database("mydb", &make_test_database("mydb", 1)).unwrap();
        let table = make_test_table("users", 2);
        store.put_table("mydb", "users", &table).unwrap();

        let retrieved = store.get_table("mydb", "users").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "users");
        assert_eq!(retrieved.id, 2);
    }

    #[test]
    fn test_delete_table() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_database("mydb", &make_test_database("mydb", 1)).unwrap();
        let table = make_test_table("users", 2);
        store.put_table("mydb", "users", &table).unwrap();
        store.delete_table("mydb", "users").unwrap();

        let retrieved = store.get_table("mydb", "users").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_tables() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_database("mydb", &make_test_database("mydb", 1)).unwrap();
        store.put_table("mydb", "t1", &make_test_table("t1", 2)).unwrap();
        store.put_table("mydb", "t2", &make_test_table("t2", 3)).unwrap();
        store.put_table("mydb", "t3", &make_test_table("t3", 4)).unwrap();

        let tables = store.list_tables("mydb").unwrap();
        assert_eq!(tables.len(), 3);
        assert!(tables.contains(&"t1".to_string()));
        assert!(tables.contains(&"t2".to_string()));
        assert!(tables.contains(&"t3".to_string()));
    }

    #[test]
    fn test_next_id() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        let id1 = store.next_id().unwrap();
        let id2 = store.next_id().unwrap();
        let id3 = store.next_id().unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_set_next_id() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.set_next_id(100).unwrap();
        let val = store.get_next_id().unwrap();
        assert_eq!(val, 100);

        let next = store.next_id().unwrap();
        assert_eq!(next, 101);
    }

    #[test]
    fn test_delete_database_cascades_tables() {
        let dir = tempdir().unwrap();
        let store = CatalogStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_database("mydb", &make_test_database("mydb", 1)).unwrap();
        store.put_table("mydb", "t1", &make_test_table("t1", 2)).unwrap();
        store.put_table("mydb", "t2", &make_test_table("t2", 3)).unwrap();

        store.delete_database("mydb").unwrap();

        // Tables should also be deleted
        assert!(store.list_tables("mydb").unwrap().is_empty());
    }
}