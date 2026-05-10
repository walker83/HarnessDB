//! Edit log store for write-ahead logging of catalog changes.
//!
//! Key schema:
//! - `log:{index}` → EditLogEntry JSON
//! - `last_applied` → u64 (last applied log index)
//! - `current_term` → u64 (current term number)

use crate::meta_store::{MetaStore, CF_EDIT_LOG, KEY_LOG, KEY_LAST_APPLIED, KEY_CURRENT_TERM, Result, RocksStoreError};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use tracing::debug;
use std::sync::Arc;

/// Operation type for edit log entries (mirrors fe-common::edit_log::OpType).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpType {
    // Catalog ops
    CreateDatabase,
    DropDatabase,
    CreateTable,
    DropTable,
    AlterDatabase,
    AlterTable,
    // Tablet ops
    CreateTablet,
    DropTablet,
    AlterTablet,
    // Node ops
    AddBackend,
    RemoveBackend,
    // Stats ops
    UpdateStats,
}

/// Edit log entry (mirrors fe-common::edit_log::EditLogEntry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditLogEntry {
    pub term: u64,
    pub index: u64,
    pub op_type: OpType,
    pub data: Vec<u8>,
}

/// Edit log store for managing WAL entries in RocksDB.
pub struct EditLogStore {
    store: Arc<MetaStore>,
}

impl EditLogStore {
    /// Create a new EditLogStore.
    pub fn new(store: Arc<MetaStore>) -> Self {
        Self { store }
    }

    /// Append a new log entry and return its index.
    pub fn append(&self, op_type: OpType, data: Vec<u8>) -> Result<u64> {
        // Increment current term
        let current_term = self.store.increment_counter(CF_EDIT_LOG, KEY_CURRENT_TERM.as_bytes())?;

        // Get next index (based on last_applied)
        let last_applied = self.get_last_applied()?;
        let index = last_applied + 1;

        // Create entry
        let entry = EditLogEntry {
            term: current_term,
            index,
            op_type,
            data,
        };

        // Write entry
        let key = format!("{}{}", KEY_LOG, index);
        let value = serialize(&entry)?;
        self.store.put_cf(CF_EDIT_LOG, key.as_bytes(), &value)?;

        // Update last_applied
        self.set_last_applied(index)?;

        debug!("Appended log entry: index={}, term={}, op_type={:?}", index, current_term, op_type);
        Ok(index)
    }

    /// Get a specific log entry by index.
    pub fn get_entry(&self, index: u64) -> Result<Option<EditLogEntry>> {
        let key = format!("{}{}", KEY_LOG, index);
        let value = self.store.get_cf(CF_EDIT_LOG, key.as_bytes())?;
        value.map(|v| deserialize(&v)).transpose()
    }

