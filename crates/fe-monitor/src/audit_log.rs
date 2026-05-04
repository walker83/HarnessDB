use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub host: String,
    pub database: Option<String>,
    pub query: String,
    pub query_type: QueryType,
    pub status: QueryStatus,
    pub duration_ms: u64,
    pub rows_affected: Option<u64>,
    pub bytes_scanned: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    CreateDatabase,
    DropDatabase,
    CreateTable,
    DropTable,
    AlterTable,
    CreateView,
    DropView,
    ShowDatabases,
    ShowTables,
    Other,
}

impl QueryType {
    pub fn from_sql(sql: &str) -> Self {
        let sql_lower = sql.trim().to_lowercase();
        if sql_lower.starts_with("select") {
            QueryType::Select
        } else if sql_lower.starts_with("insert") {
            QueryType::Insert
        } else if sql_lower.starts_with("update") {
            QueryType::Update
        } else if sql_lower.starts_with("delete") {
            QueryType::Delete
        } else if sql_lower.starts_with("create database") {
            QueryType::CreateDatabase
        } else if sql_lower.starts_with("drop database") {
            QueryType::DropDatabase
        } else if sql_lower.starts_with("create table") {
            QueryType::CreateTable
        } else if sql_lower.starts_with("drop table") {
            QueryType::DropTable
        } else if sql_lower.starts_with("alter table") {
            QueryType::AlterTable
        } else if sql_lower.starts_with("create view") {
            QueryType::CreateView
        } else if sql_lower.starts_with("drop view") {
            QueryType::DropView
        } else if sql_lower.starts_with("show databases") {
            QueryType::ShowDatabases
        } else if sql_lower.starts_with("show tables") {
            QueryType::ShowTables
        } else {
            QueryType::Other
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueryStatus {
    Success,
    Failed,
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    pub enabled: bool,
    pub log_dir: PathBuf,
    pub max_file_size_mb: usize,
    pub max_files: usize,
    pub log_queries: bool,
    pub log_slow_queries_only: bool,
    pub slow_query_threshold_ms: u64,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: PathBuf::from("data/audit"),
            max_file_size_mb: 100,
            max_files: 10,
            log_queries: true,
            log_slow_queries_only: false,
            slow_query_threshold_ms: 1000,
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    config: AuditLogConfig,
    current_file: Arc<RwLock<Option<File>>>,
    current_file_size: Arc<RwLock<usize>>,
    current_file_index: Arc<RwLock<usize>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self::with_config(AuditLogConfig::default())
    }

    pub fn with_config(config: AuditLogConfig) -> Self {
        let logger = Self {
            config: config.clone(),
            current_file: Arc::new(RwLock::new(None)),
            current_file_size: Arc::new(RwLock::new(0)),
            current_file_index: Arc::new(RwLock::new(0)),
        };

        if config.enabled {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                logger.rotate_if_needed().await;
            });
        }

        logger
    }

    async fn rotate_if_needed(&self) {
        let max_size = self.config.max_file_size_mb * 1024 * 1024;
        let mut size = self.current_file_size.write().await;
        let mut file = self.current_file.write().await;
        let mut index = self.current_file_index.write().await;

        if *size >= max_size || file.is_none() {
            if let Some(mut f) = file.take() {
                let _ = f.shutdown().await;
            }

            *index += 1;
            if *index > self.config.max_files {
                *index = 1;
            }

            let log_path = self.config.log_dir.join(format!("audit_{:04}.log", *index));

            if let Some(parent) = log_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .await
            {
                Ok(f) => {
                    *file = Some(f);
                    *size = 0;
                }
                Err(e) => {
                    tracing::error!("Failed to open audit log file: {}", e);
                }
            }
        }
    }

