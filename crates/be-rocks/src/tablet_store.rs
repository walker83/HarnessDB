//! Tablet store for tablet schema and rowset metadata.
//!
//! Key schema:
//! - `tablet:{id}:schema` → TabletSchema JSON
//! - `tablet:{id}:rowset:{rs_id}` → RowsetMeta JSON + segments JSON
//! - `tablet:{id}:next_rowset_id` → Atomic u64 counter
//! - `tablet:{id}:next_segment_id` → Atomic u64 counter

use crate::meta_store::{MetaStore, CF_TABLET, KEY_TABLET_SCHEMA, KEY_ROWSET, KEY_NEXT_ROWSET_ID, KEY_NEXT_SEGMENT_ID, Result, RocksStoreError};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use tracing::debug;
use types::DataType;
use std::sync::Arc;

/// Tablet column definition (mirrors be-storage::TabletColumn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub is_key: bool,
    pub agg_type: Option<String>,
}

/// Tablet schema definition (mirrors be-storage::TabletSchema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletSchema {
    pub tablet_id: u64,
    pub columns: Vec<TabletColumn>,
    pub keys_type: String,
    pub num_rows_per_row_block: usize,
}

/// Metadata for a rowset (mirrors be-storage::RowsetMeta).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowsetMeta {
    pub rowset_id: u64,
    pub tablet_id: u64,
    pub txn_id: u64,
    pub version: u64,
    pub num_rows: u64,
    pub data_size: u64,
    pub num_segments: u32,
    pub empty: bool,
    pub packed_data_size: u64,
    pub index_size: u64,
}

/// Reference to a segment file (mirrors be-storage::SegmentRef).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRef {
    pub segment_id: u64,
    pub path: String,
    pub num_rows: u64,
    pub size: u64,
}

/// Tablet store for managing tablet metadata in RocksDB.
pub struct TabletStore {
    store: Arc<MetaStore>,
}

impl TabletStore {
    /// Create a new TabletStore.
    pub fn new(store: Arc<MetaStore>) -> Self {
        Self { store }
    }

    /// Put a tablet schema.
    pub fn put_schema(&self, tablet_id: u64, schema: &TabletSchema) -> Result<()> {
        let key = format!("{}{}:schema", KEY_TABLET_SCHEMA, tablet_id);
        let value = serialize(schema)?;
        self.store.put_cf(CF_TABLET, key.as_bytes(), &value)?;
        debug!("Put tablet schema: {}", tablet_id);
        Ok(())
    }

    /// Get a tablet schema.
    pub fn get_schema(&self, tablet_id: u64) -> Result<Option<TabletSchema>> {
        let key = format!("{}{}:schema", KEY_TABLET_SCHEMA, tablet_id);
        let value = self.store.get_cf(CF_TABLET, key.as_bytes())?;
        value.map(|v| deserialize(&v)).transpose()
    }

    /// Delete a tablet schema.
    pub fn delete_schema(&self, tablet_id: u64) -> Result<()> {
        let key = format!("{}{}:schema", KEY_TABLET_SCHEMA, tablet_id);
        self.store.delete_cf(CF_TABLET, key.as_bytes())?;
        debug!("Deleted tablet schema: {}", tablet_id);
        Ok(())
    }

