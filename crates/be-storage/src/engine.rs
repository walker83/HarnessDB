use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use parking_lot::RwLock;

use common::{DrorisError, Result};
use types::Block;

use crate::compaction::{CompactionExecutor, CompactionManager, CompactionTask, CompactionType};
use crate::index::ColumnPredicate;
use crate::tablet::{Tablet, TabletSchema};

/// The main storage engine that manages tablets and coordinates flush/compaction.
pub struct StorageEngine {
    tablets: RwLock<HashMap<u64, Arc<Tablet>>>,
    compaction_mgr: RwLock<CompactionManager>,
    data_dir: PathBuf,
    global_segment_id: AtomicU64,
    global_rowset_id: AtomicU64,
}

impl StorageEngine {
    /// Create a new storage engine with the given data directory.
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| DrorisError::Storage(format!("Create data dir: {}", e)))?;

        let engine = Self {
            tablets: RwLock::new(HashMap::new()),
            compaction_mgr: RwLock::new(CompactionManager::new(2)),
            data_dir: data_dir.clone(),
            global_segment_id: AtomicU64::new(0),
            global_rowset_id: AtomicU64::new(0),
        };

        Ok(engine)
    }

    /// Create a new tablet with the given schema.
    pub fn create_tablet(&self, tablet_id: u64, schema: TabletSchema) -> Result<()> {
        let mut tablets = self.tablets.write();
        if tablets.contains_key(&tablet_id) {
            return Err(DrorisError::Storage(format!("tablet {} already exists", tablet_id)));
        }
        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        std::fs::create_dir_all(&tablet_dir)
            .map_err(|e| DrorisError::Storage(format!("Create tablet dir: {}", e)))?;
        let tablet = Tablet::new(tablet_id, schema, self.data_dir.clone());
        tablets.insert(tablet_id, Arc::new(tablet));
        tracing::info!("Created tablet {}", tablet_id);
        Ok(())
    }

    /// Check if a tablet exists.
    pub fn get_tablet(&self, tablet_id: u64) -> bool {
        self.tablets.read().contains_key(&tablet_id)
    }

    /// Drop a tablet, removing its data directory.
    pub fn drop_tablet(&self, tablet_id: u64) -> Result<()> {
        let mut tablets = self.tablets.write();
        let _tablet = tablets
            .remove(&tablet_id)
            .ok_or_else(|| DrorisError::Storage(format!("tablet {} not found", tablet_id)))?;

        // Remove tablet data directory
        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        if tablet_dir.exists() {
            std::fs::remove_dir_all(&tablet_dir)
                .map_err(|e| DrorisError::Storage(format!("Remove tablet dir: {}", e)))?;
        }
        tracing::info!("Dropped tablet {}", tablet_id);
        Ok(())
    }

    /// Write a batch of rows to a tablet.
    pub fn write_batch(&self, tablet_id: u64, block: &Block) -> Result<()> {
        let tablets = self.tablets.read();
        let tablet = tablets
            .get(&tablet_id)
            .ok_or_else(|| DrorisError::Storage(format!("tablet {} not found", tablet_id)))?;
        tablet.write(block)
            .map_err(|e| DrorisError::Storage(e))
    }

    /// Read data from a tablet with optional column projection and predicates.
    pub fn read_tablet(
        &self,
        tablet_id: u64,
        projection: Option<&[usize]>,
        predicates: &[ColumnPredicate],
    ) -> Result<Block> {
        let tablets = self.tablets.read();
        let tablet = tablets
            .get(&tablet_id)
            .ok_or_else(|| DrorisError::Storage(format!("tablet {} not found", tablet_id)))?;
        tablet.read(projection, predicates)
            .map_err(|e| DrorisError::Storage(e))
    }

    /// Explicitly flush a tablet's memtable to disk.
    pub fn flush(&self, tablet_id: u64) -> Result<()> {
        let tablets = self.tablets.read();
        let tablet = tablets
            .get(&tablet_id)
            .ok_or_else(|| DrorisError::Storage(format!("tablet {} not found", tablet_id)))?;
        tablet.flush()
            .map_err(|e| DrorisError::Storage(e))
    }

    /// Trigger compaction for a tablet.
    pub fn compact(&self, tablet_id: u64, compaction_type: CompactionType) -> Result<()> {
        let tablets = self.tablets.read();
        let tablet = tablets
            .get(&tablet_id)
            .ok_or_else(|| DrorisError::Storage(format!("tablet {} not found", tablet_id)))?;

        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        let new_rowset = match compaction_type {
            CompactionType::Base => CompactionExecutor::base_compact(
                tablet,
                &tablet_dir,
                &self.global_segment_id,
                &self.global_rowset_id,
            ),
            CompactionType::Cumulative => {
                let committed = tablet.committed_rowsets();
                CompactionExecutor::cumulative_compact(
                    tablet,
                    &committed,
                    &tablet_dir,
                    &self.global_segment_id,
                    &self.global_rowset_id,
                )
            }
        };

        match new_rowset {
            Ok(rowset) => {
                // Remove old rowsets and add the new one
                let old_ids: Vec<u64> = tablet.committed_rowsets()
                    .iter()
                    .map(|r| r.meta.rowset_id)
                    .collect();
                tablet.remove_rowsets(&old_ids);
                tablet.add_rowset(rowset);
                tracing::info!("Compaction completed for tablet {}", tablet_id);
                Ok(())
            }
            Err(e) => {
                // Not an error if there's nothing to compact
                tracing::info!("Compaction skipped for tablet {}: {}", tablet_id, e);
                Ok(())
            }
        }
    }

    /// Submit a compaction task to the background scheduler.
    pub fn schedule_compaction(&self, tablet_id: u64, compaction_type: CompactionType) {
        let tablets = self.tablets.read();
        if let Some(tablet) = tablets.get(&tablet_id) {
            let committed = tablet.committed_rowsets();
            let estimated_size: u64 = committed.iter().map(|r| r.data_size()).sum();
            let rowset_ids: Vec<u64> = committed.iter().map(|r| r.meta.rowset_id).collect();

            let task = CompactionTask {
                tablet_id,
                rowset_ids,
                compaction_type,
                estimated_size,
            };

            self.compaction_mgr.write().submit(task);
        }
    }

    /// Process one pending compaction task (for background polling).
    pub fn poll_and_run_compaction(&self) -> Result<bool> {
        let task = {
            let mut mgr = self.compaction_mgr.write();
            mgr.poll_task()
        };

        match task {
            Some(task) => {
                let result = self.compact(task.tablet_id, task.compaction_type);
                self.compaction_mgr.write().complete();
                result.map(|_| true)
            }
            None => Ok(false),
        }
    }

    pub fn tablet_count(&self) -> usize {
        self.tablets.read().len()
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

impl Default for StorageEngine {
    fn default() -> Self {
        Self::open("/tmp/rovisdb/storage").expect("Failed to create default storage engine")
    }
}
