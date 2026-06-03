use serde::{Deserialize, Serialize};

pub struct EditLog {
    /// In-memory log entries pending flush
    entries: Vec<EditLogEntry>,
    /// Last flushed log index
    last_applied_index: u64,
    /// Current term number
    current_term: u64,
    /// Voted for candidate
    voted_for: Option<u64>,
    /// Path to log file
    log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditLogEntry {
    pub term: u64,
    pub index: u64,
    pub op_type: OpType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpType {
    // Catalog ops
    CreateDatabase,
    DropDatabase,
    CreateTable,
    DropTable,
    AlterDatabase,
    AlterTable,
    // Tablet ops
    CreateTablet,
    DropTablet,
    AlterTablet,
    // Node ops
    AddBackend,
    RemoveBackend,
    // Stats ops
    UpdateStats,
}

impl EditLog {
    pub fn new(log_dir: impl Into<String>) -> Self {
        Self {
            entries: Vec::new(),
            last_applied_index: 0,
            current_term: 0,
            voted_for: None,
            log_path: log_dir.into(),
        }
    }

    /// Append a new log entry
    pub fn append(&mut self, op_type: OpType, data: Vec<u8>) -> u64 {
        self.current_term += 1;
        let index = self.entries.len() as u64 + self.last_applied_index + 1;
        let entry = EditLogEntry {
            term: self.current_term,
            index,
            op_type,
            data,
        };
        self.entries.push(entry.clone());
        entry.index
    }

    /// Flush entries to disk (serde JSON format, one JSON per line)
    pub async fn flush(&mut self) -> common::Result<()> {
        use tokio::io::AsyncWriteExt;
        if self.entries.is_empty() {
            return Ok(());
        }
        let path = format!(
            "{}/edit_log_{}.json",
            self.log_path, self.last_applied_index
        );
        let mut file = tokio::fs::File::create(&path).await?;
        for entry in &self.entries {
            let line = serde_json::to_string(entry)
                .map_err(|e| common::DharnessError::Internal(e.to_string()))?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        // Ensure edit log is flushed to disk (critical for durability)
        file.sync_all().await.map_err(|e| {
            common::DharnessError::Internal(format!("Failed to sync edit log: {}", e))
        })?;
        self.last_applied_index = self
            .entries
            .last()
            .map(|e| e.index)
            .unwrap_or(self.last_applied_index);
        self.entries.clear();
        Ok(())
    }

    /// Replay entries from disk on startup
    pub async fn replay(&mut self) -> common::Result<()> {
        use tokio::fs;
        let dir = fs::read_dir(&self.log_path).await;
        let Ok(dir_stream) = dir else { return Ok(()) };
        let mut paths: Vec<tokio::fs::DirEntry> = Vec::new();
        let mut stream = tokio_stream::wrappers::ReadDirStream::new(dir_stream);
        while let Some(entry) = tokio_stream::StreamExt::next(&mut stream).await {
            if let Ok(e) = entry {
                paths.push(e);
            }
        }
        paths.sort_by_key(|p| p.file_name());
        for path_entry in paths {
            let path = path_entry.path();
            let data: Vec<u8> = fs::read(&path).await?;
            for line in data.split(|&b| b == b'\n') {
                if line.is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_slice::<EditLogEntry>(line) {
                    self.last_applied_index = entry.index;
                    self.entries.push(entry);
                }
            }
        }
        Ok(())
    }

    pub fn get_entry(&self, index: u64) -> Option<&EditLogEntry> {
        self.entries.iter().find(|e| e.index == index)
    }

    pub fn last_applied_index(&self) -> u64 {
        self.last_applied_index
    }

    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    pub fn voted_for(&self) -> Option<u64> {
        self.voted_for
    }

    /// Returns a reference to all in-memory entries
    pub fn entries(&self) -> &[EditLogEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_log_append() {
        let mut log = EditLog::new("/tmp/rovisdb_test_edit_log");
        let idx = log.append(OpType::CreateDatabase, b"mydb".to_vec());
        assert_eq!(idx, 1);
        assert_eq!(log.current_term(), 1);
        assert_eq!(log.entries().len(), 1);
    }

    #[test]
    fn test_edit_log_multiple_appends() {
        let mut log = EditLog::new("/tmp/rovisdb_test_edit_log");
        log.append(OpType::CreateDatabase, b"db1".to_vec());
        log.append(OpType::CreateTable, b"t1".to_vec());
        log.append(OpType::DropDatabase, b"db1".to_vec());
        assert_eq!(log.entries().len(), 3);
        assert_eq!(log.current_term(), 3);
    }

    #[test]
    fn test_edit_log_get_entry() {
        let mut log = EditLog::new("/tmp/rovisdb_test_edit_log");
        log.append(OpType::CreateDatabase, b"mydb".to_vec());
        let entry = log.get_entry(1);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.op_type, OpType::CreateDatabase);
        assert_eq!(entry.data, b"mydb");
    }

    #[test]
    fn test_edit_log_get_nonexistent() {
        let log = EditLog::new("/tmp/rovisdb_test_edit_log");
        assert!(log.get_entry(999).is_none());
    }

    #[test]
    fn test_edit_log_empty() {
        let log = EditLog::new("/tmp/rovisdb_test_edit_log");
        assert_eq!(log.entries().len(), 0);
        assert_eq!(log.last_applied_index(), 0);
        assert_eq!(log.current_term(), 0);
        assert!(log.voted_for().is_none());
    }

    #[tokio::test]
    async fn test_edit_log_flush_and_replay() {
        let dir = format!("/tmp/rovisdb_test_flush_{}", std::process::id());
        let _ = std::fs::create_dir_all(&dir);

        // Write entries
        {
            let mut log = EditLog::new(&dir);
            log.append(OpType::CreateDatabase, b"testdb".to_vec());
            log.append(OpType::CreateTable, b"test_table".to_vec());
            log.flush().await.unwrap();
            assert_eq!(log.entries().len(), 0); // cleared after flush
        }

        // Replay
        {
            let mut log = EditLog::new(&dir);
            log.replay().await.unwrap();
            assert!(log.last_applied_index() > 0);
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_op_type_variants() {
        assert_eq!(OpType::CreateDatabase, OpType::CreateDatabase);
        assert_ne!(OpType::CreateDatabase, OpType::DropDatabase);
    }

    #[test]
    fn test_edit_log_entry_serialization() {
        let entry = EditLogEntry {
            term: 1,
            index: 5,
            op_type: OpType::CreateTable,
            data: b"hello".to_vec(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("CreateTable"));
        let decoded: EditLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.term, 1);
        assert_eq!(decoded.index, 5);
        assert_eq!(decoded.op_type, OpType::CreateTable);
        assert_eq!(decoded.data, b"hello");
    }
}
