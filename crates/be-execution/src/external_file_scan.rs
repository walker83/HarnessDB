use async_trait::async_trait;
use common::Result;
use types::{Block, Schema, Vector};
use data_io::ParquetReader;
use std::path::PathBuf;
use std::sync::Arc;

use super::ExecNode;

pub trait AsyncBytesRead: tokio::io::AsyncRead + Send {}
impl<T: tokio::io::AsyncRead + Send> AsyncBytesRead for T {}

pub trait ExternalFileSystem: Send + Sync {
    async fn open(&self, path: &str) -> Result<Box<dyn AsyncBytesRead + Send>>;
    async fn list_files(&self, path: &str) -> Result<Vec<String>>;
}

pub struct ExternalFileScanExecNode {
    pub catalog_name: Option<String>,
    pub database: String,
    pub table_name: String,
    pub location: String,
    pub file_format: String,
    pub columns: Vec<String>,
    predicates: Vec<String>,
    limit: Option<usize>,
    fs: Option<Arc<dyn ExternalFileSystem>>,
    reader: Option<ParquetReader>,
    opened: bool,
}

impl ExternalFileScanExecNode {
    pub fn new(
        catalog_name: Option<String>,
        database: String,
        table_name: String,
        location: String,
        file_format: String,
        columns: Vec<String>,
    ) -> Self {
        Self {
            catalog_name,
            database,
            table_name,
            location,
            file_format,
            columns,
            predicates: Vec::new(),
            limit: None,
            fs: None,
            reader: None,
            opened: false,
        }
    }

    pub fn with_predicates(mut self, predicates: Vec<String>) -> Self {
        self.predicates = predicates;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_fs(mut self, fs: Arc<dyn ExternalFileSystem>) -> Self {
        self.fs = Some(fs);
        self
    }

    async fn init_reader(&mut self) -> Result<()> {
        if self.reader.is_some() {
            return Ok(());
        }

        let path = PathBuf::from(&self.location);
        let reader = ParquetReader::open(path)?;
        self.reader = Some(reader);
        Ok(())
    }
}

#[async_trait]
impl ExecNode for ExternalFileScanExecNode {
    async fn open(&mut self) -> Result<()> {
        self.init_reader().await?;
        self.opened = true;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if !self.opened {
            return Err(common::DrorisError::Internal("ExternalFileScanExecNode not opened".to_string()));
        }

        if let Some(ref mut reader) = self.reader {
            match reader.next_batch()? {
                Some(block) => {
                    if let Some(limit) = self.limit {
                        if block.num_rows() > limit {
                            return Ok(Some(block.slice(0, limit)));
                        }
                    }
                    Ok(Some(block))
                }
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.opened = false;
        self.reader = None;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct MockExternalFileSystem {
    pub mock_data: Vec<u8>,
}

impl MockExternalFileSystem {
    pub fn new(mock_data: Vec<u8>) -> Self {
        Self { mock_data }
    }
}

#[async_trait]
impl ExternalFileSystem for MockExternalFileSystem {
    async fn open(&self, _path: &str) -> Result<Box<dyn AsyncBytesRead + Send>> {
        use tokio::io::AsyncReadExt;

        struct MockReader {
            data: std::io::Cursor<Vec<u8>>,
        }

        impl tokio::io::AsyncRead for MockReader {
            fn poll_read(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(self.data.read(buf.remaining_mut()).map(|n| {
                    buf.advance(n);
                }))
            }
        }

        Ok(Box::new(MockReader {
            data: std::io::Cursor::new(self.mock_data.clone()),
        }))
    }

    async fn list_files(&self, _path: &str) -> Result<Vec<String>> {
        Ok(vec!["mock_file.parquet".to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_external_fs() {
        let data = vec![1, 2, 3, 4, 5];
        let fs = MockExternalFileSystem::new(data);
        let files = fs.list_files("/test").await.unwrap();
        assert_eq!(files.len(), 1);
    }
}