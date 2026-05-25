//! Backup manager - handles backup and restore operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use parking_lot::RwLock;
use crate::repository::RepositoryManager;

/// Information about a backed-up table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupTableInfo {
    pub name: String,
    pub file_name: String,
    pub row_count: u64,
    pub size_bytes: u64,
}

/// Backup manifest describing a backup set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_name: String,
    pub database: String,
    pub timestamp: String,
    pub backup_type: String,
    pub tables: Vec<BackupTableInfo>,
    pub total_size_bytes: u64,
    pub total_rows: u64,
}

/// Backup manager coordinates backup and restore operations
pub struct BackupManager {
    repo_manager: RwLock<RepositoryManager>,
    data_dir: PathBuf,
}

impl BackupManager {
    pub fn new(meta_dir: &str, data_dir: &str) -> Self {
        Self {
            repo_manager: RwLock::new(RepositoryManager::new(meta_dir)),
            data_dir: PathBuf::from(data_dir),
        }
    }

    /// Create a backup repository
    pub fn create_repository(&self, name: &str, path: &str) -> Result<(), String> {
        self.repo_manager.write().create_repository(name, path)
    }

    /// Drop a backup repository
    pub fn drop_repository(&self, name: &str) -> Result<(), String> {
        self.repo_manager.write().drop_repository(name)
    }

    /// List all repositories
    pub fn list_repositories(&self) -> Vec<String> {
        self.repo_manager.read().list_repositories()
    }

    /// Get repository path
    pub fn get_repo_path(&self, name: &str) -> Result<PathBuf, String> {
        self.repo_manager.read().get_repo_path(name)
    }

    /// Backup a database
    pub fn backup_database(
        &self,
        catalog: &fe_catalog::CatalogManager,
        database: &str,
        repository: &str,
        backup_name: &str,
    ) -> Result<String, String> {
        let repo_path = self.get_repo_path(repository)?;

        // Create backup directory
        let backup_dir = repo_path.join(backup_name);
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        // Get tables from catalog
        let tables = catalog.list_tables(database)
            .ok_or_else(|| format!("Database '{}' not found", database))?;

        let mut table_infos = Vec::new();
        let mut total_size = 0u64;
        let mut total_rows = 0u64;

        for table_name in &tables {
            let table_dir = self.data_dir.join(database).join(table_name);
            let parquet_file = table_dir.join("data.parquet");

            if parquet_file.exists() {
                let dest_dir = backup_dir.join(table_name);
                std::fs::create_dir_all(&dest_dir)
                    .map_err(|e| format!("Failed to create table backup dir: {}", e))?;
                let dest_file = dest_dir.join("data.parquet");

                std::fs::copy(&parquet_file, &dest_file)
                    .map_err(|e| format!("Failed to copy {}: {}", table_name, e))?;

                let metadata = std::fs::metadata(&dest_file)
                    .map_err(|e| format!("Failed to get file metadata: {}", e))?;
                let size = metadata.len();
                total_size += size;

                // Try to read row count (simplified - just record 0 for now)
                let row_count = 0u64;
                total_rows += row_count;

                table_infos.push(BackupTableInfo {
                    name: table_name.clone(),
                    file_name: format!("{}/data.parquet", table_name),
                    row_count,
                    size_bytes: size,
                });
            }
        }

        // Write manifest
        let manifest = BackupManifest {
            backup_name: backup_name.to_string(),
            database: database.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            backup_type: "full".to_string(),
            tables: table_infos,
            total_size_bytes: total_size,
            total_rows,
        };

        let manifest_path = backup_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(&manifest_path, manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Write catalog snapshot
        if let Some(db) = catalog.get_database(database) {
            let snapshot_path = backup_dir.join("catalog_snapshot.json");
            let snapshot_json = serde_json::to_string_pretty(&db)
                .map_err(|e| format!("Failed to serialize catalog snapshot: {}", e))?;
            std::fs::write(&snapshot_path, snapshot_json)
                .map_err(|e| format!("Failed to write catalog snapshot: {}", e))?;
        }

        Ok(format!(
            "BACKUP DATABASE `{}` TO `{}` AS `{}` completed. {} tables backed up, {} bytes.",
            database, repository, backup_name, manifest.tables.len(), total_size
        ))
    }

    /// Restore a database from backup
    pub fn restore_database(
        &self,
        catalog: &fe_catalog::CatalogManager,
        database: &str,
        repository: &str,
        backup_name: &str,
    ) -> Result<String, String> {
        let repo_path = self.get_repo_path(repository)?;
        let backup_dir = repo_path.join(backup_name);

        if !backup_dir.exists() {
            return Err(format!("Backup '{}' not found in repository '{}'", backup_name, repository));
        }

        // Read manifest
        let manifest_path = backup_dir.join("manifest.json");
        let manifest_json = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest: BackupManifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;

        // Create database if not exists
        if catalog.get_database(database).is_none() {
            catalog.create_database(database)
                .map_err(|e| format!("Failed to create database: {}", e))?;
        }

        let mut restored_tables = 0;

        for table_info in &manifest.tables {
            let src_file = backup_dir.join(&table_info.file_name);
            let dest_dir = self.data_dir.join(database).join(&table_info.name);
            std::fs::create_dir_all(&dest_dir)
                .map_err(|e| format!("Failed to create table directory: {}", e))?;
            let dest_file = dest_dir.join("data.parquet");

            if src_file.exists() {
                std::fs::copy(&src_file, &dest_file)
                    .map_err(|e| format!("Failed to restore {}: {}", table_info.name, e))?;
                restored_tables += 1;
            }
        }

        Ok(format!(
            "RESTORE DATABASE `{}` FROM `{}` AS `{}` completed. {} tables restored.",
            database, repository, backup_name, restored_tables
        ))
    }
}
