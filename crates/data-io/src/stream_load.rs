use common::Result;
use std::collections::HashMap;

/// Stream load format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadFormat {
    Csv,
    Json,
}

impl LoadFormat {
    #[allow(should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// Result of a stream load operation
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// Number of rows successfully loaded
    pub rows_loaded: u64,
    /// Number of rows with errors
    pub errors: u64,
    /// First error message if any
    pub first_error: Option<String>,
}

impl LoadResult {
    pub fn new(rows_loaded: u64, errors: u64, first_error: Option<String>) -> Self {
        Self { rows_loaded, errors, first_error }
    }

    pub fn success(rows_loaded: u64) -> Self {
        Self { rows_loaded, errors: 0, first_error: None }
    }

    pub fn failure(error: String) -> Self {
        Self { rows_loaded: 0, errors: 1, first_error: Some(error) }
    }

    pub fn is_success(&self) -> bool {
        self.errors == 0
    }
}

/// Stream load handler for bulk data import
pub struct StreamLoad {
    db_name: String,
    table_name: String,
    timeout_secs: u64,
    headers: HashMap<String, String>,
}

impl StreamLoad {
    pub fn new(db: &str, table: &str) -> Self {
        Self {
            db_name: db.to_string(),
            table_name: table.to_string(),
            timeout_secs: 3600,
            headers: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Load data with the specified format
    /// In a real implementation, this would send the data to the server
    /// via the RPC layer or HTTP API
    pub async fn load(&self, data: Vec<u8>, format: LoadFormat) -> Result<LoadResult> {
        tracing::info!(
            "StreamLoad: db={}, table={}, format={}, size={} bytes",
            self.db_name,
            self.table_name,
            format.as_str(),
            data.len()
        );

        // Validate format
        if format != LoadFormat::Csv && format != LoadFormat::Json {
            return Ok(LoadResult::failure(format!("Unsupported format: {:?}", format)));
        }

        // Parse and validate the data
        

        // In a real implementation, this would:
        // 1. Connect to the server via RPC
        // 2. Send the data in chunks
        // 3. Handle partial responses
        // 4. Retry on failures

        match format {
            LoadFormat::Csv => self.load_csv(data).await,
            LoadFormat::Json => self.load_json(data).await,
        }
    }

    async fn load_csv(&self, data: Vec<u8>) -> Result<LoadResult> {
        use std::io::Cursor;

        let cursor = Cursor::new(data);
        let mut reader = super::csv_reader::CsvReader::new(cursor);

        let mut total_rows: u64 = 0;
        let mut error_count: u64 = 0;
        let mut first_err: Option<String> = None;

        loop {
            match reader.next_batch() {
                Ok(Some(_block)) => {
                    // In a real implementation, send block to server
                    total_rows += _block.num_rows() as u64;
                }
                Ok(None) => break,
                Err(e) => {
                    error_count += 1;
                    if first_err.is_none() {
                        first_err = Some(format!("CSV parse error: {}", e));
                    }
                    break;
                }
            }
        }

        Ok(LoadResult::new(total_rows, error_count, first_err))
    }

    async fn load_json(&self, data: Vec<u8>) -> Result<LoadResult> {
        use std::io::Cursor;

        let cursor = Cursor::new(data);
        let mut reader = super::json_reader::JsonReader::new(cursor);

        let mut total_rows: u64 = 0;
        let mut error_count: u64 = 0;
        let mut first_err: Option<String> = None;

        loop {
            match reader.next_batch() {
                Ok(Some(_block)) => {
                    // In a real implementation, send block to server
                    total_rows += _block.num_rows() as u64;
                }
                Ok(None) => break,
                Err(e) => {
                    error_count += 1;
                    if first_err.is_none() {
                        first_err = Some(format!("JSON parse error: {}", e));
                    }
                    break;
                }
            }
        }

        Ok(LoadResult::new(total_rows, error_count, first_err))
    }

    /// Get the database name
    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Get the timeout in seconds
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }
}

/// Builder for StreamLoad with fluent API
pub struct StreamLoadBuilder {
    db_name: String,
    table_name: String,
    timeout_secs: u64,
    headers: HashMap<String, String>,
    format: Option<LoadFormat>,
}

impl StreamLoadBuilder {
    pub fn new(db: &str, table: &str) -> Self {
        Self {
            db_name: db.to_string(),
            table_name: table.to_string(),
            timeout_secs: 3600,
            headers: HashMap::new(),
            format: None,
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_format(mut self, format: LoadFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> StreamLoad {
        let mut load = StreamLoad::new(&self.db_name, &self.table_name)
            .with_timeout(self.timeout_secs);

        for (k, v) in self.headers {
            load = load.with_header(&k, &v);
        }

        load
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_format_from_str() {
        assert_eq!(LoadFormat::from_str("csv"), Some(LoadFormat::Csv));
        assert_eq!(LoadFormat::from_str("CSV"), Some(LoadFormat::Csv));
        assert_eq!(LoadFormat::from_str("json"), Some(LoadFormat::Json));
        assert_eq!(LoadFormat::from_str("unknown"), None);
    }

    #[test]
    fn test_load_result_success() {
        let result = LoadResult::success(100);
        assert!(result.is_success());
        assert_eq!(result.rows_loaded, 100);
        assert_eq!(result.errors, 0);
        assert!(result.first_error.is_none());
    }

    #[test]
    fn test_load_result_failure() {
        let result = LoadResult::failure("test error".to_string());
        assert!(!result.is_success());
        assert_eq!(result.rows_loaded, 0);
        assert_eq!(result.errors, 1);
        assert_eq!(result.first_error, Some("test error".to_string()));
    }

    #[test]
    fn test_stream_load_builder() {
        let load = StreamLoadBuilder::new("mydb", "mytable")
            .with_timeout(7200)
            .with_format(LoadFormat::Csv)
            .with_header("Authorization", "Bearer token")
            .build();

        assert_eq!(load.db_name(), "mydb");
        assert_eq!(load.table_name(), "mytable");
        assert_eq!(load.timeout_secs(), 7200);
    }
}