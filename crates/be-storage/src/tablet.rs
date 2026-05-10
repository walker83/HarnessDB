use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use types::{Block, DataType, Field, Schema, Vector, ScalarValue};

use crate::rowset::{Rowset, RowsetMeta, SegmentRef, RowsetState};
use crate::segment::{SegmentWriter, SegmentReader};
use crate::index::ColumnPredicate;

/// Tablet column definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabletColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub is_key: bool,
    pub agg_type: Option<String>,
}

/// Tablet schema definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabletSchema {
    pub tablet_id: u64,
    pub columns: Vec<TabletColumn>,
    pub keys_type: String,
    pub num_rows_per_row_block: usize,
}

impl TabletSchema {
    pub fn to_schema(&self) -> Schema {
        let fields: Vec<Field> = self
            .columns
            .iter()
            .map(|c| Field::new(&c.name, c.data_type.clone(), c.nullable))
            .collect();
        Schema::new(fields)
    }
}

/// Error type for tablet metadata backend operations.
#[derive(Debug, thiserror::Error)]
pub enum TabletMetaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Path error: {0}")]
    Path(String),
    #[error("RocksDB error: {0}")]
    RocksDb(String),
    #[error("Backend not available: {0}")]
    NotAvailable(String),
}

/// Backend trait for tablet metadata storage.
/// Allows swapping between JSON files and RocksDB storage.
pub trait TabletMetaBackend: Send + Sync {
    /// Save tablet schema.
    fn save_schema(&self, tablet_id: u64, schema: &TabletSchema) -> Result<(), TabletMetaError>;

    /// Load tablet schema.
    fn load_schema(&self, tablet_id: u64) -> Result<Option<TabletSchema>, TabletMetaError>;

    /// Save rowset metadata.
    fn save_rowset(&self, tablet_id: u64, rowset_id: u64, meta: &RowsetMeta, segments: &[SegmentRef]) -> Result<(), TabletMetaError>;

    /// Load rowset metadata.
    fn load_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<Option<(RowsetMeta, Vec<SegmentRef>)>, TabletMetaError>;

    /// List all rowset IDs for a tablet.
    fn list_rowsets(&self, tablet_id: u64) -> Result<Vec<u64>, TabletMetaError>;

    /// Delete a rowset.
    fn delete_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<(), TabletMetaError>;

    /// Delete all metadata for a tablet.
    fn delete_tablet(&self, tablet_id: u64) -> Result<(), TabletMetaError>;

    /// Get the next rowset ID (atomically increment and return).
    fn next_rowset_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError>;

    /// Get the next segment ID (atomically increment and return).
    fn next_segment_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError>;

    /// Set the next rowset ID counter (for migration).
    fn set_next_rowset_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError>;

    /// Set the next segment ID counter (for migration).
    fn set_next_segment_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError>;

    /// Flush any pending writes.
    fn flush(&self) -> Result<(), TabletMetaError>;
}

/// JSON file-based tablet metadata backend.
/// Stores metadata in `{tablet_dir}/schema.json` and `{tablet_dir}/rowset_{id}.json`.
pub struct JsonTabletMetaBackend {
    data_dir: PathBuf,
}

impl JsonTabletMetaBackend {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn tablet_dir(&self, tablet_id: u64) -> PathBuf {
        self.data_dir.join(format!("tablet_{}", tablet_id))
    }

    fn schema_path(&self, tablet_id: u64) -> PathBuf {
        self.tablet_dir(tablet_id).join("schema.json")
    }

    fn rowset_path(&self, tablet_id: u64, rowset_id: u64) -> PathBuf {
        self.tablet_dir(tablet_id).join(format!("rowset_{}.json", rowset_id))
    }
}

impl TabletMetaBackend for JsonTabletMetaBackend {
    fn save_schema(&self, tablet_id: u64, schema: &TabletSchema) -> Result<(), TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        std::fs::create_dir_all(&tablet_dir)?;
        let path = self.schema_path(tablet_id);
        let json = serde_json::to_string_pretty(schema)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn load_schema(&self, tablet_id: u64) -> Result<Option<TabletSchema>, TabletMetaError> {
        let path = self.schema_path(tablet_id);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let schema: TabletSchema = serde_json::from_str(&json)?;
        Ok(Some(schema))
    }

    fn save_rowset(&self, tablet_id: u64, rowset_id: u64, meta: &RowsetMeta, segments: &[SegmentRef]) -> Result<(), TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        std::fs::create_dir_all(&tablet_dir)?;
        let path = self.rowset_path(tablet_id, rowset_id);
        let json = serde_json::to_string_pretty(&(meta, segments))?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn load_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<Option<(RowsetMeta, Vec<SegmentRef>)>, TabletMetaError> {
        let path = self.rowset_path(tablet_id, rowset_id);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let data: (RowsetMeta, Vec<SegmentRef>) = serde_json::from_str(&json)?;
        Ok(Some(data))
    }

    fn list_rowsets(&self, tablet_id: u64) -> Result<Vec<u64>, TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        if !tablet_dir.exists() {
            return Ok(Vec::new());
        }

