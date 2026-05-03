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
        let path = format!("{}/edit_log_{}.json", self.log_path, self.last_applied_index);
        let mut file = tokio::fs::File::create(&path).await?;
        for entry in &self.entries {
            let line = serde_json::to_string(entry)
                .map_err(|e| common::DrorisError::Internal(e.to_string()))?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        self.last_applied_index = self.entries.last().map(|e| e.index).unwrap_or(self.last_applied_index);
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