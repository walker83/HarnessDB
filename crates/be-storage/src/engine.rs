use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use parking_lot::RwLock;

use common::{DrorisError, Result, StorageError};
use types::Block;

use crate::compaction::{CompactionExecutor, CompactionManager, CompactionTask, CompactionType};
use crate::index::ColumnPredicate;
use crate::tablet::{Tablet, TabletSchema};

/// The main storage engine that manages tablets and coordinates flush/compaction.
pub struct StorageEngine {
    tablets: DashMap<u64, Arc<Tablet>>,
    compaction_mgr: RwLock<CompactionManager>,
    data_dir: PathBuf,
    global_segment_id: AtomicU64,
    global_rowset_id: AtomicU64,
}

impl StorageEngine {
    /// Create a new storage engine with the given data directory.
    /// Recovers existing tablets from disk if any.
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, format!("Create data dir: {}", e)))?;

        let engine = Self {
            tablets: DashMap::new(),
            compaction_mgr: RwLock::new(CompactionManager::new(2)),
            data_dir: data_dir.clone(),
            global_segment_id: AtomicU64::new(0),
            global_rowset_id: AtomicU64::new(0),
        };

        // Recover existing tablets from disk
        engine.recover()?;

        Ok(engine)
    }

    /// Recover existing tablets from disk.
    /// Scans the data directory for tablet_* folders and reloads them.
    fn recover(&self) -> Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| DrorisError::storage(StorageError::ReadFailed, format!("Read data dir: {}", e)))?;

        let mut recovered_count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| DrorisError::storage(StorageError::ReadFailed, format!("Read dir entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                    if dir_name.starts_with("tablet_") {
                        if let Ok(tablet_id) = dir_name[7..].parse::<u64>() {
                            // Try to recover this tablet
                            match self.recover_tablet(tablet_id) {
                                Ok(_) => {
                                    recovered_count += 1;
                                    tracing::info!("Recovered tablet {}", tablet_id);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to recover tablet {}: {}", tablet_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Recovery complete: {} tablets recovered", recovered_count);
        Ok(())
    }

    /// Recover a single tablet from disk.
    fn recover_tablet(&self, tablet_id: u64) -> Result<()> {
        // Read tablet schema from metadata file
        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        let schema_path = tablet_dir.join("schema.json");

        let schema: TabletSchema = if schema_path.exists() {
            let json = std::fs::read_to_string(&schema_path)
                .map_err(|e| DrorisError::storage(StorageError::ReadFailed, format!("Read schema: {}", e)))?;
            serde_json::from_str(&json)
                .map_err(|e| DrorisError::storage(StorageError::ReadFailed, format!("Parse schema: {}", e)))?
        } else {
            return Err(DrorisError::storage(
                StorageError::TabletNotFound,
                format!("Schema file not found for tablet {}", tablet_id),
            ));
        };

        // Load tablet from disk
        let tablet = Tablet::load_from_disk(tablet_id, schema, self.data_dir.clone())
            .map_err(|e| DrorisError::storage(StorageError::ReadFailed, e))?;

        self.tablets.insert(tablet_id, Arc::new(tablet));
        Ok(())
    }

    /// Create a new tablet with the given schema.
    /// Persists schema to disk for recovery.
    pub fn create_tablet(&self, tablet_id: u64, schema: TabletSchema) -> Result<()> {
        if self.tablets.contains_key(&tablet_id) {
            return Err(DrorisError::storage_with_tablet(StorageError::TabletAlreadyExists, tablet_id, format!("tablet {} already exists", tablet_id)));
        }
        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        std::fs::create_dir_all(&tablet_dir)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, format!("Create tablet dir: {}", e)))?;
        
        // Save schema to disk for recovery
        let schema_path = tablet_dir.join("schema.json");
        let schema_json = serde_json::to_string_pretty(&schema)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, format!("Serialize schema: {}", e)))?;
        std::fs::write(&schema_path, schema_json)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, format!("Write schema: {}", e)))?;
        
        let tablet = Tablet::new(tablet_id, schema, self.data_dir.clone());
        self.tablets.insert(tablet_id, Arc::new(tablet));
        tracing::info!("Created tablet {}", tablet_id);
        Ok(())
    }

    /// Check if a tablet exists.
    pub fn get_tablet(&self, tablet_id: u64) -> bool {
        self.tablets.contains_key(&tablet_id)
    }

    /// Drop a tablet, removing its data directory.
    pub fn drop_tablet(&self, tablet_id: u64) -> Result<()> {
        let _tablet = self.tablets
            .remove(&tablet_id)
            .map(|(_, v)| v)
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;

        // Remove tablet data directory
        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        if tablet_dir.exists() {
            std::fs::remove_dir_all(&tablet_dir)
                .map_err(|e| DrorisError::storage(StorageError::WriteFailed, format!("Remove tablet dir: {}", e)))?;
        }
        tracing::info!("Dropped tablet {}", tablet_id);
        Ok(())
    }

    /// Write a batch of rows to a tablet.
    pub fn write_batch(&self, tablet_id: u64, block: &Block) -> Result<()> {
        let tablet = self.tablets
            .get(&tablet_id)
            .map(|v| v.clone())
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;
        tablet.write(block)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, e.to_string()))
    }

    /// Read data from a tablet with optional column projection and predicates.
    pub fn read_tablet(
        &self,
        tablet_id: u64,
        projection: Option<&[usize]>,
        predicates: &[ColumnPredicate],
    ) -> Result<Block> {
        let tablet = self.tablets
            .get(&tablet_id)
            .map(|v| v.clone())
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;
        tablet.read(projection, predicates)
            .map_err(|e| DrorisError::storage(StorageError::ReadFailed, e.to_string()))
    }

    /// Explicitly flush a tablet's memtable to disk.
    pub fn flush(&self, tablet_id: u64) -> Result<()> {
        let tablet = self.tablets
            .get(&tablet_id)
            .map(|v| v.clone())
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;
        tablet.flush()
            .map_err(|e| DrorisError::storage(StorageError::FlushFailed, e.to_string()))
    }

    /// Delete rows from a tablet matching the given predicates.
    pub fn delete(&self, tablet_id: u64, predicates: &[ColumnPredicate]) -> Result<usize> {
        let tablet = self.tablets
            .get(&tablet_id)
            .map(|v| v.clone())
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;
        tablet.delete(predicates)
            .map_err(|e| DrorisError::storage(StorageError::WriteFailed, e.to_string()))
    }

    /// Trigger compaction for a tablet.
    pub fn compact(&self, tablet_id: u64, compaction_type: CompactionType) -> Result<()> {
        let tablet = self.tablets
            .get(&tablet_id)
            .map(|v| v.clone())
            .ok_or_else(|| DrorisError::storage_with_tablet(StorageError::TabletNotFound, tablet_id, format!("tablet {} not found", tablet_id)))?;

        let tablet_dir = self.data_dir.join(format!("tablet_{}", tablet_id));
        let new_rowset = match compaction_type {
            CompactionType::Base => CompactionExecutor::base_compact(
                &tablet,
                &tablet_dir,
                &self.global_segment_id,
                &self.global_rowset_id,
            ),
            CompactionType::Cumulative => {
                let committed = tablet.committed_rowsets();
                CompactionExecutor::cumulative_compact(
                    &tablet,
                    &committed,
                    &tablet_dir,
                    &self.global_segment_id,
                    &self.global_rowset_id,
                )
            }
        };

        match new_rowset {
            Ok(rowset) => {
                // Collect old segment file paths before removing rowsets
                let old_segment_paths: Vec<String> = tablet.committed_rowsets()
                    .iter()
                    .flat_map(|r| r.segments.iter().map(|s| s.path.clone()))
                    .collect();
                
                // Remove old rowsets and add the new one
                let old_ids: Vec<u64> = tablet.committed_rowsets()
                    .iter()
                    .map(|r| r.meta.rowset_id)
                    .collect();
                
                tablet.remove_rowsets(&old_ids);
                tablet.add_rowset(rowset);
                
                // Delete old segment files from disk
                for path in &old_segment_paths {
                    if let Err(e) = std::fs::remove_file(path) {
                        tracing::warn!("Failed to delete old segment file {}: {}", path, e);
                    }
                }
                
                tracing::info!("Compaction completed for tablet {}, deleted {} old segment files", 
                    tablet_id, old_segment_paths.len());
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
        if let Some(tablet) = self.tablets.get(&tablet_id) {
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
        self.tablets.len()
    }

    /// Get the key column index for a tablet (first column marked as key).
    /// Returns None if tablet doesn't exist.
    pub fn get_key_column_index(&self, tablet_id: u64) -> Option<usize> {
        self.tablets.get(&tablet_id).map(|t| {
            t.schema.columns.iter().position(|c| c.is_key).unwrap_or(0)
        })
    }

    /// Get the key column name for a tablet.
    /// Returns None if tablet doesn't exist.
    pub fn get_key_column_name(&self, tablet_id: u64) -> Option<String> {
        self.tablets.get(&tablet_id).map(|t| {
            t.schema.columns.iter()
                .position(|c| c.is_key)
                .and_then(|idx| t.schema.columns.get(idx))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "id".to_string())
        })
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

impl Default for StorageEngine {
    fn default() -> Self {
        Self::open("/tmp/rorisdb/storage").expect("Failed to create default storage engine")
    }
}