    pub async fn log_query(
        &self,
        user: String,
        host: String,
        database: Option<String>,
        query: String,
        status: QueryStatus,
        duration_ms: u64,
        rows_affected: Option<u64>,
        bytes_scanned: Option<u64>,
        error_message: Option<String>,
    ) {
        if !self.config.enabled {
            return;
        }

        if self.config.log_slow_queries_only && duration_ms < self.config.slow_query_threshold_ms {
            return;
        }

        let query_type = QueryType::from_sql(&query);

        let entry = AuditLogEntry {
            timestamp: Utc::now(),
            user,
            host,
            database,
            query,
            query_type,
            status,
            duration_ms,
            rows_affected,
            bytes_scanned,
            error_message,
        };

        if self.config.log_queries {
            self.write_entry(&entry).await;
        }
    }

    async fn write_entry(&self, entry: &AuditLogEntry) {
        self.rotate_if_needed().await;

        let log_line = serde_json::to_string(entry).unwrap_or_else(|_| "Failed to serialize entry".to_string());
        let log_line = format!("{}\n", log_line);

        let mut file = self.current_file.write().await;
        let mut size = self.current_file_size.write().await;

        if let Some(f) = file.as_mut() {
            match f.write_all(log_line.as_bytes()).await {
                Ok(_) => {
                    *size += log_line.len();
                    let _ = f.flush().await;
                }
                Err(e) => {
                    tracing::error!("Failed to write audit log: {}", e);
                }
            }
        }
    }

    pub async fn flush(&self) {
        let mut file = self.current_file.write().await;
        if let Some(f) = file.as_mut() {
            let _ = f.flush().await;
        }
    }

    pub fn get_config(&self) -> &AuditLogConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger_create() {
        let config = AuditLogConfig {
            enabled: false,
            ..Default::default()
        };
        let logger = AuditLogger::with_config(config);
        assert!(!logger.get_config().enabled);
    }

    #[tokio::test]
    async fn test_audit_logger_log_query() {
        let config = AuditLogConfig {
            enabled: false,
            ..Default::default()
        };
        let logger = AuditLogger::with_config(config);

        logger.log_query(
            "test_user".to_string(),
            "127.0.0.1".to_string(),
            Some("test_db".to_string()),
            "SELECT * FROM users".to_string(),
            QueryStatus::Success,
            100,
            Some(10),
            Some(1024),
            None,
        ).await;

        logger.flush().await;
    }

    #[test]
    fn test_query_type_from_sql() {
        assert!(matches!(QueryType::from_sql("SELECT * FROM users"), QueryType::Select));
        assert!(matches!(QueryType::from_sql("INSERT INTO users VALUES (1)"), QueryType::Insert));
        assert!(matches!(QueryType::from_sql("UPDATE users SET name='test'"), QueryType::Update));
        assert!(matches!(QueryType::from_sql("DELETE FROM users"), QueryType::Delete));
        assert!(matches!(QueryType::from_sql("CREATE DATABASE test"), QueryType::CreateDatabase));
        assert!(matches!(QueryType::from_sql("DROP DATABASE test"), QueryType::DropDatabase));
        assert!(matches!(QueryType::from_sql("CREATE TABLE test (id INT)"), QueryType::CreateTable));
        assert!(matches!(QueryType::from_sql("DROP TABLE test"), QueryType::DropTable));
        assert!(matches!(QueryType::from_sql("ALTER TABLE test ADD COLUMN name VARCHAR"), QueryType::AlterTable));
        assert!(matches!(QueryType::from_sql("SHOW DATABASES"), QueryType::ShowDatabases));
        assert!(matches!(QueryType::from_sql("SHOW TABLES"), QueryType::ShowTables));
        assert!(matches!(QueryType::from_sql("UNKNOWN QUERY"), QueryType::Other));
    }

    #[test]
    fn test_query_type_from_sql_case_insensitive() {
        assert!(matches!(QueryType::from_sql("select * from users"), QueryType::Select));
        assert!(matches!(QueryType::from_sql("Select * From Users"), QueryType::Select));
        assert!(matches!(QueryType::from_sql("INSERT INTO users VALUES (1)"), QueryType::Insert));
        assert!(matches!(QueryType::from_sql("insert into users values (1)"), QueryType::Insert));
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}
