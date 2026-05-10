//! Write-Ahead Log (WAL) for RorisDB storage engine.
//!
//! ## Design
//!
//! The WAL ensures durability: every write is appended to the WAL and fsync'd
//! *before* being inserted into the in-memory MemTable. On flush, a FlushMarker
//! is written and the WAL is truncated. On crash recovery, entries after the
//! last FlushMarker are replayed to reconstruct the MemTable.
//!
//! ### Binary Format
//!
//! ```text
//! ┌─────────────── Entry Header (13 bytes) ───────────────┐
//! │ entry_type: u8     (0x01=Insert, 0x02=FlushMarker)     │
//! │ body_len:   u32    (payload length in LE bytes)        │
//! │ crc32:      u32    (CRC32 of body)                     │
//! │ checksum:   u32    (CRC32 of entire entry including header)  │
//! ├─────────────── Body ──────────────────────────────────┤
//! │ Insert: schema_json + columnar binary data             │
//! │ FlushMarker: u64 version number                        │
//! └───────────────────────────────────────────────────────┘
//! ```
//!
//! ### Lifecycle
//!
//! 1. **Create**: `WalWriter::open()` on Tablet creation
//! 2. **Write**: append Insert entries on each `Tablet::write()`
//! 3. **Flush**: write FlushMarker, truncate WAL on `Tablet::flush()`
//! 4. **Crash Recovery**: `WalReader::replay_and_recover()` scans WAL, replays
//!    unflushed entries, then truncates

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use types::{Block, DataType, Field, Schema, Vector};

const WAL_FILE_NAME: &str = "wal.dat";

// Entry type constants
const ENTRY_INSERT: u8 = 0x01;
const ENTRY_FLUSH_MARKER: u8 = 0x02;

// Header size: type(1) + body_len(4) + crc32(4) + checksum(4)
const ENTRY_HEADER_SIZE: usize = 13;

// ---------------------------------------------------------------------------
// CRC32 (IEEE 802.3, reflected)
// ---------------------------------------------------------------------------

fn crc32_table() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (i, entry) in table.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = 0xEDB88320u32 ^ (crc >> 1);
                } else {
                    crc >>= 1;
                }
            }
            *entry = crc;
        }
        table
    })
}

/// Compute standard CRC32 of arbitrary bytes.
pub fn crc32(data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    !crc
}

// ---------------------------------------------------------------------------
// Block serialization helpers (mirrors SegmentWriter but simpler)
// ---------------------------------------------------------------------------

/// Serializable schema descriptor for WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalSchemaDesc {
    fields: Vec<WalFieldDesc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalFieldDesc {
    name: String,
    data_type: DataType,
    nullable: bool,
}

impl From<&Schema> for WalSchemaDesc {
    fn from(s: &Schema) -> Self {
        Self {
            fields: s
                .fields()
                .iter()
                .map(|f| WalFieldDesc {
                    name: f.name.clone(),
                    data_type: f.data_type.clone(),
                    nullable: f.nullable,
                })
                .collect(),
        }
    }
}

impl WalSchemaDesc {
    fn to_schema(&self) -> Schema {
        let fields: Vec<Field> = self
            .fields
            .iter()
            .map(|f| Field::new(&f.name, f.data_type.clone(), f.nullable))
            .collect();
        Schema::new(fields)
    }
}

