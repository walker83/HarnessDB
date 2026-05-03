use serde::{Deserialize, Serialize};

/// Metadata for a rowset, persisted alongside tablet data.
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

impl RowsetMeta {
    pub fn new(rowset_id: u64, tablet_id: u64, version: u64) -> Self {
        Self {
            rowset_id,
            tablet_id,
            txn_id: 0,
            version,
            num_rows: 0,
            data_size: 0,
            num_segments: 0,
            empty: true,
            packed_data_size: 0,
            index_size: 0,
        }
    }
}

/// A rowset is a collection of segment files. Immutable once flushed.
#[derive(Debug, Clone)]
pub struct Rowset {
    pub meta: RowsetMeta,
    pub segments: Vec<SegmentRef>,
    pub state: RowsetState,
}

/// Reference to a segment file on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRef {
    pub segment_id: u64,
    pub path: String,
    pub num_rows: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowsetState {
    /// Currently accepting writes (in-memory).
    Active,
    /// Flushed to disk, immutable.
    Committed,
    /// Being compacted.
    Compacting,
    /// Marked for deletion after compaction.
    PendingDelete,
}

impl Rowset {
    pub fn new(meta: RowsetMeta) -> Self {
        Self {
            meta,
            segments: Vec::new(),
            state: RowsetState::Active,
        }
    }

    pub fn with_segments(meta: RowsetMeta, segments: Vec<SegmentRef>) -> Self {
        let num_rows = segments.iter().map(|s| s.num_rows).sum();
        let data_size = segments.iter().map(|s| s.size).sum();
        let num_segments = segments.len() as u32;
        Self {
            meta: RowsetMeta {
                num_rows,
                data_size,
                num_segments,
                empty: segments.is_empty(),
                ..meta
            },
            segments,
            state: RowsetState::Committed,
        }
    }

    pub fn num_rows(&self) -> u64 {
        self.meta.num_rows
    }

    pub fn data_size(&self) -> u64 {
        self.meta.data_size
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn add_segment(&mut self, seg: SegmentRef) {
        self.meta.num_rows += seg.num_rows;
        self.meta.data_size += seg.size;
        self.meta.num_segments += 1;
        self.meta.empty = false;
        self.segments.push(seg);
    }

    /// Mark the rowset as committed (immutable).
    pub fn commit(&mut self) {
        self.state = RowsetState::Committed;
    }

    /// Persist rowset metadata to a JSON file.
    pub fn save_meta(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&(&self.meta, &self.segments))
            .map_err(|e| format!("Serialize rowset meta: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Write rowset meta: {}", e))
    }

    /// Load rowset metadata from a JSON file.
    pub fn load_meta(path: &std::path::Path) -> Result<(RowsetMeta, Vec<SegmentRef>), String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Read rowset meta: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Deserialize rowset meta: {}", e))
    }
}
