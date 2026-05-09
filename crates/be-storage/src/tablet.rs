use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
        }
    }

    pub fn insert(&mut self, block: &Block, key_column_idx: usize) -> Result<(), String> {
        for row_idx in 0..block.num_rows() {
            let key = self.extract_key(block, row_idx, key_column_idx)?;
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
        let mut deleted_count = 0;

        for row_idx in 0..block.num_rows() {
            if selection.get(row_idx) {
                let key = self.extract_key(block, row_idx, key_column_idx)?;
                if self.rows.remove(&key).is_some() {
                    deleted_count += 1;
                }
            }
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
pub struct Tablet {
    pub tablet_id: u64,
    pub schema: TabletSchema,
    pub max_version: AtomicU64,
    memtable: RwLock<MemTable>,
    rowsets: RwLock<Vec<Rowset>>,
    data_dir: PathBuf,
    next_segment_id: AtomicU64,
    next_rowset_id: AtomicU64,
}

impl Tablet {
    pub fn new(tablet_id: u64, schema: TabletSchema, data_dir: PathBuf) -> Self {
        let memtable_capacity = 64 * 1024 * 1024; // 64MB default
        Self {
            tablet_id,
            schema: schema.clone(),
            max_version: AtomicU64::new(0),
            memtable: RwLock::new(MemTable::new(memtable_capacity, schema)),
            rowsets: RwLock::new(Vec::new()),
            data_dir,
            next_segment_id: AtomicU64::new(0),
            next_rowset_id: AtomicU64::new(0),
        }
    }

    /// Load an existing tablet from disk by reading rowset metadata files.
    /// Returns the loaded Tablet or an error if the tablet directory doesn't exist.
    pub fn load_from_disk(tablet_id: u64, schema: TabletSchema, data_dir: PathBuf) -> Result<Self, String> {
        let tablet = Self::new(tablet_id, schema, data_dir.clone());
        
        let tablet_dir = data_dir.join(format!("tablet_{}", tablet_id));
        if !tablet_dir.exists() {
            return Err(format!("Tablet directory not found: {:?}", tablet_dir));
        }

        // Read all rowset metadata files
        let entries = std::fs::read_dir(&tablet_dir)
            .map_err(|e| format!("Failed to read tablet directory: {}", e))?;

        let mut max_version: u64 = 0;
        let mut max_segment_id: u64 = 0;
        let mut max_rowset_id: u64 = 0;

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
            
            // Track max segment ID
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

        // Update atomic counters
        tablet.max_version.store(max_version, Ordering::SeqCst);
        tablet.next_segment_id.store(max_segment_id + 1, Ordering::SeqCst);
        tablet.next_rowset_id.store(max_rowset_id + 1, Ordering::SeqCst);

        tracing::info!(
            "Loaded tablet {} from disk: {} rowsets, max_version={}",
            tablet_id,
            tablet.rowsets.read().len(),
            max_version
        );

        Ok(tablet)
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

        let seg_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst);
        let rowset_id = self.next_rowset_id.fetch_add(1, Ordering::SeqCst);

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

        // Save rowset metadata
        let meta_path = tablet_dir.join(format!("rowset_{}.json", rowset_id));
        rowset.save_meta(&meta_path)?;

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
