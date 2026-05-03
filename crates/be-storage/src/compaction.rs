use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use types::Block;

use crate::rowset::{Rowset, RowsetMeta, SegmentRef};
use crate::segment::{SegmentReader, SegmentWriter};
use crate::tablet::Tablet;

/// A compaction task to be scheduled.
#[derive(Debug, Clone)]
pub struct CompactionTask {
    pub tablet_id: u64,
    pub rowset_ids: Vec<u64>,
    pub compaction_type: CompactionType,
    pub estimated_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionType {
    /// Merge several small segments into a larger one.
    Cumulative,
    /// Full merge of all segments in a tablet.
    Base,
}

impl PartialEq for CompactionTask {
    fn eq(&self, other: &Self) -> bool {
        self.tablet_id == other.tablet_id
            && self.rowset_ids == other.rowset_ids
    }
}

impl Eq for CompactionTask {}

impl PartialOrd for CompactionTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompactionTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher estimated_size = higher priority
        self.estimated_size.cmp(&other.estimated_size)
    }
}

/// Manages background compaction tasks with a priority queue.
pub struct CompactionManager {
    pending_tasks: BinaryHeap<CompactionTask>,
    max_concurrent: usize,
    running: usize,
}

impl CompactionManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            pending_tasks: BinaryHeap::new(),
            max_concurrent,
            running: 0,
        }
    }

    pub fn submit(&mut self, task: CompactionTask) {
        self.pending_tasks.push(task);
    }

    pub fn poll_task(&mut self) -> Option<CompactionTask> {
        if self.running >= self.max_concurrent {
            return None;
        }
        let task = self.pending_tasks.pop()?;
        self.running += 1;
        Some(task)
    }

    pub fn complete(&mut self) {
        if self.running > 0 {
            self.running -= 1;
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_tasks.len()
    }

    pub fn running_count(&self) -> usize {
        self.running
    }
}

/// Compaction executor that merges rowsets.
pub struct CompactionExecutor;

impl CompactionExecutor {
    /// Execute a cumulative compaction: merge small rowsets into fewer larger ones.
    pub fn cumulative_compact(
        tablet: &Tablet,
        rowsets: &[Rowset],
        output_dir: &Path,
        next_segment_id: &AtomicU64,
        next_rowset_id: &AtomicU64,
    ) -> Result<Rowset, String> {
        let _schema = tablet.schema.to_schema();

        // Read all segments from the input rowsets
        let mut blocks = Vec::new();
        for rowset in rowsets {
            for seg_ref in &rowset.segments {
                let path = Path::new(&seg_ref.path);
                if path.exists() {
                    let block = SegmentReader::scan_segment(path, None, &[])?;
                    if !block.is_empty() {
                        blocks.push(block);
                    }
                }
            }
        }

        if blocks.is_empty() {
            return Err("No data to compact".to_string());
        }

        // Merge into a single block
        let merged = Block::concat(&blocks).ok_or("Failed to merge blocks")?;

        // Write merged block as a new segment
        let seg_id = next_segment_id.fetch_add(1, AtomicOrdering::SeqCst);
        let rowset_id = next_rowset_id.fetch_add(1, AtomicOrdering::SeqCst);
        let seg_path = output_dir.join(format!("seg_{}.dat", seg_id));

        let file_size = SegmentWriter::write_segment(&seg_path, &merged)?;

        let seg_ref = SegmentRef {
            segment_id: seg_id,
            path: seg_path.to_string_lossy().to_string(),
            num_rows: merged.num_rows() as u64,
            size: file_size,
        };

        let meta = RowsetMeta::new(rowset_id, tablet.tablet_id, tablet.max_version() + 1);
        let mut new_rowset = Rowset::new(meta);
        new_rowset.add_segment(seg_ref);
        new_rowset.commit();

        // Save new rowset meta
        let meta_path = output_dir.join(format!("rowset_{}.json", rowset_id));
        new_rowset.save_meta(&meta_path)?;

        tracing::info!(
            "Cumulative compaction for tablet {}: merged {} rowsets into {} rows, {} bytes",
            tablet.tablet_id,
            rowsets.len(),
            merged.num_rows(),
            file_size
        );

        Ok(new_rowset)
    }

    /// Execute a base compaction: full merge of all committed rowsets.
    pub fn base_compact(
        tablet: &Tablet,
        output_dir: &Path,
        next_segment_id: &AtomicU64,
        next_rowset_id: &AtomicU64,
    ) -> Result<Rowset, String> {
        let committed = tablet.committed_rowsets();
        if committed.len() <= 1 {
            return Err("Not enough rowsets for base compaction".to_string());
        }
        Self::cumulative_compact(tablet, &committed, output_dir, next_segment_id, next_rowset_id)
    }
}