    /// Get all log entries in order.
    pub fn get_all_entries(&self) -> Result<Vec<EditLogEntry>> {
        let prefix = KEY_LOG.as_bytes();
        let keys = self.list_keys_with_prefix(CF_EDIT_LOG, prefix)?;

        keys.iter()
            .filter_map(|k| {
                let key_str = String::from_utf8_lossy(k);
                let index_str = &key_str[prefix.len()..];
                if let Ok(index) = index_str.parse::<u64>() {
                    self.get_entry(index).transpose()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Replay all log entries from the beginning.
    pub fn replay(&self) -> Result<Vec<EditLogEntry>> {
        self.get_all_entries()
    }

    /// Get the last applied log index.
    pub fn get_last_applied(&self) -> Result<u64> {
        self.store.get_counter(CF_EDIT_LOG, KEY_LAST_APPLIED.as_bytes())
    }

    /// Set the last applied log index.
    pub fn set_last_applied(&self, index: u64) -> Result<()> {
        self.store.set_counter(CF_EDIT_LOG, KEY_LAST_APPLIED.as_bytes(), index)
    }

    /// Get the current term number.
    pub fn get_current_term(&self) -> Result<u64> {
        self.store.get_counter(CF_EDIT_LOG, KEY_CURRENT_TERM.as_bytes())
    }

    /// Set the current term number.
    pub fn set_current_term(&self, term: u64) -> Result<()> {
        self.store.set_counter(CF_EDIT_LOG, KEY_CURRENT_TERM.as_bytes(), term)
    }

    /// Clear all log entries (for testing/reset).
    pub fn clear(&self) -> Result<()> {
        let prefix = KEY_LOG.as_bytes();
        self.delete_keys_with_prefix(CF_EDIT_LOG, prefix)?;
        self.set_last_applied(0)?;
        self.set_current_term(0)?;
        Ok(())
    }

    /// Truncate log entries after a given index (for compaction).
    pub fn truncate_after(&self, index: u64) -> Result<()> {
        let prefix = KEY_LOG.as_bytes();
        let keys = self.list_keys_with_prefix(CF_EDIT_LOG, prefix)?;

        for key in keys {
            let key_str = String::from_utf8_lossy(&key);
            let entry_index_str = &key_str[prefix.len()..];
            if let Ok(entry_index) = entry_index_str.parse::<u64>() {
                if entry_index > index {
                    self.store.delete_cf(CF_EDIT_LOG, &key)?;
                }
            }
        }
        Ok(())
    }

    /// Compact log entries up to a given index (delete old entries).
    pub fn compact_up_to(&self, index: u64) -> Result<()> {
        let prefix = KEY_LOG.as_bytes();
        let keys = self.list_keys_with_prefix(CF_EDIT_LOG, prefix)?;

        for key in keys {
            let key_str = String::from_utf8_lossy(&key);
            let entry_index_str = &key_str[prefix.len()..];
            if let Ok(entry_index) = entry_index_str.parse::<u64>() {
                if entry_index <= index {
                    self.store.delete_cf(CF_EDIT_LOG, &key)?;
                }
            }
        }
        Ok(())
    }

    /// Flush to disk.
    pub fn flush(&self) -> Result<()> {
        self.store.flush()
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

    #[test]
    fn test_append() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        let idx = store.append(OpType::CreateDatabase, b"mydb".to_vec()).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(store.get_last_applied().unwrap(), 1);
        assert_eq!(store.get_current_term().unwrap(), 1);
    }

    #[test]
    fn test_append_multiple() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.append(OpType::CreateDatabase, b"db1".to_vec()).unwrap();
        store.append(OpType::CreateTable, b"t1".to_vec()).unwrap();
        store.append(OpType::DropDatabase, b"db1".to_vec()).unwrap();

        assert_eq!(store.get_last_applied().unwrap(), 3);
        assert_eq!(store.get_current_term().unwrap(), 3);
    }

    #[test]
    fn test_get_entry() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.append(OpType::CreateDatabase, b"mydb".to_vec()).unwrap();

        let entry = store.get_entry(1).unwrap();
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.index, 1);
        assert_eq!(entry.op_type, OpType::CreateDatabase);
        assert_eq!(entry.data, b"mydb");
    }

    #[test]
    fn test_get_nonexistent_entry() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        let entry = store.get_entry(999).unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn test_replay() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.append(OpType::CreateDatabase, b"db1".to_vec()).unwrap();
        store.append(OpType::CreateTable, b"t1".to_vec()).unwrap();
        store.append(OpType::DropDatabase, b"db1".to_vec()).unwrap();

        let entries = store.replay().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].op_type, OpType::CreateDatabase);
        assert_eq!(entries[1].op_type, OpType::CreateTable);
        assert_eq!(entries[2].op_type, OpType::DropDatabase);
    }

    #[test]
    fn test_clear() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.append(OpType::CreateDatabase, b"db1".to_vec()).unwrap();
        store.append(OpType::CreateTable, b"t1".to_vec()).unwrap();
        store.clear().unwrap();

        assert_eq!(store.get_last_applied().unwrap(), 0);
        assert_eq!(store.get_current_term().unwrap(), 0);
        assert!(store.replay().unwrap().is_empty());
    }

    #[test]
    fn test_compact_up_to() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.append(OpType::CreateDatabase, b"db1".to_vec()).unwrap();
        store.append(OpType::CreateTable, b"t1".to_vec()).unwrap();
        store.append(OpType::DropDatabase, b"db1".to_vec()).unwrap();

        // Compact entries up to index 2
        store.compact_up_to(2).unwrap();

        // Only entry 3 should remain
        let entries = store.replay().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, 3);
    }

    #[test]
    fn test_set_counters() {
        let dir = tempdir().unwrap();
        let store = EditLogStore::new(MetaStore::open(dir.path()).unwrap());

        store.set_last_applied(100).unwrap();
        store.set_current_term(50).unwrap();

        assert_eq!(store.get_last_applied().unwrap(), 100);
        assert_eq!(store.get_current_term().unwrap(), 50);
    }
}