        let mut rowset_ids = Vec::new();
        for entry in std::fs::read_dir(&tablet_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_name.starts_with("rowset_") {
                        if let Ok(id) = file_name[7..].parse::<u64>() {
                            rowset_ids.push(id);
                        }
                    }
                }
            }
        }
        rowset_ids.sort();
        Ok(rowset_ids)
    }

    fn delete_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<(), TabletMetaError> {
        let path = self.rowset_path(tablet_id, rowset_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn delete_tablet(&self, tablet_id: u64) -> Result<(), TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        if tablet_dir.exists() {
            std::fs::remove_dir_all(tablet_dir)?;
        }
        Ok(())
    }

    fn next_rowset_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
        // JSON backend doesn't have atomic counters, read from file
        let path = self.tablet_dir(tablet_id).join("next_rowset_id");
        let current = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            data.parse::<u64>().unwrap_or(0)
        } else {
            // Infer from existing rowsets
            let rowsets = self.list_rowsets(tablet_id)?;
            rowsets.into_iter().max().map(|m| m + 1).unwrap_or(1)
        };
        std::fs::write(&path, (current + 1).to_string())?;
        Ok(current + 1)
    }

    fn next_segment_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
        // JSON backend doesn't have atomic counters, read from file
        let path = self.tablet_dir(tablet_id).join("next_segment_id");
        let current = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            data.parse::<u64>().unwrap_or(0)
        } else {
            // Infer from existing segment files
            let tablet_dir = self.tablet_dir(tablet_id);
            let mut max_id = 0u64;
            if tablet_dir.exists() {
                for entry in std::fs::read_dir(&tablet_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("dat") {
                        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                            if file_name.starts_with("seg_") {
                                if let Ok(id) = file_name[4..].parse::<u64>() {
                                    if id > max_id {
                                        max_id = id;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            max_id + 1
        };
        std::fs::write(&path, (current + 1).to_string())?;
        Ok(current + 1)
    }

    fn set_next_rowset_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        std::fs::create_dir_all(&tablet_dir)?;
        let path = tablet_dir.join("next_rowset_id");
        std::fs::write(&path, value.to_string())?;
        Ok(())
    }

    fn set_next_segment_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
        let tablet_dir = self.tablet_dir(tablet_id);
        std::fs::create_dir_all(&tablet_dir)?;
        let path = tablet_dir.join("next_segment_id");
        std::fs::write(&path, value.to_string())?;
        Ok(())
    }

    fn flush(&self) -> Result<(), TabletMetaError> {
        // JSON backend writes directly to disk, no flush needed
        Ok(())
    }
}

#[cfg(feature = "rocksdb")]
mod rocks_backend {
    use super::*;
    use be_rocks::{MetaStore, TabletStore};

    /// RocksDB-based tablet metadata backend.
    /// Stores all metadata in a central RocksDB instance.
    pub struct RocksTabletMetaBackend {
        store: TabletStore,
    }

    impl RocksTabletMetaBackend {
        pub fn new(store: MetaStore) -> Self {
            Self { store: TabletStore::new(store) }
        }

        pub fn from_tablet_store(store: TabletStore) -> Self {
            Self { store }
        }
    }

    impl TabletMetaBackend for RocksTabletMetaBackend {
        fn save_schema(&self, tablet_id: u64, schema: &TabletSchema) -> Result<(), TabletMetaError> {
            self.store.put_schema(tablet_id, schema)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn load_schema(&self, tablet_id: u64) -> Result<Option<TabletSchema>, TabletMetaError> {
            self.store.get_schema(tablet_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn save_rowset(&self, tablet_id: u64, rowset_id: u64, meta: &RowsetMeta, segments: &[SegmentRef]) -> Result<(), TabletMetaError> {
            self.store.put_rowset(tablet_id, rowset_id, meta, segments)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn load_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<Option<(RowsetMeta, Vec<SegmentRef>)>, TabletMetaError> {
            self.store.get_rowset(tablet_id, rowset_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn list_rowsets(&self, tablet_id: u64) -> Result<Vec<u64>, TabletMetaError> {
            self.store.list_rowsets(tablet_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn delete_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<(), TabletMetaError> {
            self.store.delete_rowset(tablet_id, rowset_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn delete_tablet(&self, tablet_id: u64) -> Result<(), TabletMetaError> {
            self.store.delete_tablet(tablet_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn next_rowset_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
            self.store.next_rowset_id(tablet_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn next_segment_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
            self.store.next_segment_id(tablet_id)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn set_next_rowset_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
            self.store.set_next_rowset_id(tablet_id, value)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn set_next_segment_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
            self.store.set_next_segment_id(tablet_id, value)
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }

        fn flush(&self) -> Result<(), TabletMetaError> {
            self.store.flush()
                .map_err(|e| TabletMetaError::RocksDb(e.to_string()))
        }
    }
}

#[cfg(feature = "rocksdb")]
pub use rocks_backend::RocksTabletMetaBackend;

/// Dual-write backend that writes to both JSON and RocksDB.
/// Useful for migration - write to both, read from primary.
pub struct DualWriteBackend {
    primary: Arc<dyn TabletMetaBackend>,
    secondary: Arc<dyn TabletMetaBackend>,
}

impl DualWriteBackend {
    pub fn new(primary: Arc<dyn TabletMetaBackend>, secondary: Arc<dyn TabletMetaBackend>) -> Self {
        Self { primary, secondary }
    }
}

impl TabletMetaBackend for DualWriteBackend {
    fn save_schema(&self, tablet_id: u64, schema: &TabletSchema) -> Result<(), TabletMetaError> {
        self.primary.save_schema(tablet_id, schema)?;
        // Best-effort write to secondary
        let _ = self.secondary.save_schema(tablet_id, schema);
        Ok(())
    }

    fn load_schema(&self, tablet_id: u64) -> Result<Option<TabletSchema>, TabletMetaError> {
        self.primary.load_schema(tablet_id)
    }

    fn save_rowset(&self, tablet_id: u64, rowset_id: u64, meta: &RowsetMeta, segments: &[SegmentRef]) -> Result<(), TabletMetaError> {
        self.primary.save_rowset(tablet_id, rowset_id, meta, segments)?;
        let _ = self.secondary.save_rowset(tablet_id, rowset_id, meta, segments);
        Ok(())
    }

    fn load_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<Option<(RowsetMeta, Vec<SegmentRef>)>, TabletMetaError> {
        self.primary.load_rowset(tablet_id, rowset_id)
    }

    fn list_rowsets(&self, tablet_id: u64) -> Result<Vec<u64>, TabletMetaError> {
        self.primary.list_rowsets(tablet_id)
    }

    fn delete_rowset(&self, tablet_id: u64, rowset_id: u64) -> Result<(), TabletMetaError> {
        self.primary.delete_rowset(tablet_id, rowset_id)?;
        let _ = self.secondary.delete_rowset(tablet_id, rowset_id);
        Ok(())
    }

    fn delete_tablet(&self, tablet_id: u64) -> Result<(), TabletMetaError> {
        self.primary.delete_tablet(tablet_id)?;
        let _ = self.secondary.delete_tablet(tablet_id);
        Ok(())
    }

    fn next_rowset_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
        // Use primary for atomic counter
        self.primary.next_rowset_id(tablet_id)
    }

    fn next_segment_id(&self, tablet_id: u64) -> Result<u64, TabletMetaError> {
        self.primary.next_segment_id(tablet_id)
    }

    fn set_next_rowset_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
        self.primary.set_next_rowset_id(tablet_id, value)?;
        let _ = self.secondary.set_next_rowset_id(tablet_id, value);
        Ok(())
    }

    fn set_next_segment_id(&self, tablet_id: u64, value: u64) -> Result<(), TabletMetaError> {
        self.primary.set_next_segment_id(tablet_id, value)?;
        let _ = self.secondary.set_next_segment_id(tablet_id, value);
        Ok(())
    }

    fn flush(&self) -> Result<(), TabletMetaError> {
        self.primary.flush()?;
        let _ = self.secondary.flush();
        Ok(())
    }
}

/// A key for the memtable BTreeMap. Uses a composite of sort key values.
/// For simplicity, we use a row-level string key derived from key columns.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemTableKey(Vec<u8>);

impl MemTableKey {
    pub fn from_i64(v: i64) -> Self {
        Self(v.to_be_bytes().to_vec())
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

/// In-memory write buffer for a tablet.
/// Stores rows in sorted order by key for efficient flushing.
pub struct MemTable {
    rows: BTreeMap<MemTableKey, ColumnarRow>,
    memory_size: u64,
    capacity: u64,
    #[allow(dead_code)]
    schema: TabletSchema,
    /// Monotonically increasing row counter to ensure unique keys for Duplicate tables.
    next_row_id: u64,
}

#[derive(Clone)]
struct ColumnarRow {
    columns: Vec<ScalarValue>,
}

impl ColumnarRow {
    fn new(columns: Vec<ScalarValue>) -> Self {
        Self { columns }
    }

    fn memory_size(&self) -> u64 {
        let mut size = 0u64;
        for val in &self.columns {
            size += match val {
                ScalarValue::Boolean(_) => 1,
                ScalarValue::Int8(_) => 1,
                ScalarValue::Int16(_) => 2,
                ScalarValue::Int32(_) => 4,
                ScalarValue::Int64(_) => 8,
                ScalarValue::Int128(_) => 16,
                ScalarValue::Float32(_) => 4,
                ScalarValue::Float64(_) => 8,
                ScalarValue::String(s) => s.len() as u64 + 8,
                ScalarValue::Date(_) => 4,
                ScalarValue::DateTime(_) => 8,
                ScalarValue::Json(_) => 64,
                ScalarValue::Null => 0,
                ScalarValue::Binary(b) => b.len() as u64 + 8,
                ScalarValue::Array(a) => a.len() as u64 * 8 + 8,
                ScalarValue::Float32Array(arr) => arr.len() as u64 * 4 + 8,
            };
        }
        size
    }
}

impl MemTable {
    pub fn new(capacity: u64, schema: TabletSchema) -> Self {
        Self {
            rows: BTreeMap::new(),
            memory_size: 0,
            capacity,
            schema,
            next_row_id: 0,
        }
    }

    pub fn insert(&mut self, block: &Block, key_column_idx: usize) -> Result<(), String> {
        for row_idx in 0..block.num_rows() {
            let mut key = self.extract_key(block, row_idx, key_column_idx)?;
            // Append a unique row ID suffix to prevent duplicate key overwrites
            // for Duplicate-key tables where multiple rows share the same key value.
            let row_id = self.next_row_id;
            self.next_row_id += 1;
            key.0.extend_from_slice(&row_id.to_be_bytes());
            let row_values: Vec<ScalarValue> = (0..block.num_columns())
                .map(|col_idx| {
                    if let Some(col) = block.column(col_idx) {
                        col.scalar_at(row_idx)
                    } else {
                        ScalarValue::Null
                    }
                })
                .collect();

            let row = ColumnarRow::new(row_values);
            self.memory_size += row.memory_size();
            self.rows.insert(key, row);
        }
        Ok(())
    }

    fn coerce_scalar(value: &ScalarValue, target_type: &DataType) -> ScalarValue {
        match (value, target_type) {
            (ScalarValue::Int64(n), DataType::Int8) => ScalarValue::Int8(*n as i8),
            (ScalarValue::Int64(n), DataType::Int16) => ScalarValue::Int16(*n as i16),
            (ScalarValue::Int64(n), DataType::Int32) => ScalarValue::Int32(*n as i32),
            (ScalarValue::Int64(n), DataType::Float32) => ScalarValue::Float32(*n as f32),
            (ScalarValue::Int64(n), DataType::Float64) => ScalarValue::Float64(*n as f64),
            (ScalarValue::Int32(n), DataType::Int8) => ScalarValue::Int8(*n as i8),
            (ScalarValue::Int32(n), DataType::Int16) => ScalarValue::Int16(*n as i16),
            (ScalarValue::Int32(n), DataType::Int64) => ScalarValue::Int64(*n as i64),
            (ScalarValue::Int32(n), DataType::Float32) => ScalarValue::Float32(*n as f32),
            (ScalarValue::Int32(n), DataType::Float64) => ScalarValue::Float64(*n as f64),
            (ScalarValue::Float64(f), DataType::Float32) => ScalarValue::Float32(*f as f32),
            (ScalarValue::Null, _) => ScalarValue::Null,
            _ => value.clone(),
        }
    }

    pub fn to_block(&self, schema: &Schema) -> Block {
        if self.rows.is_empty() {
            return Block::empty(schema.clone());
        }

        let num_rows = self.rows.len();
        let num_cols = schema.fields().len();

        let mut columns: Vec<Vector> = Vec::with_capacity(num_cols);

        for col_idx in 0..num_cols {
            let field = &schema.fields()[col_idx];
            let scalars: Vec<ScalarValue> = self.rows.values()
                .map(|row| row.columns.get(col_idx).cloned().unwrap_or(ScalarValue::Null))
                .map(|s| Self::coerce_scalar(&s, &field.data_type))
                .collect();

            let vector = match field.data_type {
                DataType::Boolean => {
                    let data: Vec<Option<bool>> = scalars.iter()
                        .map(|s| if let ScalarValue::Boolean(b) = s { Some(*b) } else { None })
                        .collect();
                    Vector::Boolean(types::vector::BooleanVector::from_nullable_vec(data))
                }
                DataType::Int8 => {
                    let data: Vec<Option<i8>> = scalars.iter()
                        .map(|s| if let ScalarValue::Int8(i) = s { Some(*i) } else { None })
                        .collect();
                    Vector::Int8(types::vector::Int8Vector::from_nullable_vec(data))
                }
                DataType::Int16 => {
                    let data: Vec<Option<i16>> = scalars.iter()
                        .map(|s| if let ScalarValue::Int16(i) = s { Some(*i) } else { None })
                        .collect();
                    Vector::Int16(types::vector::Int16Vector::from_nullable_vec(data))
                }
                DataType::Int32 => {
                    let data: Vec<Option<i32>> = scalars.iter()
                        .map(|s| if let ScalarValue::Int32(i) = s { Some(*i) } else { None })
                        .collect();
                    Vector::Int32(types::vector::Int32Vector::from_nullable_vec(data))
                }
                DataType::Int64 => {
                    let data: Vec<Option<i64>> = scalars.iter()
                        .map(|s| if let ScalarValue::Int64(i) = s { Some(*i) } else { None })
                        .collect();
                    Vector::Int64(types::vector::Int64Vector::from_nullable_vec(data))
                }
                DataType::Int128 => {
                    let data: Vec<Option<i128>> = scalars.iter()
                        .map(|s| if let ScalarValue::Int128(i) = s { Some(*i) } else { None })
                        .collect();
                    Vector::Int128(types::vector::Int128Vector::from_nullable_vec(data))
                }
                DataType::Float32 => {
                    let data: Vec<Option<f32>> = scalars.iter()
                        .map(|s| if let ScalarValue::Float32(f) = s { Some(*f) } else { None })
                        .collect();
                    Vector::Float32(types::vector::Float32Vector::from_nullable_vec(data))
                }
                DataType::Float64 => {
                    let data: Vec<Option<f64>> = scalars.iter()
                        .map(|s| if let ScalarValue::Float64(f) = s { Some(*f) } else { None })
                        .collect();
                    Vector::Float64(types::vector::Float64Vector::from_nullable_vec(data))
                }
                DataType::String => {
                    let data: Vec<Option<String>> = scalars.iter()
                        .map(|s| if let ScalarValue::String(s) = s { Some(s.clone()) } else { None })
                        .collect();
                    Vector::String(types::vector::StringVector::from_option_vec(data))
                }
                DataType::Date => {
                    let data: Vec<Option<i32>> = scalars.iter()
                        .map(|s| if let ScalarValue::Date(d) = s { Some(*d) } else { None })
                        .collect();
                    Vector::Date(types::vector::DateVector::from_nullable_vec(data))
                }
                DataType::DateTime => {
                    let data: Vec<Option<i64>> = scalars.iter()
                        .map(|s| if let ScalarValue::DateTime(d) = s { Some(*d) } else { None })
                        .collect();
                    Vector::DateTime(types::vector::DateTimeVector::from_nullable_vec(data))
                }
                _ => Vector::Null(types::vector::NullVector::new(num_rows)),
            };
            columns.push(vector);
        }

        Block::new(schema.clone(), columns)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn memory_size(&self) -> u64 {
        self.memory_size
    }

    pub fn should_flush(&self) -> bool {
        self.memory_size >= self.capacity
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.memory_size = 0;
    }

    /// Delete rows from memtable matching the given predicates.
    pub fn delete(&mut self, block: &Block, key_column_idx: usize, predicates: &[ColumnPredicate]) -> Result<usize, String> {
        let selection = crate::index::apply_predicates_to_block(block, predicates);
        let mut keys_to_remove = Vec::new();

        for row_idx in 0..block.num_rows() {
            if selection.get(row_idx) {
                let prefix = self.extract_key(block, row_idx, key_column_idx)?;
                // Find all entries whose key starts with this prefix (key has row_id suffix)
                let prefix_len = prefix.0.len();
                keys_to_remove.extend(
                    self.rows.keys()
                        .filter(|k| k.0.len() >= prefix_len && k.0[..prefix_len] == prefix.0[..])
                        .cloned()
                );
            }
        }

        let deleted_count = keys_to_remove.len();
        for key in keys_to_remove {
            self.rows.remove(&key);
        }
        Ok(deleted_count)
    }

    fn extract_key(&self, block: &Block, row_idx: usize, col_idx: usize) -> Result<MemTableKey, String> {
        let col = block.column(col_idx)
            .ok_or_else(|| format!("Key column index {} out of bounds", col_idx))?;
        let scalar = col.scalar_at(row_idx);
        Ok(match scalar {
            types::ScalarValue::Int64(v) => MemTableKey::from_i64(v),
            types::ScalarValue::Int32(v) => MemTableKey::from_i64(v as i64),
            types::ScalarValue::String(s) => MemTableKey::from_string(&s),
            types::ScalarValue::Int8(v) => MemTableKey::from_i64(v as i64),
            types::ScalarValue::Int16(v) => MemTableKey::from_i64(v as i64),
            types::ScalarValue::Int128(v) => MemTableKey::from_i64(v as i64),
            types::ScalarValue::Float32(f) => MemTableKey::from_i64(f.to_bits() as i64),
            types::ScalarValue::Float64(f) => MemTableKey::from_i64(f.to_bits() as i64),
            types::ScalarValue::Date(d) => MemTableKey::from_i64(d as i64),
            types::ScalarValue::DateTime(d) => MemTableKey::from_i64(d),
            _ => return Err(format!("Unsupported key type: {}", scalar.data_type())),
        })
    }
}

/// Truncate a tablet, removing all data but keeping the schema.
pub fn truncate_tablet(tablet: &Tablet) -> Result<(), String> {
    // Clear the memtable
    {
        let mut memtable = tablet.memtable.write();
        memtable.clear();
    }

    // Remove all rowsets from memory
    {
        let mut rowsets = tablet.rowsets.write();
        rowsets.clear();
    }

    // Delete all segment files on disk
    let tablet_dir = tablet.data_dir.join(format!("tablet_{}", tablet.tablet_id));
    if tablet_dir.exists() {
        let entries = std::fs::read_dir(&tablet_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

/// A tablet manages in-memory writes (memtable) and persistent rowsets.
/// Supports both JSON and RocksDB metadata backends.
pub struct Tablet {
    pub tablet_id: u64,
    pub schema: TabletSchema,
    pub max_version: AtomicU64,
    memtable: RwLock<MemTable>,
    rowsets: RwLock<Vec<Rowset>>,
    data_dir: PathBuf,
    next_segment_id: AtomicU64,
    next_rowset_id: AtomicU64,
    meta_backend: Option<Arc<dyn TabletMetaBackend>>,
}

/// Configuration for tablet loading.
#[derive(Clone)]
pub struct TabletConfig {
    pub data_dir: PathBuf,
    pub meta_backend: Option<Arc<dyn TabletMetaBackend>>,
}

impl TabletConfig {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir, meta_backend: None }
    }

    pub fn with_backend(mut self, backend: Arc<dyn TabletMetaBackend>) -> Self {
        self.meta_backend = Some(backend);
        self
    }

    pub fn with_json_backend(mut self) -> Self {
        self.meta_backend = Some(Arc::new(JsonTabletMetaBackend::new(self.data_dir.clone())));
        self
    }
}

impl Tablet {
    /// Create a new tablet with the given configuration.
    pub fn new(tablet_id: u64, schema: TabletSchema, config: TabletConfig) -> Self {
        let memtable_capacity = 64 * 1024 * 1024; // 64MB default
        Self {
            tablet_id,
            schema: schema.clone(),
            max_version: AtomicU64::new(0),
            memtable: RwLock::new(MemTable::new(memtable_capacity, schema)),
            rowsets: RwLock::new(Vec::new()),
            data_dir: config.data_dir,
            next_segment_id: AtomicU64::new(0),
            next_rowset_id: AtomicU64::new(0),
            meta_backend: config.meta_backend,
        }
    }

    /// Create a new tablet with legacy parameters (no backend).
    pub fn new_legacy(tablet_id: u64, schema: TabletSchema, data_dir: PathBuf) -> Self {
        Self::new(tablet_id, schema, TabletConfig::new(data_dir))
    }

    /// Load an existing tablet from disk using the legacy JSON file format.
    /// Returns the loaded Tablet or an error if the tablet directory doesn't exist.
    pub fn load_from_disk(tablet_id: u64, schema: TabletSchema, data_dir: PathBuf) -> Result<Self, String> {
        Self::load(tablet_id, schema, TabletConfig::new(data_dir))
    }

    /// Load a tablet using the configured backend.
    /// Supports both JSON and RocksDB backends.
    pub fn load(tablet_id: u64, schema: TabletSchema, config: TabletConfig) -> Result<Self, String> {
        let tablet = Self::new(tablet_id, schema.clone(), config.clone());

        let tablet_dir = config.data_dir.join(format!("tablet_{}", tablet_id));
        if !tablet_dir.exists() {
            return Err(format!("Tablet directory not found: {:?}", tablet_dir));
        }

        let mut max_version: u64 = 0;
        let mut max_segment_id: u64 = 0;
        let mut max_rowset_id: u64 = 0;

        // Try to load from backend if available
        if let Some(backend) = &tablet.meta_backend {
            // Load rowsets from backend
            let rowset_ids = backend.list_rowsets(tablet_id)
                .map_err(|e| format!("Failed to list rowsets: {}", e))?;

            for rowset_id in rowset_ids {
                match backend.load_rowset(tablet_id, rowset_id) {
                    Ok(Some((meta, segments))) => {
                        // Track segment IDs before moving segments into rowset
                        for seg in &segments {
                            if seg.segment_id > max_segment_id {
                                max_segment_id = seg.segment_id;
                            }
                        }

                        let mut rowset = Rowset::with_segments(meta.clone(), segments);
                        rowset.commit();
                        tablet.rowsets.write().push(rowset);

                        if meta.version > max_version {
                            max_version = meta.version;
                        }
                        if meta.rowset_id > max_rowset_id {
                            max_rowset_id = meta.rowset_id;
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("Rowset {} not found in backend", rowset_id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load rowset {} from backend: {}", rowset_id, e);
                    }
                }
            }

            tracing::info!(
                "Loaded tablet {} from backend: {} rowsets, max_version={}",
                tablet_id,
                tablet.rowsets.read().len(),
                max_version
            );
        } else {
            // Fall back to legacy JSON file loading
            let entries = std::fs::read_dir(&tablet_dir)
                .map_err(|e| format!("Failed to read tablet directory: {}", e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let path = entry.path();

                // Load rowset metadata files
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if file_name.starts_with("rowset_") {
                            match Rowset::load_meta(&path) {
                                Ok((meta, segments)) => {
                                    let mut rowset = Rowset::with_segments(meta.clone(), segments);
                                    rowset.commit();
                                    tablet.rowsets.write().push(rowset);

                                    if meta.version > max_version {
                                        max_version = meta.version;
                                    }
                                    if meta.rowset_id > max_rowset_id {
                                        max_rowset_id = meta.rowset_id;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to load rowset meta {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }

                // Track max segment ID for .dat files
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("dat") {
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if file_name.starts_with("seg_") {
                            if let Ok(seg_id) = file_name[4..].parse::<u64>() {
                                if seg_id > max_segment_id {
                                    max_segment_id = seg_id;
                                }
                            }
                        }
                    }
                }
            }

            // If no rowsets found from JSON files, scan for Parquet files directly
            #[cfg(feature = "parquet-storage")]
            if tablet.rowsets.read().is_empty() {
                use crate::segment::is_parquet_file;
                let entries = std::fs::read_dir(&tablet_dir)
                    .map_err(|e| format!("Failed to read tablet directory for Parquet scan: {}", e))?;

                for entry in entries {
                    let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                    let path = entry.path();

                    if path.is_file() && is_parquet_file(&path) {
                        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                            if file_name.starts_with("seg_") {
                                if let Ok(seg_id) = file_name[4..].parse::<u64>() {
                                    // Get file size
                                    let size = std::fs::metadata(&path)
                                        .map(|m| m.len())
                                        .unwrap_or(0);

                                    // Read Parquet metadata to get row count
                                    let num_rows = crate::segment::read_parquet_meta(&path)
                                        .map(|m| m.num_rows)
                                        .unwrap_or(0);

                                    // Create a rowset for this Parquet file
                                    let rowset_id = max_rowset_id + 1;
                                    max_rowset_id = rowset_id;

                                    if seg_id > max_segment_id {
                                        max_segment_id = seg_id;
                                    }

                                    let rowset_meta = RowsetMeta {
                                        rowset_id,
                                        tablet_id,
                                        txn_id: 0,
                                        version: max_version + 1,
                                        num_rows,
                                        data_size: size,
                                        num_segments: 1,
                                        empty: false,
                                        packed_data_size: size,
                                        index_size: 0,
                                    };

                                    let seg_ref = SegmentRef {
                                        segment_id: seg_id,
                                        path: path.to_string_lossy().to_string(),
                                        num_rows,
                                        size,
                                    };

                                    let mut rowset = Rowset::with_segments(rowset_meta, vec![seg_ref]);
                                    rowset.commit();
                                    tablet.rowsets.write().push(rowset);

                                    max_version += 1;

                                    tracing::debug!(
                                        "Recovered Parquet segment {} for tablet {}: {} rows, {} bytes",
                                        seg_id, tablet_id, num_rows, size
                                    );
                                }
                            }
                        }
                    }
                }
            }

            tracing::info!(
                "Loaded tablet {} from JSON files: {} rowsets, max_version={}",
                tablet_id,
                tablet.rowsets.read().len(),
                max_version
            );
        }

        // Update atomic counters
        tablet.max_version.store(max_version, Ordering::SeqCst);
        tablet.next_segment_id.store(max_segment_id + 1, Ordering::SeqCst);
        tablet.next_rowset_id.store(max_rowset_id + 1, Ordering::SeqCst);

        Ok(tablet)
    }

    /// Get the metadata backend if configured.
    pub fn meta_backend(&self) -> Option<&Arc<dyn TabletMetaBackend>> {
        self.meta_backend.as_ref()
    }

    /// Set the metadata backend.
    pub fn set_meta_backend(&mut self, backend: Arc<dyn TabletMetaBackend>) {
        self.meta_backend = Some(backend);
    }

    /// Write a block of rows into the memtable.
    pub fn write(&self, block: &Block) -> Result<(), String> {
        // Find the key column index (first column marked as key)
        let key_col_idx = self.schema
            .columns
            .iter()
            .position(|c| c.is_key)
            .unwrap_or(0);

        let mut memtable = self.memtable.write();
        memtable.insert(block, key_col_idx)?;

        if memtable.should_flush() {
            drop(memtable);
            // Auto-flush when memtable is full
            self.flush()?;
        }
        Ok(())
    }

    /// Delete rows from the tablet matching the given predicates.
    pub fn delete(&self, predicates: &[ColumnPredicate]) -> Result<usize, String> {
        let key_col_idx = self.schema
            .columns
            .iter()
            .position(|c| c.is_key)
            .unwrap_or(0);

        // Read all data
        let block = self.read(None, &[])?;

        let mut memtable = self.memtable.write();
        let deleted = memtable.delete(&block, key_col_idx, predicates)?;
        Ok(deleted)
    }

    /// Flush the current memtable to a new segment file on disk.
    pub fn flush(&self) -> Result<(), String> {
        let mut memtable = self.memtable.write();
        if memtable.is_empty() {
            return Ok(());
        }

        let schema = self.schema.to_schema();
        let block = memtable.to_block(&schema);
        let version = self.max_version.fetch_add(1, Ordering::SeqCst);

        // Get IDs - use backend if available for atomic counters
        let (seg_id, rowset_id) = if let Some(backend) = &self.meta_backend {
            let seg_id = backend.next_segment_id(self.tablet_id)
                .map_err(|e| e.to_string())?;
            let rowset_id = backend.next_rowset_id(self.tablet_id)
                .map_err(|e| e.to_string())?;
            (seg_id, rowset_id)
        } else {
            let seg_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst);
            let rowset_id = self.next_rowset_id.fetch_add(1, Ordering::SeqCst);
            (seg_id, rowset_id)
        };

        // Ensure tablet data directory exists
        let tablet_dir = self.data_dir.join(format!("tablet_{}", self.tablet_id));
        std::fs::create_dir_all(&tablet_dir)
            .map_err(|e| format!("Create tablet dir: {}", e))?;

        let seg_path = tablet_dir.join(format!("seg_{}.dat", seg_id));
        let file_size = SegmentWriter::write_segment(&seg_path, &block)?;

        let seg_ref = SegmentRef {
            segment_id: seg_id,
            path: seg_path.to_string_lossy().to_string(),
            num_rows: block.num_rows() as u64,
            size: file_size,
        };

        let meta = RowsetMeta::new(rowset_id, self.tablet_id, version);
        let mut rowset = Rowset::new(meta);
        rowset.add_segment(seg_ref);
        rowset.commit();

        // Save rowset metadata - use backend if available
        if let Some(backend) = &self.meta_backend {
            backend.save_rowset(self.tablet_id, rowset_id, &rowset.meta, &rowset.segments)
                .map_err(|e| format!("Save rowset to backend: {}", e))?;
        } else {
            // Legacy JSON file saving
            let meta_path = tablet_dir.join(format!("rowset_{}.json", rowset_id));
            rowset.save_meta(&meta_path)?;
        }

        self.rowsets.write().push(rowset);
        memtable.clear();

        tracing::info!(
            "Flushed tablet {}: {} rows, {} bytes to segment {}",
            self.tablet_id,
            block.num_rows(),
            file_size,
            seg_id
        );

        Ok(())
    }

    /// Read all data from the tablet, applying projection and predicates.
    pub fn read(
        &self,
        projection: Option<&[usize]>,
        predicates: &[ColumnPredicate],
    ) -> Result<Block, String> {
        let schema = self.schema.to_schema();

        // Read from memtable first
        let mut blocks = Vec::new();
        {
            let memtable = self.memtable.read();
            if !memtable.is_empty() {
                let block = memtable.to_block(&schema);
                blocks.push(block);
            }
        }

        // Read from all committed rowsets
        let rowsets = self.rowsets.read();
        for rowset in rowsets.iter() {
            for seg_ref in &rowset.segments {
                let path = Path::new(&seg_ref.path);
                if path.exists() {
                    match SegmentReader::scan_segment(path, projection, predicates) {
                        Ok(block) => {
                            if !block.is_empty() {
                                blocks.push(block);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Error reading segment {}: {}", seg_ref.path, e);
                        }
                    }
                }
            }
        }

        // Combine all blocks
        if blocks.is_empty() {
            let proj_schema = if let Some(proj) = projection {
                schema.project(proj)
            } else {
                schema
            };
            return Ok(Block::empty(proj_schema));
        }

        // Apply predicates to memtable blocks (segments already filtered during scan)
        let mut filtered_blocks = Vec::new();
        for (i, block) in blocks.into_iter().enumerate() {
            // The first block is from memtable (index 0 if memtable had data)
            // Subsequent blocks are already filtered by SegmentReader
            if i == 0 && !predicates.is_empty() && !rowsets.is_empty() {
                // This might be a memtable block, need to apply predicates
                let selection = crate::index::apply_predicates_to_block(&block, predicates);
                filtered_blocks.push(block.filter(&selection));
            } else if i == 0 && !predicates.is_empty() {
                // Only memtable, need to filter
                let selection = crate::index::apply_predicates_to_block(&block, predicates);
                filtered_blocks.push(block.filter(&selection));
            } else {
                filtered_blocks.push(block);
            }
        }

        // Project columns if needed
        let projected: Vec<Block> = if let Some(proj) = projection {
            filtered_blocks.into_iter().map(|b| b.project(proj)).collect()
        } else {
            filtered_blocks
        };

        // Concatenate
        match Block::concat(&projected) {
            Some(block) => Ok(block),
            None => {
                let proj_schema = if let Some(proj) = projection {
                    schema.project(proj)
                } else {
                    schema
                };
                Ok(Block::empty(proj_schema))
            }
        }
    }

    pub fn add_rowset(&self, rowset: Rowset) {
        self.rowsets.write().push(rowset);
        self.max_version.fetch_add(1, Ordering::SeqCst);
    }

    pub fn rowset_count(&self) -> usize {
        self.rowsets.read().len()
    }

    pub fn max_version(&self) -> u64 {
        self.max_version.load(Ordering::Relaxed)
    }

    /// Get committed rowsets for compaction.
    pub fn committed_rowsets(&self) -> Vec<Rowset> {
        self.rowsets
            .read()
            .iter()
            .filter(|r| r.state == RowsetState::Committed)
            .cloned()
            .collect()
    }

    /// Remove rowsets by their IDs (after compaction produces new ones).
    pub fn remove_rowsets(&self, rowset_ids: &[u64]) {
        let mut rowsets = self.rowsets.write();
        rowsets.retain(|r| !rowset_ids.contains(&r.meta.rowset_id));
    }

    /// Get the data directory for this tablet.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get memtable row count.
    pub fn memtable_num_rows(&self) -> usize {
        self.memtable.read().num_rows()
    }

    /// Get memtable memory size.
    pub fn memtable_memory_size(&self) -> u64 {
        self.memtable.read().memory_size()
    }

    /// Check if memtable is empty.
    pub fn memtable_is_empty(&self) -> bool {
        self.memtable.read().is_empty()
    }

    /// Check if memtable should flush.
    pub fn memtable_should_flush(&self) -> bool {
        self.memtable.read().should_flush()
    }

    /// Clear memtable.
    pub fn memtable_clear(&self) {
        self.memtable.write().clear()
    }

    /// Convert memtable to block.
    pub fn memtable_to_block(&self, schema: &Schema) -> Block {
        self.memtable.read().to_block(schema)
    }

    /// Get next segment ID (atomically increment and return).
    fn next_segment_id(&self) -> u64 {
        if let Some(backend) = &self.meta_backend {
            backend.next_segment_id(self.tablet_id).unwrap_or_else(|_| {
                self.next_segment_id.fetch_add(1, Ordering::SeqCst)
            })
        } else {
            self.next_segment_id.fetch_add(1, Ordering::SeqCst)
        }
    }

    /// Get next rowset ID (atomically increment and return).
    fn next_rowset_id(&self) -> u64 {
        if let Some(backend) = &self.meta_backend {
            backend.next_rowset_id(self.tablet_id).unwrap_or_else(|_| {
                self.next_rowset_id.fetch_add(1, Ordering::SeqCst)
            })
        } else {
            self.next_rowset_id.fetch_add(1, Ordering::SeqCst)
        }
    }

    // =========================================================================
    // Arrow/Parquet interfaces (when parquet-storage feature is enabled)
    // =========================================================================

    /// Read data as Arrow RecordBatch.
    #[cfg(feature = "parquet-storage")]
    pub fn read_arrow(
        &self,
        projection: Option<&[String]>,
        predicates: &[crate::segment::ReadPredicate],
        limit: Option<usize>,
    ) -> Result<arrow_array::RecordBatch, String> {
        use arrow_array::RecordBatch;
        use arrow_schema::{Schema as ArrowSchema, Field, DataType as ArrowDataType};
        use crate::segment::{read_parquet_segment, ParquetReadOptions, is_parquet_file};

        // Build Arrow schema from tablet schema
        let arrow_fields: Vec<Field> = self.schema.columns.iter()
            .map(|c| {
                Field::new(
                    &c.name,
                    to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        let arrow_schema = Arc::new(ArrowSchema::new(arrow_fields));

        // Read from Parquet segments
        let mut batches: Vec<RecordBatch> = Vec::new();

        // Read from memtable (convert Block to RecordBatch)
        {
            let memtable = self.memtable.read();
            if !memtable.is_empty() {
                let schema = self.schema.to_schema();
                let block = memtable.to_block(&schema);
                // Convert Block to RecordBatch
                if let Ok(batch) = block_to_record_batch(&block) {
                    batches.push(batch);
                }
            }
        }

        // Read from rowsets
        let rowsets = self.rowsets.read();
        for rowset in rowsets.iter() {
            for seg_ref in &rowset.segments {
                let path = Path::new(&seg_ref.path);
                if path.exists() {
                    // Check if it's a Parquet file
                    if is_parquet_file(path) {
                        let options = ParquetReadOptions {
                            projection: projection.map(|p| p.to_vec()),
                            predicates: predicates.to_vec(),
                            limit,
                        };
                        match read_parquet_segment(path, &options) {
                            Ok(batch) => {
                                if batch.num_rows() > 0 {
                                    batches.push(batch);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Error reading Parquet segment {}: {}", seg_ref.path, e);
                            }
                        }
                    } else {
                        // Legacy .dat format - read as Block and convert
                        match SegmentReader::scan_segment(path, None, &[]) {
                            Ok(block) => {
                                if !block.is_empty() {
                                    if let Ok(batch) = block_to_record_batch(&block) {
                                        batches.push(batch);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Error reading segment {}: {}", seg_ref.path, e);
                            }
                        }
                    }
                }
            }
        }

        // Merge batches
        if batches.is_empty() {
            // Return empty batch with correct schema
            let empty_columns: Vec<Arc<dyn arrow_array::Array>> = arrow_schema.fields()
                .iter()
                .map(|f| arrow_array::new_empty_array(f.data_type()))
                .collect();
            return RecordBatch::try_new(arrow_schema, empty_columns)
                .map_err(|e| e.to_string());
        }

        if batches.len() == 1 {
            // Apply projection if needed
            let batch = &batches[0];
            if let Some(proj) = projection {
                let indices: Vec<usize> = proj.iter()
                    .filter_map(|col| batch.schema().index_of(col).ok())
                    .collect();
                return batch.project(&indices)
                    .map_err(|e| e.to_string());
            }
            return Ok(batch.clone());
        }

        // Concatenate all batches
        let merged = arrow_select::concat::concat_batches(&arrow_schema, &batches)
            .map_err(|e| e.to_string())?;

        // Apply projection
        if let Some(proj) = projection {
            let indices: Vec<usize> = proj.iter()
                .filter_map(|col| merged.schema().index_of(col).ok())
                .collect();
            return merged.project(&indices)
                .map_err(|e| e.to_string());
        }

        // Apply limit
        if let Some(limit) = limit {
            if merged.num_rows() > limit {
                return Ok(merged.slice(0, limit));
            }
        }

        Ok(merged)
    }

    /// Write an Arrow RecordBatch to the tablet.
    #[cfg(feature = "parquet-storage")]
    pub fn write_arrow(&self, batch: &arrow_array::RecordBatch) -> Result<(), String> {
        // Convert RecordBatch to Block and use existing write logic
        let block = record_batch_to_block(batch)?;
        self.write(&block)
    }

    /// Flush memtable to Parquet file.
    #[cfg(feature = "parquet-storage")]
    pub fn flush_parquet(&self) -> Result<(), String> {
        use crate::segment::{write_parquet_segment, ParquetWriterConfig};

        // First check if memtable is empty and get the data
        let (block, batch) = {
            let memtable = self.memtable.read();
            if memtable.is_empty() {
                return Ok(());
            }

            let schema = self.schema.to_schema();
            let block = memtable.to_block(&schema);

            // Convert to RecordBatch
            let batch = block_to_record_batch(&block)?;
            (block, batch)
        };

        // Get next segment ID
        let seg_id = self.next_segment_id();

        // Write Parquet file
        let tablet_dir = self.data_dir.join(format!("tablet_{}", self.tablet_id));
        std::fs::create_dir_all(&tablet_dir)
            .map_err(|e| format!("Create tablet dir: {}", e))?;
        let seg_path = tablet_dir.join(format!("seg_{}.parquet", seg_id));

        let config = ParquetWriterConfig::default();
        let meta = write_parquet_segment(&seg_path, &batch, &config)
            .map_err(|e| e.to_string())?;

        // Create rowset
        let rowset_id = self.next_rowset_id();
        let rowset_meta = RowsetMeta {
            rowset_id,
            tablet_id: self.tablet_id,
            txn_id: 0,
            version: self.max_version.load(Ordering::SeqCst),
            num_rows: meta.num_rows,
            data_size: meta.size,
            num_segments: 1,
            empty: false,
            packed_data_size: meta.size,
            index_size: 0,
        };

        let seg_ref = SegmentRef {
            segment_id: seg_id,
            path: seg_path.to_string_lossy().to_string(),
            num_rows: meta.num_rows,
            size: meta.size,
        };

        let rowset = Rowset::with_segments(rowset_meta, vec![seg_ref]);

        // Save to backend if available
        if let Some(backend) = &self.meta_backend {
            backend.save_rowset(self.tablet_id, rowset_id, &rowset.meta, &rowset.segments)
                .map_err(|e| format!("Save rowset to backend: {}", e))?;
        }

        self.rowsets.write().push(rowset);

        // Now clear the memtable with a write lock
        self.memtable.write().clear();

        tracing::info!(
            "Flushed tablet {} to Parquet: {} rows, {} bytes",
            self.tablet_id,
            meta.num_rows,
            meta.size
        );

        Ok(())
    }
}

/// Convert RorisDB DataType to Arrow DataType.
#[cfg(feature = "parquet-storage")]
fn to_arrow_data_type(dt: &types::DataType) -> arrow_schema::DataType {
    use arrow_schema::DataType as ArrowDT;
    match dt {
        types::DataType::Boolean => ArrowDT::Boolean,
        types::DataType::Int8 => ArrowDT::Int8,
        types::DataType::Int16 => ArrowDT::Int16,
        types::DataType::Int32 => ArrowDT::Int32,
        types::DataType::Int64 => ArrowDT::Int64,
        types::DataType::Int128 => ArrowDT::Decimal128(38, 0),
        types::DataType::Float32 => ArrowDT::Float32,
        types::DataType::Float64 => ArrowDT::Float64,
        types::DataType::String => ArrowDT::Utf8,
        types::DataType::Date => ArrowDT::Date32,
        types::DataType::DateTime => ArrowDT::Timestamp(arrow_schema::TimeUnit::Second, None),
        _ => ArrowDT::Null,
    }
}

/// Convert Block to Arrow RecordBatch.
#[cfg(feature = "parquet-storage")]
fn block_to_record_batch(block: &Block) -> Result<arrow_array::RecordBatch, String> {
    use arrow_array::{Array, Int8Array, Int16Array, Int32Array, Int64Array, Float32Array, Float64Array, StringArray, Date32Array, TimestampSecondArray, BooleanArray, RecordBatch};
    use arrow_schema::{Schema as ArrowSchema, Field};

    // Build Arrow schema
    let fields: Vec<Field> = block.schema().fields().iter()
        .map(|f| Field::new(&f.name, to_arrow_data_type(&f.data_type), f.nullable))
        .collect();
    let schema = Arc::new(ArrowSchema::new(fields));

    // Convert columns
    let columns: Vec<Arc<dyn Array>> = (0..block.num_columns())
        .map(|col_idx| {
            let col = block.column(col_idx).unwrap();
            let arr: Arc<dyn Array> = match col {
                types::Vector::Int8(v) => {
                    let data: Vec<Option<i8>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Int8Array::from(data))
                }
                types::Vector::Int16(v) => {
                    let data: Vec<Option<i16>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Int16Array::from(data))
                }
                types::Vector::Int32(v) => {
                    let data: Vec<Option<i32>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Int32Array::from(data))
                }
                types::Vector::Int64(v) => {
                    let data: Vec<Option<i64>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Int64Array::from(data))
                }
                types::Vector::Float32(v) => {
                    let data: Vec<Option<f32>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Float32Array::from(data))
                }
                types::Vector::Float64(v) => {
                    let data: Vec<Option<f64>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Float64Array::from(data))
                }
                types::Vector::String(v) => {
                    let data: Vec<Option<String>> = (0..v.len())
                        .map(|i| v.get(i).map(|s| s.to_string()))
                        .collect();
                    Arc::new(StringArray::from(data))
                }
                types::Vector::Date(v) => {
                    let data: Vec<Option<i32>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(Date32Array::from(data))
                }
                types::Vector::DateTime(v) => {
                    let data: Vec<Option<i64>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(TimestampSecondArray::from(data))
                }
                types::Vector::Boolean(v) => {
                    let data: Vec<Option<bool>> = (0..v.len()).map(|i| v.get(i)).collect();
                    Arc::new(BooleanArray::from(data))
                }
                types::Vector::Null(v) => {
                    arrow_array::new_null_array(&to_arrow_data_type(&col.data_type()), col.len())
                }
                _ => {
                    tracing::warn!("Unsupported vector type for Arrow conversion: {:?}", col);
                    arrow_array::new_null_array(&to_arrow_data_type(&col.data_type()), col.len())
                }
            };
            arr
        })
        .collect();

    RecordBatch::try_new(schema, columns)
        .map_err(|e| e.to_string())
}

/// Convert Arrow RecordBatch to Block.
#[cfg(feature = "parquet-storage")]
fn record_batch_to_block(batch: &arrow_array::RecordBatch) -> Result<Block, String> {
    use types::{Schema, Field, DataType, Vector};
    use arrow_array::{Int64Array, Float64Array, StringArray, Date32Array, BooleanArray};

    // Build RorisDB schema
    let fields: Vec<Field> = batch.schema().fields().iter()
        .map(|f| {
            let dt = match f.data_type() {
                arrow_schema::DataType::Boolean => DataType::Boolean,
                arrow_schema::DataType::Int64 => DataType::Int64,
                arrow_schema::DataType::Float64 => DataType::Float64,
                arrow_schema::DataType::Utf8 => DataType::String,
                arrow_schema::DataType::Date32 => DataType::Date,
                _ => DataType::String,
            };
            Field::new(f.name(), dt, f.is_nullable())
        })
        .collect();
    let schema = Schema::new(fields);

    // Convert columns
    let columns: Vec<Vector> = batch.columns().iter()
        .map(|col| {
            match col.data_type() {
                arrow_schema::DataType::Int64 => {
                    let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                    let data: Vec<Option<i64>> = arr.iter().collect();
                    Vector::Int64(types::vector::Int64Vector::from_nullable_vec(data))
                }
                arrow_schema::DataType::Float64 => {
                    let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
                    let data: Vec<Option<f64>> = arr.iter().collect();
                    Vector::Float64(types::vector::Float64Vector::from_nullable_vec(data))
                }
                arrow_schema::DataType::Utf8 => {
                    let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
                    let data: Vec<Option<String>> = arr.iter()
                        .map(|s| s.map(|s| s.to_string()))
                        .collect();
                    Vector::String(types::vector::StringVector::from_option_vec(data))
                }
                arrow_schema::DataType::Date32 => {
                    let arr = col.as_any().downcast_ref::<Date32Array>().unwrap();
                    let data: Vec<Option<i32>> = arr.iter().collect();
                    Vector::Date(types::vector::DateVector::from_nullable_vec(data))
                }
                arrow_schema::DataType::Boolean => {
                    let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                    let data: Vec<Option<bool>> = arr.iter().collect();
                    Vector::Boolean(types::vector::BooleanVector::from_nullable_vec(data))
                }
                _ => Vector::Null(types::vector::NullVector::new(col.len())),
            }
        })
        .collect();

    Ok(Block::new(schema, columns))
}

/// Migrate existing JSON metadata to RocksDB backend.
/// This function reads all tablet metadata from JSON files and writes it to RocksDB.
pub fn migrate_tablet_to_rocks(
    tablet_id: u64,
    data_dir: PathBuf,
    rocks_backend: Arc<dyn TabletMetaBackend>,
) -> Result<(), TabletMetaError> {
    let json_backend = JsonTabletMetaBackend::new(data_dir.clone());

    // Load schema from JSON if it exists
    if let Some(schema) = json_backend.load_schema(tablet_id)? {
        rocks_backend.save_schema(tablet_id, &schema)?;
        tracing::info!("Migrated schema for tablet {}", tablet_id);
    }

    // Load all rowsets from JSON
    let rowset_ids = json_backend.list_rowsets(tablet_id)?;
    let mut max_rowset_id = 0u64;
    let mut max_segment_id = 0u64;

    for rowset_id in rowset_ids {
        if let Some((meta, segments)) = json_backend.load_rowset(tablet_id, rowset_id)? {
            rocks_backend.save_rowset(tablet_id, rowset_id, &meta, &segments)?;

            if rowset_id > max_rowset_id {
                max_rowset_id = rowset_id;
            }
            for seg in &segments {
                if seg.segment_id > max_segment_id {
                    max_segment_id = seg.segment_id;
                }
            }
            tracing::info!("Migrated rowset {} for tablet {}", rowset_id, tablet_id);
        }
    }

    // Set counters to next values
    rocks_backend.set_next_rowset_id(tablet_id, max_rowset_id + 1)?;
    rocks_backend.set_next_segment_id(tablet_id, max_segment_id + 1)?;

    rocks_backend.flush()?;
    tracing::info!("Migration complete for tablet {}", tablet_id);

    Ok(())
}

/// Migrate all tablets in a data directory to RocksDB.
#[cfg(feature = "rocksdb")]
pub fn migrate_all_tablets_to_rocks(
    data_dir: PathBuf,
    rocks_backend: Arc<RocksTabletMetaBackend>,
) -> Result<Vec<u64>, TabletMetaError> {
    let tablet_ids = discover_tablet_ids(&data_dir)?;
    let backend: Arc<dyn TabletMetaBackend> = rocks_backend;

    for tablet_id in &tablet_ids {
        migrate_tablet_to_rocks(*tablet_id, data_dir.clone(), backend.clone())?;
    }

    Ok(tablet_ids)
}

/// Discover all tablet IDs in a data directory.
pub fn discover_tablet_ids(data_dir: &Path) -> Result<Vec<u64>, TabletMetaError> {
    let mut tablet_ids = Vec::new();

    if !data_dir.exists() {
        return Ok(tablet_ids);
    }

    for entry in std::fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if dir_name.starts_with("tablet_") {
                if let Ok(id) = dir_name[7..].parse::<u64>() {
                    tablet_ids.push(id);
                }
            }
        }
    }

    tablet_ids.sort();
    Ok(tablet_ids)
}

/// Estimate the memory size of a block.
#[allow(dead_code)]
fn estimate_block_size(block: &Block) -> u64 {
    let mut size = 0u64;
    for col in block.columns() {
        size += match col {
            types::Vector::Boolean(v) => v.len() as u64,
            types::Vector::Int8(v) => v.len() as u64,
            types::Vector::Int16(v) => v.len() as u64 * 2,
            types::Vector::Int32(v) => v.len() as u64 * 4,
            types::Vector::Int64(v) => v.len() as u64 * 8,
            types::Vector::Int128(v) => v.len() as u64 * 16,
            types::Vector::Float32(v) => v.len() as u64 * 4,
            types::Vector::Float64(v) => v.len() as u64 * 8,
            types::Vector::String(v) => {
                // Rough estimate
                v.len() as u64 * 32
            }
            types::Vector::Date(v) => v.len() as u64 * 4,
            types::Vector::DateTime(v) => v.len() as u64 * 8,
            types::Vector::Json(v) => v.len() as u64 * 64,
            types::Vector::Null(v) => v.len() as u64,
            types::Vector::Float32Array(v) => {
                let dim = if v.len() > 0 { v.data()[0].len() } else { 0 };
                v.len() as u64 * dim as u64 * 4
            }
        };
    }
    size
}