    /// Put a rowset with its segments.
    pub fn put_rowset(&self, tablet_id: u64, rowset_id: u64, meta: &RowsetMeta, segments: &[SegmentRef]) -> Result<()> {
        let key = format!("{}{}:{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_ROWSET, rowset_id);
        let value = serialize(&(meta, segments))?;
        self.store.put_cf(CF_TABLET, key.as_bytes(), &value)?;
        debug!("Put rowset: tablet={}, rowset={}", tablet_id, rowset_id);
        Ok(())
    }

    /// Get a rowset with its segments.
    pub fn get_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<Option<(RowsetMeta, Vec<SegmentRef>)>> {
        let key = format!("{}{}:{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_ROWSET, rowset_id);
        let value = self.store.get_cf(CF_TABLET, key.as_bytes())?;
        value.map(|v| deserialize(&v)).transpose()
    }

    /// Delete a rowset.
    pub fn delete_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<()> {
        let key = format!("{}{}:{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_ROWSET, rowset_id);
        self.store.delete_cf(CF_TABLET, key.as_bytes())?;
        debug!("Deleted rowset: tablet={}, rowset={}", tablet_id, rowset_id);
        Ok(())
    }

    /// List all rowset IDs for a tablet.
    pub fn list_rowsets(&self, tablet_id: u64) -> Result<Vec<u64>> {
        let prefix = format!("{}{}:{}", KEY_TABLET_SCHEMA, tablet_id, KEY_ROWSET);
        let keys = self.list_keys_with_prefix(CF_TABLET, prefix.as_bytes())?;
        keys.iter()
            .map(|k| {
                let id_str = String::from_utf8_lossy(&k[prefix.len()..]);
                id_str.parse::<u64>()
                    .map_err(|e| RocksStoreError::InvalidKey(e.to_string()))
            })
            .collect()
    }

    /// Get all rowsets for a tablet.
    pub fn get_all_rowsets(&self, tablet_id: u64) -> Result<Vec<(RowsetMeta, Vec<SegmentRef>)>> {
        let rowset_ids = self.list_rowsets(tablet_id)?;
        rowset_ids
            .iter()
            .filter_map(|id| self.get_rowset(tablet_id, *id).transpose())
            .collect()
    }

    /// Delete all rowsets for a tablet.
    pub fn delete_all_rowsets(&self, tablet_id: u64) -> Result<()> {
        let prefix = format!("{}{}:{}", KEY_TABLET_SCHEMA, tablet_id, KEY_ROWSET);
        self.delete_keys_with_prefix(CF_TABLET, prefix.as_bytes())?;
        Ok(())
    }

    /// Delete all metadata for a tablet (schema + all rowsets + counters).
    pub fn delete_tablet(&self, tablet_id: u64) -> Result<()> {
        let prefix = format!("{}{}", KEY_TABLET_SCHEMA, tablet_id);
        self.delete_keys_with_prefix(CF_TABLET, prefix.as_bytes())?;
        debug!("Deleted tablet: {}", tablet_id);
        Ok(())
    }

    /// Get the next rowset ID and increment the counter atomically.
    pub fn next_rowset_id(&self, tablet_id: u64) -> Result<u64> {
        let key = format!("{}{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_NEXT_ROWSET_ID);
        self.store.increment_counter(CF_TABLET, key.as_bytes())
    }

    /// Get the next segment ID and increment the counter atomically.
    pub fn next_segment_id(&self, tablet_id: u64) -> Result<u64> {
        let key = format!("{}{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_NEXT_SEGMENT_ID);
        self.store.increment_counter(CF_TABLET, key.as_bytes())
    }

    /// Set rowset ID counter (for migration).
    pub fn set_next_rowset_id(&self, tablet_id: u64, value: u64) -> Result<()> {
        let key = format!("{}{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_NEXT_ROWSET_ID);
        self.store.set_counter(CF_TABLET, key.as_bytes(), value)
    }

    /// Set segment ID counter (for migration).
    pub fn set_next_segment_id(&self, tablet_id: u64, value: u64) -> Result<()> {
        let key = format!("{}{}{}", KEY_TABLET_SCHEMA, tablet_id, KEY_NEXT_SEGMENT_ID);
        self.store.set_counter(CF_TABLET, key.as_bytes(), value)
    }

    /// List all tablet IDs.
    pub fn list_tablets(&self) -> Result<Vec<u64>> {
        let prefix = KEY_TABLET_SCHEMA.as_bytes();
        let keys = self.list_keys_with_prefix(CF_TABLET, prefix)?;

        // Extract tablet IDs from keys like "tablet:{id}:schema"
        let mut tablet_ids = Vec::new();
        for key in keys {
            let key_str = String::from_utf8_lossy(&key);
            if key_str.contains(":schema") {
                // Extract ID from "tablet:{id}:schema"
                let parts: Vec<&str> = key_str.split(':').collect();
                if parts.len() >= 3 && parts[0] == "tablet" {
                    if let Ok(id) = parts[1].parse::<u64>() {
                        tablet_ids.push(id);
                    }
                }
            }
        }
        Ok(tablet_ids)
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

    fn make_test_schema(tablet_id: u64) -> TabletSchema {
        TabletSchema {
            tablet_id,
            columns: vec![
                TabletColumn {
                    name: "id".to_string(),
                    data_type: DataType::Int64,
                    nullable: false,
                    is_key: true,
                    agg_type: None,
                },
                TabletColumn {
                    name: "value".to_string(),
                    data_type: DataType::Float64,
                    nullable: true,
                    is_key: false,
                    agg_type: None,
                },
            ],
            keys_type: "duplicate".to_string(),
            num_rows_per_row_block: 1024,
        }
    }

    fn make_test_rowset(tablet_id: u64, rowset_id: u64) -> RowsetMeta {
        RowsetMeta {
            rowset_id,
            tablet_id,
            txn_id: 0,
            version: 1,
            num_rows: 0,
            data_size: 0,
            num_segments: 0,
            empty: true,
            packed_data_size: 0,
            index_size: 0,
        }
    }

    fn make_test_segment(segment_id: u64) -> SegmentRef {
        SegmentRef {
            segment_id,
            path: format!("seg_{}.dat", segment_id),
            num_rows: 100,
            size: 4096,
        }
    }

    #[test]
    fn test_put_and_get_schema() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        let schema = make_test_schema(1);
        store.put_schema(1, &schema).unwrap();

        let retrieved = store.get_schema(1).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.tablet_id, 1);
        assert_eq!(retrieved.columns.len(), 2);
    }

    #[test]
    fn test_delete_schema() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        let schema = make_test_schema(1);
        store.put_schema(1, &schema).unwrap();
        store.delete_schema(1).unwrap();

        let retrieved = store.get_schema(1).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_put_and_get_rowset() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        let meta = make_test_rowset(1, 1);
        let segments = vec![make_test_segment(1), make_test_segment(2)];
        store.put_rowset(1, 1, &meta, &segments).unwrap();

        let retrieved = store.get_rowset(1, 1).unwrap();
        assert!(retrieved.is_some());
        let (meta, segments) = retrieved.unwrap();
        assert_eq!(meta.rowset_id, 1);
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_list_rowsets() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_rowset(1, 1, &make_test_rowset(1, 1), &vec![make_test_segment(1)]).unwrap();
        store.put_rowset(1, 2, &make_test_rowset(1, 2), &vec![make_test_segment(2)]).unwrap();
        store.put_rowset(1, 3, &make_test_rowset(1, 3), &vec![make_test_segment(3)]).unwrap();

        let rowsets = store.list_rowsets(1).unwrap();
        assert_eq!(rowsets.len(), 3);
        assert!(rowsets.contains(&1));
        assert!(rowsets.contains(&2));
        assert!(rowsets.contains(&3));
    }

    #[test]
    fn test_delete_rowset() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_rowset(1, 1, &make_test_rowset(1, 1), &vec![make_test_segment(1)]).unwrap();
        store.delete_rowset(1, 1).unwrap();

        let retrieved = store.get_rowset(1, 1).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_next_rowset_id() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        let id1 = store.next_rowset_id(1).unwrap();
        let id2 = store.next_rowset_id(1).unwrap();
        let id3 = store.next_rowset_id(1).unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_next_segment_id() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        let id1 = store.next_segment_id(1).unwrap();
        let id2 = store.next_segment_id(1).unwrap();
        let id3 = store.next_segment_id(1).unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_delete_tablet() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_schema(1, &make_test_schema(1)).unwrap();
        store.put_rowset(1, 1, &make_test_rowset(1, 1), &vec![]).unwrap();
        store.put_rowset(1, 2, &make_test_rowset(1, 2), &vec![]).unwrap();

        store.delete_tablet(1).unwrap();

        // Everything should be deleted
        assert!(store.get_schema(1).unwrap().is_none());
        assert!(store.list_rowsets(1).unwrap().is_empty());
    }

    #[test]
    fn test_list_tablets() {
        let dir = tempdir().unwrap();
        let store = TabletStore::new(MetaStore::open(dir.path()).unwrap());

        store.put_schema(1, &make_test_schema(1)).unwrap();
        store.put_schema(2, &make_test_schema(2)).unwrap();
        store.put_schema(3, &make_test_schema(3)).unwrap();

        let tablets = store.list_tablets().unwrap();
        assert_eq!(tablets.len(), 3);
        assert!(tablets.contains(&1));
        assert!(tablets.contains(&2));
        assert!(tablets.contains(&3));
    }
}