/// Serialize a Vector to raw bytes. Reuses the logic from SegmentWriter.
/// See `serialize_column` in `segment/writer.rs`.
pub fn serialize_column(column: &Vector) -> Vec<u8> {
    match column {
        Vector::Boolean(v) => {
            let mut buf = Vec::with_capacity(v.len());
            for i in 0..v.len() {
                buf.push(if v.get(i).unwrap_or(false) { 1u8 } else { 0u8 });
            }
            buf
        }
        Vector::Int8(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::Int16(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::Int32(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::Int64(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::Int128(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::Float32(v) => {
            let data = v.data();
            data.iter().flat_map(|f| f.to_le_bytes()).collect()
        }
        Vector::Float64(v) => {
            let data = v.data();
            data.iter().flat_map(|f| f.to_le_bytes()).collect()
        }
        Vector::Date(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::DateTime(v) => {
            let data = v.data();
            data.iter().flat_map(|n| n.to_le_bytes()).collect()
        }
        Vector::String(v) => {
            let len = v.len();
            let mut buf = Vec::new();
            buf.extend_from_slice(&(len as u32).to_le_bytes());
            for i in 0..len {
                if let Some(s) = v.get(i) {
                    let bytes = s.as_bytes();
                    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    buf.extend_from_slice(bytes);
                } else {
                    buf.extend_from_slice(&0u32.to_le_bytes());
                }
            }
            buf
        }
        Vector::Json(v) => {
            let len = v.len();
            let mut buf = Vec::new();
            buf.extend_from_slice(&(len as u32).to_le_bytes());
            for i in 0..len {
                if let Some(val) = v.get(i) {
                    let json_str =
                        serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string());
                    let bytes = json_str.as_bytes();
                    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    buf.extend_from_slice(bytes);
                } else {
                    buf.extend_from_slice(&0u32.to_le_bytes());
                }
            }
            buf
        }
        Vector::Null(v) => vec![0u8; v.len()],
        Vector::Float32Array(v) => {
            let len = v.len();
            let mut buf = Vec::new();
            buf.extend_from_slice(&(len as u32).to_le_bytes());
            for i in 0..len {
                if let Some(arr) = v.get(i) {
                    buf.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                    for f in arr {
                        buf.extend_from_slice(&f.to_le_bytes());
                    }
                } else {
                    buf.extend_from_slice(&0u32.to_le_bytes());
                }
            }
            buf
        }
    }
}

/// Serialize null bitmap of a Vector to bytes. Reuses logic from SegmentWriter.
pub fn serialize_null_bitmap(column: &Vector) -> Vec<u8> {
    let num_rows = column.len();
    let bitmap_words = num_rows.div_ceil(64);
    let mut bitmap = vec![0u64; bitmap_words];

    for i in 0..num_rows {
        let is_valid = match column {
            Vector::Boolean(v) => v.validity().is_valid(i),
            Vector::Int8(v) => v.validity().is_valid(i),
            Vector::Int16(v) => v.validity().is_valid(i),
            Vector::Int32(v) => v.validity().is_valid(i),
            Vector::Int64(v) => v.validity().is_valid(i),
            Vector::Int128(v) => v.validity().is_valid(i),
            Vector::Float32(v) => v.validity().is_valid(i),
            Vector::Float64(v) => v.validity().is_valid(i),
            Vector::Date(v) => v.validity().is_valid(i),
            Vector::DateTime(v) => v.validity().is_valid(i),
            Vector::String(v) => v.validity().is_valid(i),
            Vector::Json(v) => v.validity().is_valid(i),
            Vector::Null(_) => false,
            Vector::Float32Array(v) => v.validity().is_valid(i),
        };
        if is_valid {
            let word_idx = i / 64;
            let bit_idx = i % 64;
            bitmap[word_idx] |= 1u64 << bit_idx;
        }
    }

    bitmap.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn deserialize_null_bitmap(data: &[u8], num_rows: usize) -> types::Bitmap {
    let words: Vec<u64> = data
        .chunks(8)
        .map(|chunk| {
            let mut bytes = [0u8; 8];
            let len = chunk.len().min(8);
            bytes[..len].copy_from_slice(&chunk[..len]);
            u64::from_le_bytes(bytes)
        })
        .collect();

    let mut bm = types::Bitmap::with_capacity(num_rows);
    for i in 0..num_rows {
        let word_idx = i / 64;
        let bit_idx = i % 64;
        let is_valid = if word_idx < words.len() {
            (words[word_idx] >> bit_idx) & 1 == 1
        } else {
            false
        };
        bm.push(is_valid);
    }
    bm
}

fn deserialize_vector(data: &[u8], data_type: &DataType, null_bitmap: &types::Bitmap, num_rows: usize) -> Result<Vector, String> {
    match data_type {
        DataType::Boolean => {
            let mut v = types::vector::BooleanVector::new();
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < data.len() {
                    v.push(Some(data[i] != 0));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Boolean(v))
        }
        DataType::Int8 => {
            let mut v = types::vector::Int8Vector::new();
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < data.len() {
                    v.push(Some(data[i] as i8));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Int8(v))
        }
        DataType::Int16 => {
            let mut v = types::vector::Int16Vector::new();
            let needed = (num_rows * 2).min(data.len());
            let items = needed / 2;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 2;
                    let b = [data[off], data[off + 1]];
                    v.push(Some(i16::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Int16(v))
        }
        DataType::Int32 => {
            let mut v = types::vector::Int32Vector::new();
            let needed = (num_rows * 4).min(data.len());
            let items = needed / 4;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 4;
                    let b = [data[off], data[off + 1], data[off + 2], data[off + 3]];
                    v.push(Some(i32::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Int32(v))
        }
        DataType::Int64 | DataType::DateTime => {
            let mut v64 = types::vector::Int64Vector::new();
            let mut vdt = types::vector::DateTimeVector::new();
            let needed = (num_rows * 8).min(data.len());
            let items = needed / 8;
            let is_dt = matches!(data_type, DataType::DateTime);
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 8;
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&data[off..off + 8]);
                    let val = i64::from_le_bytes(b);
                    if is_dt {
                        vdt.push(Some(val));
                    } else {
                        v64.push(Some(val));
                    }
                } else {
                    if is_dt {
                        vdt.push(None);
                    } else {
                        v64.push(None);
                    }
                }
            }
            if is_dt { Ok(Vector::DateTime(vdt)) } else { Ok(Vector::Int64(v64)) }
        }
        DataType::Int128 => {
            let mut v = types::vector::Int128Vector::new();
            let needed = (num_rows * 16).min(data.len());
            let items = needed / 16;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 16;
                    let mut b = [0u8; 16];
                    b.copy_from_slice(&data[off..off + 16]);
                    v.push(Some(i128::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Int128(v))
        }
        DataType::Float32 => {
            let mut v = types::vector::Float32Vector::new();
            let needed = (num_rows * 4).min(data.len());
            let items = needed / 4;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 4;
                    let b = [data[off], data[off + 1], data[off + 2], data[off + 3]];
                    v.push(Some(f32::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Float32(v))
        }
        DataType::Float64 => {
            let mut v = types::vector::Float64Vector::new();
            let needed = (num_rows * 8).min(data.len());
            let items = needed / 8;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 8;
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&data[off..off + 8]);
                    v.push(Some(f64::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Float64(v))
        }
        DataType::Date => {
            let mut v = types::vector::DateVector::new();
            let needed = (num_rows * 4).min(data.len());
            let items = needed / 4;
            for i in 0..num_rows {
                if null_bitmap.is_valid(i) && i < items {
                    let off = i * 4;
                    let b = [data[off], data[off + 1], data[off + 2], data[off + 3]];
                    v.push(Some(i32::from_le_bytes(b)));
                } else {
                    v.push(None);
                }
            }
            Ok(Vector::Date(v))
        }
        DataType::String => {
            let mut v = types::vector::StringVector::new();
            if data.len() < 4 {
                return Ok(Vector::String(v));
            }
            let num_strings = u32::from_le_bytes(data[0..4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;
            let mut offset = 4;
            for i in 0..num_strings.min(num_rows) {
                if null_bitmap.is_valid(i) {
                    if offset + 4 > data.len() {
                        v.push(None);
                        continue;
                    }
                    let slen = u32::from_le_bytes(data[offset..offset + 4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;
                    offset += 4;
                    if slen > 0 && offset + slen <= data.len() {
                        let s = std::str::from_utf8(&data[offset..offset + slen]).unwrap_or("");
                        v.push(Some(s));
                        offset += slen;
                    } else {
                        v.push(None);
                    }
                } else {
                    if offset + 4 <= data.len() {
                        let slen = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap_or([0u8; 4])) as usize;
                        offset += 4 + slen;
                    }
                    v.push(None);
                }
            }
            Ok(Vector::String(v))
        }
        _ => Ok(Vector::Null(types::vector::NullVector::new(num_rows))),
    }
}

fn empty_vector(data_type: &DataType) -> Vector {
    match data_type {
        DataType::Boolean => Vector::Boolean(types::vector::BooleanVector::new()),
        DataType::Int8 => Vector::Int8(types::vector::Int8Vector::new()),
        DataType::Int16 => Vector::Int16(types::vector::Int16Vector::new()),
        DataType::Int32 => Vector::Int32(types::vector::Int32Vector::new()),
        DataType::Int64 => Vector::Int64(types::vector::Int64Vector::new()),
        DataType::Int128 => Vector::Int128(types::vector::Int128Vector::new()),
        DataType::Float32 => Vector::Float32(types::vector::Float32Vector::new()),
        DataType::Float64 => Vector::Float64(types::vector::Float64Vector::new()),
        DataType::String => Vector::String(types::vector::StringVector::new()),
        DataType::Date => Vector::Date(types::vector::DateVector::new()),
        DataType::DateTime => Vector::DateTime(types::vector::DateTimeVector::new()),
        _ => Vector::Null(types::vector::NullVector::new(0)),
    }
}

// ---------------------------------------------------------------------------
// Block <-> binary for WAL
// ---------------------------------------------------------------------------

/// Serialize an entire Block to binary format for WAL Insert entry body.
///
/// Layout:
/// ```text
/// schema_json_len : u32 LE
/// schema_json     : [u8; schema_json_len]
/// num_rows        : u32 LE
/// ── per column ──
/// raw_data_len    : u32 LE
/// raw_data        : [u8; raw_data_len]
/// null_bitmap_len : u32 LE
/// null_bitmap     : [u8; null_bitmap_len]
/// ```
fn serialize_block(block: &Block) -> Vec<u8> {
    let schema = block.schema();
    let schema_desc = WalSchemaDesc::from(schema);
    let schema_json = serde_json::to_vec(&schema_desc).unwrap_or_default();

    let mut buf = Vec::new();

    // Schema
    buf.extend_from_slice(&(schema_json.len() as u32).to_le_bytes());
    buf.extend_from_slice(&schema_json);

    // Num rows
    buf.extend_from_slice(&(block.num_rows() as u32).to_le_bytes());

    // Columns
    for col_idx in 0..block.num_columns() {
        if let Some(col) = block.column(col_idx) {
            let raw = serialize_column(col);
            let nb = serialize_null_bitmap(col);

            buf.extend_from_slice(&(raw.len() as u32).to_le_bytes());
            buf.extend_from_slice(&raw);
            buf.extend_from_slice(&(nb.len() as u32).to_le_bytes());
            buf.extend_from_slice(&nb);
        } else {
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
    }

    buf
}

/// Deserialize an Insert entry body back into a Block.
fn deserialize_block(data: &[u8]) -> Result<Block, String> {
    if data.len() < 8 {
        return Err("WAL block data too short".to_string());
    }

    // Schema
    let schema_json_len = u32::from_le_bytes(data[0..4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;
    let schema_json_end = 4 + schema_json_len;
    if schema_json_end > data.len() {
        return Err("WAL block data truncated (schema)".to_string());
    }
    let schema_desc: WalSchemaDesc = serde_json::from_slice(&data[4..schema_json_end])
        .map_err(|e| format!("WAL schema deserialize: {}", e))?;
    let schema = schema_desc.to_schema();
    let num_fields = schema.fields().len();

    // Num rows
    let num_rows_start = schema_json_end;
    if num_rows_start + 4 > data.len() {
        return Err("WAL block data truncated (num_rows)".to_string());
    }
    let num_rows = u32::from_le_bytes(data[num_rows_start..num_rows_start + 4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;

    // Columns
    let mut offset = num_rows_start + 4;
    let mut columns: Vec<Vector> = Vec::with_capacity(num_fields);

    for col_idx in 0..num_fields {
        let field = schema.fields().get(col_idx)
            .ok_or_else(|| "Field index out of bounds".to_string())?;

        if offset + 4 > data.len() {
            columns.push(empty_vector(&field.data_type));
            continue;
        }
        let raw_len = u32::from_le_bytes(data[offset..offset + 4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;
        offset += 4;

        if offset + raw_len > data.len() {
            return Err("WAL block data truncated (col data)".to_string());
        }
        let raw = &data[offset..offset + raw_len];
        offset += raw_len;

        if offset + 4 > data.len() {
            return Err("WAL block data truncated (null bitmap len)".to_string());
        }
        let nb_len = u32::from_le_bytes(data[offset..offset + 4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?) as usize;
        offset += 4;

        if offset + nb_len > data.len() {
            return Err("WAL block data truncated (null bitmap)".to_string());
        }
        let nb_data = &data[offset..offset + nb_len];
        offset += nb_len;

        let null_bitmap = deserialize_null_bitmap(nb_data, num_rows);
        let vec = deserialize_vector(raw, &field.data_type, &null_bitmap, num_rows)?;
        columns.push(vec);
    }

    if columns.is_empty() {
        return Ok(Block::empty(schema));
    }

    Ok(Block::new(schema, columns))
}

// ---------------------------------------------------------------------------
// WAL Writer
// ---------------------------------------------------------------------------

/// Append-only WAL writer. Thread-safe via internal Mutex.
///
/// Every `write_insert` is fsync'd before returning. The WAL file is rotated
/// (truncated) after a successful flush via `write_flush_marker` + `truncate`.
pub struct WalWriter {
    file: Mutex<File>,
    path: PathBuf,
    tablet_id: u64,
}

impl WalWriter {
    /// Open (or create) the WAL file for a tablet.
    /// Does NOT replay existing entries — use `replay_and_recover` for that.
    pub fn open(tablet_dir: &Path, tablet_id: u64) -> Result<Self, String> {
        std::fs::create_dir_all(tablet_dir)
            .map_err(|e| format!("Create tablet dir for WAL: {}", e))?;
        let path = tablet_dir.join(WAL_FILE_NAME);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|e| format!("Failed to open WAL for tablet {}: {}", tablet_id, e))?;
        tracing::debug!("Opened WAL for tablet {} at {:?}", tablet_id, path);
        Ok(Self {
            file: Mutex::new(file),
            path,
            tablet_id,
        })
    }

    /// Append an Insert entry for the given Block.
    /// The entry is fsync'd to disk before returning.
    pub fn write_insert(&self, block: &Block) -> Result<(), String> {
        let body = serialize_block(block);
        self.append_entry(ENTRY_INSERT, &body)
    }

    /// Append a FlushMarker entry (version number of the flush).
    /// After this, `truncate()` can be called to rotate the WAL.
    pub fn write_flush_marker(&self, version: u64) -> Result<(), String> {
        let body = version.to_le_bytes().to_vec();
        self.append_entry(ENTRY_FLUSH_MARKER, &body)
    }

    /// Truncate the WAL to zero bytes (rotate).
    /// Must only be called after a FlushMarker has been written,
    /// and all data up to the marker is safely in Segment files.
    pub fn truncate(&self) -> Result<(), String> {
        let file = &mut *self.file.lock();
        file.set_len(0)
            .map_err(|e| format!("WAL truncate error (tablet {}): {}", self.tablet_id, e))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| format!("WAL seek after trunc (tablet {}): {}", self.tablet_id, e))?;
        file.sync_all()
            .map_err(|e| format!("WAL sync after trunc (tablet {}): {}", self.tablet_id, e))?;
        tracing::info!("WAL truncated for tablet {}", self.tablet_id);
        Ok(())
    }

    /// Force sync the WAL file.
    pub fn sync(&self) -> Result<(), String> {
        let file = self.file.lock();
        file.sync_all()
            .map_err(|e| format!("WAL sync error (tablet {}): {}", self.tablet_id, e))
    }

    /// Get the current WAL file size in bytes.
    pub fn file_size(&self) -> u64 {
        self.file.lock().metadata().map(|m| m.len()).unwrap_or(0)
    }

    // ── internal ──

    fn append_entry(&self, entry_type: u8, body: &[u8]) -> Result<(), String> {
        let file = &mut *self.file.lock();
        let body_len = body.len() as u32;
        let body_crc = crc32(body);
        let body_len_bytes = body_len.to_le_bytes();
        let body_crc_bytes = body_crc.to_le_bytes();

        // Build header + body without checksum first, then compute checksum
        // Header (no checksum): type(1) + body_len(4) + crc32(4) = 9 bytes
        let pre_header_len = 9;
        let total_len = pre_header_len + body.len();

        // Pre-compute checksum bytes placeholder
        let zero_checksum: [u8; 4] = [0u8; 4];

        // Write header
        file.write_all(&[entry_type])
            .map_err(|e| format!("WAL write error (tablet {}): {}", self.tablet_id, e))?;
        file.write_all(&body_len_bytes)
            .map_err(|e| format!("WAL write error (tablet {}): {}", self.tablet_id, e))?;
        file.write_all(&body_crc_bytes)
            .map_err(|e| format!("WAL write error (tablet {}): {}", self.tablet_id, e))?;

        // Compute checksum of everything written so far + body
        let mut header_buf = Vec::with_capacity(9);
        header_buf.push(entry_type);
        header_buf.extend_from_slice(&body_len_bytes);
        header_buf.extend_from_slice(&body_crc_bytes);
        let checksum = crc32(body);
        // Simple combined checksum: crc32(header) ^ body_crc ^ body_len
        let entry_checksum = crc32(&header_buf) ^ body_crc ^ body_len;
        file.write_all(&entry_checksum.to_le_bytes())
            .map_err(|e| format!("WAL write error (tablet {}): {}", self.tablet_id, e))?;

        // Write body
        file.write_all(body)
            .map_err(|e| format!("WAL write error (tablet {}): {}", self.tablet_id, e))?;

        // fsync
        file.sync_all()
            .map_err(|e| format!("WAL sync error (tablet {}): {}", self.tablet_id, e))?;

        tracing::trace!(
            "WAL append tablet={} type={} body_len={} crc=0x{:08X}",
            self.tablet_id,
            entry_type,
            body_len,
            body_crc
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WAL Recovery
// ---------------------------------------------------------------------------

/// Result of replaying the WAL on recovery.
#[derive(Debug, Clone)]
pub struct WalRecoveryResult {
    /// Reconstructed Block from unflushed WAL entries.
    /// None if WAL was clean or had no inserts.
    pub recovered_block: Option<Block>,
    /// Version number of the last FlushMarker seen, if any.
    pub last_flush_version: Option<u64>,
    /// Total number of entries replayed.
    pub entry_count: usize,
}

/// Individual entry parsed from the WAL stream.
#[derive(Debug, Clone)]
pub enum WalEntry {
    Insert(Block),
    FlushMarker(u64),
}

/// Replay WAL entries from the given tablet directory.
///
/// Scans the WAL from the end backwards to find the last FlushMarker,
/// then replays all Insert entries after that point. Returns the
/// recovered block or None if the WAL is empty/missing.
pub fn replay_and_recover(tablet_dir: &Path) -> Result<WalRecoveryResult, String> {
    let wal_path = tablet_dir.join(WAL_FILE_NAME);

    if !wal_path.exists() {
        return Ok(WalRecoveryResult {
            recovered_block: None,
            last_flush_version: None,
            entry_count: 0,
        });
    }

    let metadata = std::fs::metadata(&wal_path)
        .map_err(|e| format!("WAL metadata error: {}", e))?;

    if metadata.len() == 0 {
        return Ok(WalRecoveryResult {
            recovered_block: None,
            last_flush_version: None,
            entry_count: 0,
        });
    }

    // Read entire WAL into memory (WAL is small — cleared on flush)
    let data = std::fs::read(&wal_path)
        .map_err(|e| format!("WAL read error: {}", e))?;

    // Parse all entries
    let entries = parse_all_entries(&data)?;

    // Find the position of the last FlushMarker
    let last_flush_idx = entries.iter()
        .enumerate()
        .rev()
        .find_map(|(i, e)| {
            if let WalEntry::FlushMarker(v) = e {
                Some((i, *v))
            } else {
                None
            }
        });

    let last_flush_version = last_flush_idx.map(|(_, v)| v);

    // Determine which entries to replay
    let replay_start: usize = match last_flush_idx {
        Some((idx, _)) => idx + 1, // Entries after the marker
        None => 0,                 // No marker — replay everything
    };

    // Collect Insert entries for replay
    let insert_blocks: Vec<Block> = entries[replay_start..]
        .iter()
        .filter_map(|e| {
            if let WalEntry::Insert(block) = e {
                Some(block.clone())
            } else {
                None
            }
        })
        .collect();

    let recovered = if insert_blocks.is_empty() {
        None
    } else {
        Block::concat(&insert_blocks)
    };

    tracing::info!(
        "WAL replay: {} total entries, {} inserts replayed, flush_marker={:?}, wal_size={}",
        entries.len(),
        insert_blocks.len(),
        last_flush_version,
        metadata.len()
    );

    Ok(WalRecoveryResult {
        recovered_block: recovered,
        last_flush_version,
        entry_count: entries.len(),
    })
}

/// After replay, truncate the WAL file so future writes start fresh.
pub fn truncate_after_recovery(tablet_dir: &Path) -> Result<(), String> {
    let wal_path = tablet_dir.join(WAL_FILE_NAME);
    if wal_path.exists() {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&wal_path)
            .map_err(|e| format!("WAL truncate after recovery: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("WAL sync after recovery truncate: {}", e))?;
    }
    Ok(())
}

// ── Internal entry parsing ──

/// Parse all WAL entries from raw bytes.
fn parse_all_entries(data: &[u8]) -> Result<Vec<WalEntry>, String> {
    let mut entries = Vec::new();
    let mut offset = 0usize;
    let len = data.len();

    while offset < len {
        // Need at least: type(1) + body_len(4) + crc32(4) + checksum(4) = 13 bytes
        if offset + ENTRY_HEADER_SIZE > len {
            tracing::warn!("WAL truncated at offset {} (remaining {} bytes < header size {})",
                offset, len - offset, ENTRY_HEADER_SIZE);
            break;
        }

        let entry_type = data[offset];
        let body_len = u32::from_le_bytes(data[offset + 1..offset + 5].try_into()
            .map_err(|_| "WAL parse: invalid body_len".to_string())?) as usize;
        let stored_crc = u32::from_le_bytes(data[offset + 5..offset + 9].try_into()
            .map_err(|_| "WAL parse: invalid crc32".to_string())?);
        let _stored_checksum = u32::from_le_bytes(data[offset + 9..offset + 13].try_into()
            .map_err(|_| "WAL parse: invalid checksum".to_string())?);

        let body_start = offset + ENTRY_HEADER_SIZE;
        let entry_end = body_start + body_len;

        if entry_end > len {
            tracing::warn!("WAL entry at offset {} body_len={} exceeds file ({} bytes), stopping",
                offset, body_len, len);
            break;
        }

        let body = &data[body_start..entry_end];

        // Verify CRC32
        let computed_crc = crc32(body);
        if stored_crc != 0 && computed_crc != stored_crc {
            return Err(format!(
                "WAL CRC mismatch at offset {}: stored=0x{:08X} computed=0x{:08X}",
                offset, stored_crc, computed_crc
            ));
        }

        match entry_type {
            ENTRY_INSERT => {
                let block = deserialize_block(body)?;
                entries.push(WalEntry::Insert(block));
            }
            ENTRY_FLUSH_MARKER => {
                if body.len() >= 8 {
                    let version = u64::from_le_bytes(body[0..8].try_into()
                        .map_err(|_| "WAL parse: invalid version".to_string())?);
                    entries.push(WalEntry::FlushMarker(version));
                } else {
                    tracing::warn!("WAL FlushMarker too short ({} bytes), skipping", body.len());
                }
            }
            other => {
                tracing::warn!("WAL unknown entry type 0x{:02X} at offset {}, skipping",
                    other, offset);
            }
        }

        offset = entry_end;
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use types::{DataType, Field, Schema, Block, Vector};

    fn make_schema() -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::String, true),
        ])
    }

    fn make_test_block(schema: &Schema, ids: &[i64], names: &[&str]) -> Block {
        let mut id_vec = types::vector::Int64Vector::new();
        let mut name_vec = types::vector::StringVector::new();

        for (&id, &name) in ids.iter().zip(names.iter()) {
            id_vec.push(Some(id));
            name_vec.push(Some(name));
        }

        let columns: Vec<Vector> = vec![
            Vector::Int64(id_vec),
            Vector::String(name_vec),
        ];
        Block::new(schema.clone(), columns)
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let schema = make_schema();
        let block = make_test_block(&schema, &[1, 2, 3], &["a", "b", "c"]);
        assert_eq!(block.num_rows(), 3);

        let serialized = serialize_block(&block);
        let deserialized = deserialize_block(&serialized).unwrap();

        assert_eq!(deserialized.num_rows(), 3);
        assert!(deserialized.schema().index_of("id").is_some());
        assert!(deserialized.schema().index_of("name").is_some());
    }

    #[test]
    fn test_wal_write_and_replay() {
        let dir = tempfile::tempdir().unwrap();
        let tablet_dir = dir.path().join("tablet_1");
        std::fs::create_dir_all(&tablet_dir).unwrap();

        let schema = make_schema();

        // Open WAL
        let wal = WalWriter::open(&tablet_dir, 1).unwrap();

        // Write inserts
        let block1 = make_test_block(&schema, &[1, 2], &["alice", "bob"]);
        wal.write_insert(&block1).unwrap();

        let block2 = make_test_block(&schema, &[3, 4], &["charlie", "dave"]);
        wal.write_insert(&block2).unwrap();

        // Write flush marker
        wal.write_flush_marker(42).unwrap();

        // Write more after marker (simulating new writes)
        let block3 = make_test_block(&schema, &[5], &["eve"]);
        wal.write_insert(&block3).unwrap();

        drop(wal);

        // Now replay
        let result = replay_and_recover(&tablet_dir).unwrap();
        assert_eq!(result.last_flush_version, Some(42));
        assert!(result.recovered_block.is_some());
        assert_eq!(result.recovered_block.as_ref().unwrap().num_rows(), 1);

        // Clean up
        truncate_after_recovery(&tablet_dir).unwrap();

        // Replay again — should be clean
        let result2 = replay_and_recover(&tablet_dir).unwrap();
        assert!(result2.recovered_block.is_none());
        assert!(result2.last_flush_version.is_none());
    }

    #[test]
    fn test_wal_no_flush_marker() {
        let dir = tempfile::tempdir().unwrap();
        let tablet_dir = dir.path().join("tablet_2");
        std::fs::create_dir_all(&tablet_dir).unwrap();

        let schema = make_schema();

        // Simulate crash: write data, no flush marker
        let wal = WalWriter::open(&tablet_dir, 2).unwrap();
        let block = make_test_block(&schema, &[10, 20], &["x", "y"]);
        wal.write_insert(&block).unwrap();
        drop(wal);

        // Replay — should replay everything
        let result = replay_and_recover(&tablet_dir).unwrap();
        assert_eq!(result.last_flush_version, None);
        assert!(result.recovered_block.is_some());
        assert_eq!(result.recovered_block.unwrap().num_rows(), 2);
    }

    #[test]
    fn test_wal_empty() {
        let dir = tempfile::tempdir().unwrap();
        let tablet_dir = dir.path().join("tablet_3");
        std::fs::create_dir_all(&tablet_dir).unwrap();

        // No WAL exists yet
        let result = replay_and_recover(&tablet_dir).unwrap();
        assert!(result.recovered_block.is_none());
        assert_eq!(result.entry_count, 0);
    }

    #[test]
    fn test_crc32_deterministic() {
        assert_eq!(crc32(b"hello"), crc32(b"hello"));
        assert_ne!(crc32(b"hello"), crc32(b"world"));
    }
}