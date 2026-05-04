use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub repo_type: RepositoryType,
    pub properties: HashMap<String, String>,
    pub create_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepositoryType {
    Local,
    S3,
    Hdfs,
}

impl Repository {
    pub fn new(name: String, repo_type: RepositoryType, properties: HashMap<String, String>) -> Self {
        Self {
            name,
            repo_type,
            properties,
            create_time: current_timestamp(),
        }
    }

    pub fn base_path(&self) -> PathBuf {
        match &self.repo_type {
            RepositoryType::Local => {
                self.properties
                    .get("location")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp/roris_backup"))
            }
            RepositoryType::S3 => {
                self.properties
                    .get("location")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp/roris_backup"))
            }
            RepositoryType::Hdfs => {
                self.properties
                    .get("location")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp/roris_backup"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMeta {
    pub backup_name: String,
    pub database: String,
    pub version: u64,
    pub create_time: u64,
    pub tables: Vec<TableBackupMeta>,
    pub total_size: u64,
    pub status: BackupStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBackupMeta {
    pub table_name: String,
    pub partitions: Vec<PartitionBackupMeta>,
    pub schema: TableSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionBackupMeta {
    pub partition_id: u64,
    pub partition_name: String,
    pub tablets: Vec<TabletBackupMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletBackupMeta {
    pub tablet_id: u64,
    pub rowsets: Vec<RowsetBackupMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowsetBackupMeta {
    pub rowset_id: u64,
    pub segment_files: Vec<String>,
    pub num_rows: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub columns: Vec<ColumnInfo>,
    pub keys_type: String,
    pub distribution: DistributionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_key: bool,
    pub agg_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionInfo {
    pub dist_type: String,
    pub buckets: usize,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

impl Default for BackupStatus {
    fn default() -> Self {
        BackupStatus::Pending
    }
}

pub struct BackupManager {
    repositories: RwLock<HashMap<String, Repository>>,
    backups: RwLock<HashMap<String, BackupMeta>>,
    repo_base_dir: PathBuf,
}

impl BackupManager {
    pub fn new(repo_base_dir: impl Into<PathBuf>) -> Self {
        Self {
            repositories: RwLock::new(HashMap::new()),
            backups: RwLock::new(HashMap::new()),
            repo_base_dir: repo_base_dir.into(),
        }
    }

    pub async fn create_repository(
        &self,
        name: String,
        repo_type: RepositoryType,
        properties: Vec<(String, String)>,
    ) -> common::Result<()> {
        let props: HashMap<String, String> = properties.into_iter().collect();
        let repo = Repository::new(name.clone(), repo_type, props);

        let base_path = self.repo_base_dir.join(&name);
        tokio::fs::create_dir_all(&base_path)
            .await
            .map_err(|e| common::DrorisError::Internal(format!("Failed to create repo dir: {}", e)))?;

        let mut repos = self.repositories.write().await;
        repos.insert(name, repo);
        Ok(())
    }

    pub async fn drop_repository(&self, name: &str, if_exists: bool) -> common::Result<()> {
        let mut repos = self.repositories.write().await;
        if if_exists {
            repos.remove(name);
            Ok(())
        } else {
            repos
                .remove(name)
                .ok_or_else(|| common::DrorisError::Internal(format!("Repository {} not found", name)))?;
            Ok(())
        }
    }

    pub async fn list_repositories(&self) -> Vec<Repository> {
        let repos = self.repositories.read().await;
        repos.values().cloned().collect()
    }

    pub async fn get_repository(&self, name: &str) -> Option<Repository> {
        let repos = self.repositories.read().await;
        repos.get(name).cloned()
    }

    pub async fn start_backup(
        &self,
        backup_name: String,
        database: String,
        tables: Vec<TableBackupMeta>,
    ) -> common::Result<()> {
        let total_size = tables.iter().map(|t| {
            t.partitions.iter().map(|p| {
                p.tablets.iter().map(|tab| {
                    tab.rowsets.iter().map(|r| r.size).sum::<u64>()
                }).sum::<u64>()
            }).sum::<u64>()
        }).sum::<u64>();

        let meta = BackupMeta {
            backup_name: backup_name.clone(),
            database,
            version: 1,
            create_time: current_timestamp(),
            tables,
            total_size,
            status: BackupStatus::Completed,
        };

        let mut backups = self.backups.write().await;
        backups.insert(backup_name, meta);
        Ok(())
    }

    pub async fn get_backup(&self, name: &str) -> Option<BackupMeta> {
        let backups = self.backups.read().await;
        backups.get(name).cloned()
    }

    pub fn repo_path(&self, repo_name: &str) -> PathBuf {
        self.repo_base_dir.join(repo_name)
    }

    pub fn backup_path(&self, repo_name: &str, backup_name: &str) -> PathBuf {
        self.repo_path(repo_name).join(backup_name)
    }

    pub fn database_backup_path(&self, repo_name: &str, backup_name: &str, db: &str) -> PathBuf {
        self.backup_path(repo_name, backup_name).join(db)
    }

    pub fn table_backup_path(
        &self,
        repo_name: &str,
        backup_name: &str,
        db: &str,
        table: &str,
    ) -> PathBuf {
        self.database_backup_path(repo_name, backup_name, db).join(table)
    }

    pub fn tablet_backup_path(
        &self,
        repo_name: &str,
        backup_name: &str,
        db: &str,
        table: &str,
        tablet_id: u64,
    ) -> PathBuf {
        self.table_backup_path(repo_name, backup_name, db, table).join(format!("tablet_{}", tablet_id))
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub type BackupManagerRef = Arc<BackupManager>;

pub fn create_backup_manager(base_dir: impl Into<PathBuf>) -> BackupManagerRef {
    Arc::new(BackupManager::new(base_dir))
}
