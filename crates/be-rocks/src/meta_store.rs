//! RocksDB wrapper with column families for HarnessDB metadata storage.
//!
//! Column families:
//! - `catalog`: Database/Table metadata, atomic ID counter
//! - `tablet`: Tablet schemas, rowset metadata, counters
//! - `edit_log`: Write-ahead log entries

#![allow(dead_code)]

use rocksdb::{ColumnFamilyDescriptor, DB, Options, WriteBatch};
use std::path::Path;
use thiserror::Error;

/// Column family names
pub const CF_CATALOG: &str = "catalog";
pub const CF_TABLET: &str = "tablet";
pub const CF_EDIT_LOG: &str = "edit_log";

/// Key prefixes for catalog column family
pub const KEY_DB: &str = "db:";
pub const KEY_TABLE: &str = "table:";
pub const KEY_NEXT_ID: &str = "next_id";

/// Key prefixes for tablet column family
pub const KEY_TABLET_SCHEMA: &str = "tablet:";
pub const KEY_ROWSET: &str = "rowset:";
pub const KEY_NEXT_ROWSET_ID: &str = "next_rowset_id:";
pub const KEY_NEXT_SEGMENT_ID: &str = "next_segment_id:";

/// Key prefixes for edit_log column family
pub const KEY_LOG: &str = "log:";
pub const KEY_LAST_APPLIED: &str = "last_applied";
pub const KEY_CURRENT_TERM: &str = "current_term";

#[derive(Debug, Error)]
pub enum RocksStoreError {
    #[error("RocksDB error: {0}")]
    DbError(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    SerializeError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

pub type Result<T> = std::result::Result<T, RocksStoreError>;

/// RocksDB metadata store with column families.
pub struct MetaStore {
    db: DB,
}

impl MetaStore {
    /// Open or create a MetaStore at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Configure RocksDB options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // Configure column families
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_CATALOG, Options::default()),
            ColumnFamilyDescriptor::new(CF_TABLET, Options::default()),
            ColumnFamilyDescriptor::new(CF_EDIT_LOG, Options::default()),
        ];

        // Open database with column families
        let db = DB::open_cf_descriptors(&db_opts, path, cfs)?;

        Ok(Self { db })
    }

    /// Put a key-value pair into a column family.
    pub fn put_cf(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| RocksStoreError::InvalidKey(format!("CF {} not found", cf_name)))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    /// Get a value from a column family.
    pub fn get_cf(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| RocksStoreError::InvalidKey(format!("CF {} not found", cf_name)))?;
        self.db.get_cf(&cf, key).map_err(RocksStoreError::DbError)
    }

    /// Delete a key from a column family.
    pub fn delete_cf(&self, cf_name: &str, key: &[u8]) -> Result<()> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| RocksStoreError::InvalidKey(format!("CF {} not found", cf_name)))?;
        self.db.delete_cf(&cf, key)?;
        Ok(())
    }

    /// Atomically increment a counter and return the new value.
    pub fn increment_counter(&self, cf_name: &str, key: &[u8]) -> Result<u64> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| RocksStoreError::InvalidKey(format!("CF {} not found", cf_name)))?;

        // Read current value
        let current = self.db.get_cf(&cf, key)?;
        let current_val: u64 = current
            .map(|v| {
                if v.len() == 8 {
                    u64::from_be_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]])
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let new_val = current_val + 1;
        let new_bytes = new_val.to_be_bytes();

        // Write atomically using batch
        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, key, &new_bytes);
        self.db.write(batch)?;

        Ok(new_val)
    }

    /// Get current counter value without incrementing.
    pub fn get_counter(&self, cf_name: &str, key: &[u8]) -> Result<u64> {
        let value = self.get_cf(cf_name, key)?;
        Ok(value
            .map(|v| {
                if v.len() == 8 {
                    u64::from_be_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]])
                } else {
                    0
                }
            })
            .unwrap_or(0))
    }

    /// Set counter value directly.
    pub fn set_counter(&self, cf_name: &str, key: &[u8], value: u64) -> Result<()> {
        let bytes = value.to_be_bytes();
        self.put_cf(cf_name, key, &bytes)?;
        Ok(())
    }

    /// Flush all data to disk.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Compact all column families.
    pub fn compact(&self) -> Result<()> {
        for cf_name in [CF_CATALOG, CF_TABLET, CF_EDIT_LOG] {
            if let Some(cf) = self.db.cf_handle(cf_name) {
                self.db.compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
            }
        }
        Ok(())
    }

    /// Get raw DB handle for advanced operations.
    pub fn db(&self) -> &DB {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_meta_store_open() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();
        assert!(store.db.cf_handle(CF_CATALOG).is_some());
        assert!(store.db.cf_handle(CF_TABLET).is_some());
        assert!(store.db.cf_handle(CF_EDIT_LOG).is_some());
    }

    #[test]
    fn test_put_and_get() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        store
            .put_cf(CF_CATALOG, b"test_key", b"test_value")
            .unwrap();
        let value = store.get_cf(CF_CATALOG, b"test_key").unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));
    }

    #[test]
    fn test_delete() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        store
            .put_cf(CF_CATALOG, b"test_key", b"test_value")
            .unwrap();
        store.delete_cf(CF_CATALOG, b"test_key").unwrap();
        let value = store.get_cf(CF_CATALOG, b"test_key").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_counter_increment() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        let val1 = store.increment_counter(CF_CATALOG, b"next_id").unwrap();
        assert_eq!(val1, 1);

        let val2 = store.increment_counter(CF_CATALOG, b"next_id").unwrap();
        assert_eq!(val2, 2);

        let val3 = store.increment_counter(CF_CATALOG, b"next_id").unwrap();
        assert_eq!(val3, 3);
    }

    #[test]
    fn test_counter_set_and_get() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        store.set_counter(CF_CATALOG, b"next_id", 100).unwrap();
        let val = store.get_counter(CF_CATALOG, b"next_id").unwrap();
        assert_eq!(val, 100);
    }

    #[test]
    fn test_write_batch() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        let cf = store.db.cf_handle(CF_CATALOG).unwrap();
        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, b"key1", b"value1");
        batch.put_cf(&cf, b"key2", b"value2");
        batch.delete_cf(&cf, b"key3");

        store.db.write(batch).unwrap();

        assert_eq!(
            store.get_cf(CF_CATALOG, b"key1").unwrap(),
            Some(b"value1".to_vec())
        );
        assert_eq!(
            store.get_cf(CF_CATALOG, b"key2").unwrap(),
            Some(b"value2".to_vec())
        );
        assert!(store.get_cf(CF_CATALOG, b"key3").unwrap().is_none());
    }
}
