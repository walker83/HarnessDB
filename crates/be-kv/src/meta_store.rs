//! redb-backed metadata store for HarnessDB.
//!
//! Replaces the previous RocksDB backend with a pure-Rust embedded KV store
//! (redb, an LMDB-inspired ACID B+tree). The public API is intentionally kept
//! identical to the old RocksDB wrapper so that `CatalogStore` and downstream
//! callers (fe-catalog) need no changes.
//!
//! Three logical "column families" map to three redb tables, all using
//! `&[u8]` -> `&[u8]`:
//! - `catalog`: Database/Table metadata, atomic ID counter
//! - `tablet`: Tablet schemas, rowset metadata, counters
//! - `edit_log`: Write-ahead log entries
//!
//! Every operation is wrapped in its own transaction (redb has no
//! non-transactional API). The read-modify-write in `increment_counter` now
//! happens inside a single write transaction, which actually makes it atomic
//! (the old RocksDB version used a one-entry WriteBatch and was not).

#![allow(dead_code)]

use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use thiserror::Error;

/// redb tables that replace the RocksDB column families.
/// Key/value types are `&'static [u8]` (caller-owned bytes borrowed for the
/// duration of a transaction).
const T_CATALOG: TableDefinition<&[u8], &[u8]> = TableDefinition::new("catalog");
const T_TABLET: TableDefinition<&[u8], &[u8]> = TableDefinition::new("tablet");
const T_EDIT_LOG: TableDefinition<&[u8], &[u8]> = TableDefinition::new("edit_log");

/// Column family names (kept as string constants for API compatibility).
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
    #[error("KV store error: {0}")]
    DbError(#[from] redb::Error),
    #[error("KV store database error: {0}")]
    DatabaseError(#[from] redb::DatabaseError),
    #[error("KV store transaction error: {0}")]
    TransactionError(#[from] redb::TransactionError),
    #[error("KV storage error: {0}")]
    StorageError(#[from] redb::StorageError),
    #[error("Commit error: {0}")]
    CommitError(#[from] redb::CommitError),
    #[error("Table error: {0}")]
    TableError(#[from] redb::TableError),
    #[error("Serialization error: {0}")]
    SerializeError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

// Name kept for source compatibility; the error type itself now wraps redb.
pub type Result<T> = std::result::Result<T, RocksStoreError>;

/// Metadata store backed by redb.
///
/// The struct name `MetaStore` and all public method signatures are preserved
/// from the RocksDB version so `CatalogStore` and fe-catalog compile unchanged.
pub struct MetaStore {
    db: Database,
}

impl MetaStore {
    /// Open or create a MetaStore at the given path.
    ///
    /// Unlike RocksDB (which uses a whole directory), redb stores everything
    /// in a single file. To preserve the old "pass a directory" API, if the
    /// given path is (or will be) a directory we store the database as
    /// `<path>/meta.redb` inside it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let db_path = resolve_db_path(path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = Database::create(&db_path)?;

        // Ensure all three tables exist (redb creates a table on first
        // `open_table` inside a write transaction).
        let txn = db.begin_write()?;
        {
            let _ = txn.open_table(T_CATALOG)?;
            let _ = txn.open_table(T_TABLET)?;
            let _ = txn.open_table(T_EDIT_LOG)?;
        }
        txn.commit()?;

        Ok(Self { db })
    }

    /// Put a key-value pair into a column family.
    pub fn put_cf(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def(cf_name)?)?;
            table.insert(key, value)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Get a value from a column family.
    pub fn get_cf(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def(cf_name)?)?;
        Ok(table.get(key)?.map(|v| v.value().to_vec()))
    }

    /// Delete a key from a column family.
    pub fn delete_cf(&self, cf_name: &str, key: &[u8]) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def(cf_name)?)?;
            // redb's remove returns Ok(Some(old_value)) if present, Ok(None) otherwise;
            // presence is irrelevant to callers.
            let _ = table.remove(key)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Atomically increment a counter and return the new value.
    ///
    /// Unlike the old RocksDB implementation (which read, then wrote inside a
    /// one-item WriteBatch), the read-modify-write here is fully atomic: it
    /// happens entirely within a single write transaction.
    pub fn increment_counter(&self, cf_name: &str, key: &[u8]) -> Result<u64> {
        let txn = self.db.begin_write()?;
        let new_val = {
            let mut table = txn.open_table(table_def(cf_name)?)?;
            let current_val: u64 = table
                .get(key)?
                .map(|v| decode_u64(v.value()))
                .unwrap_or(0);
            let new_val = current_val + 1;
            table.insert(key, new_val.to_be_bytes().as_slice())?;
            new_val
        };
        txn.commit()?;
        Ok(new_val)
    }

    /// Get current counter value without incrementing.
    pub fn get_counter(&self, cf_name: &str, key: &[u8]) -> Result<u64> {
        let value = self.get_cf(cf_name, key)?;
        Ok(value.as_deref().map(decode_u64).unwrap_or(0))
    }

    /// Set counter value directly.
    pub fn set_counter(&self, cf_name: &str, key: &[u8], value: u64) -> Result<()> {
        let bytes = value.to_be_bytes();
        self.put_cf(cf_name, key, &bytes)?;
        Ok(())
    }

    /// Iterate all key/value pairs whose key starts with `prefix`.
    ///
    /// Returns `(key, value)` pairs. Used by `CatalogStore` prefix-list helpers.
    pub fn iter_prefix(&self, cf_name: &str, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def(cf_name)?)?;
        // redb range over &[u8] keys: [prefix..) then filter by starts_with.
        // Keys are lexicographically ordered so all matches are contiguous.
        let mut out = Vec::new();
        let range = table.range::<&[u8]>(prefix..)?;
        for item in range {
            let (k, v) = item?;
            let key_bytes = k.value();
            if key_bytes.starts_with(prefix) {
                out.push((key_bytes.to_vec(), v.value().to_vec()));
            } else {
                // Keys are sorted; once we leave the prefix we're done.
                break;
            }
        }
        Ok(out)
    }

    /// Flush all data to disk.
    ///
    /// redb's write transactions are durable on commit; there is no separate
    /// flush call. Kept as a no-op for API compatibility.
    pub fn flush(&self) -> Result<()> {
        Ok(())
    }

    /// Compact all column families.
    ///
    /// redb performs compaction automatically; no manual call needed. Kept as
    /// a no-op for API compatibility.
    pub fn compact(&self) -> Result<()> {
        Ok(())
    }

    /// Get raw DB handle for advanced operations.
    pub fn db(&self) -> &Database {
        &self.db
    }
}

/// Resolve the on-disk database file path from a caller-supplied path.
///
/// redb is a single-file database, but this crate's public API historically
/// receives a directory (RocksDB used the whole directory). To stay
/// drop-in compatible:
/// - if the path exists and is a directory, use `<path>/meta.redb`;
/// - if it doesn't exist but looks like a directory intent (no extension),
///   treat it as a directory and append `meta.redb`;
/// - otherwise treat the path as a file path directly.
fn resolve_db_path(path: &Path) -> std::path::PathBuf {
    if path.is_dir() {
        path.join("meta.redb")
    } else if path.extension().is_none() {
        // Doesn't exist yet and has no extension — assume directory intent.
        path.join("meta.redb")
    } else {
        path.to_path_buf()
    }
}

/// Map a column-family name string to its `TableDefinition`.
///
/// The returned definition's name is a `&'static str` literal, so despite the
/// `cf_name: &str` input the output borrows nothing from the caller.
fn table_def(cf_name: &str) -> Result<TableDefinition<'static, &'static [u8], &'static [u8]>> {
    match cf_name {
        CF_CATALOG => Ok(T_CATALOG),
        CF_TABLET => Ok(T_TABLET),
        CF_EDIT_LOG => Ok(T_EDIT_LOG),
        other => Err(RocksStoreError::InvalidKey(format!("CF {} not found", other))),
    }
}

/// Decode a big-endian u64 from a byte slice (0 if wrong length).
fn decode_u64(v: &[u8]) -> u64 {
    if v.len() == 8 {
        u64::from_be_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]])
    } else {
        0
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
        // Tables are created on open; verify we can read/write each.
        store.put_cf(CF_CATALOG, b"probe", b"1").unwrap();
        store.put_cf(CF_TABLET, b"probe", b"2").unwrap();
        store.put_cf(CF_EDIT_LOG, b"probe", b"3").unwrap();
        assert_eq!(store.get_cf(CF_CATALOG, b"probe").unwrap(), Some(b"1".to_vec()));
        assert_eq!(store.get_cf(CF_TABLET, b"probe").unwrap(), Some(b"2".to_vec()));
        assert_eq!(store.get_cf(CF_EDIT_LOG, b"probe").unwrap(), Some(b"3".to_vec()));
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
    fn test_iter_prefix() {
        let dir = tempdir().unwrap();
        let store = MetaStore::open(dir.path()).unwrap();

        store.put_cf(CF_CATALOG, b"db:alpha", b"1").unwrap();
        store.put_cf(CF_CATALOG, b"db:beta", b"2").unwrap();
        store.put_cf(CF_CATALOG, b"db:alpha:table:t1", b"3").unwrap();
        store.put_cf(CF_CATALOG, b"unrelated", b"4").unwrap();

        // Prefix scan returns only prefix-matching keys (3 of the 4).
        let mut keys: Vec<Vec<u8>> = store
            .iter_prefix(CF_CATALOG, b"db:")
            .unwrap()
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        assert_eq!(keys.len(), 3);
        assert!(keys.iter().all(|k| k.starts_with(b"db:")));
    }
}
