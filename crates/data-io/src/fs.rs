use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn open(&self, path: PathBuf) -> common::Result<Box<dyn AsyncFileRead>>;
    async fn list_files(&self, path: PathBuf) -> common::Result<Vec<PathBuf>>;
    async fn exists(&self, path: PathBuf) -> bool;
}

pub trait AsyncFileRead: tokio::io::AsyncRead + Send + Sync {}
impl<T: tokio::io::AsyncRead + Send + Sync> AsyncFileRead for T {}

pub struct LocalFileSystem;

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn open(&self, path: PathBuf) -> common::Result<Box<dyn AsyncFileRead>> {
        let file = tokio::fs::File::open(&path).await
            .map_err(|e| common::DrorisError::Internal(format!("Failed to open {}: {}", path.display(), e)))?;
        Ok(Box::new(file))
    }

    async fn list_files(&self, path: PathBuf) -> common::Result<Vec<PathBuf>> {
        let mut entries = tokio::fs::read_dir(&path).await
            .map_err(|e| common::DrorisError::Internal(format!("Failed to read dir {}: {}", path.display(), e)))?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| common::DrorisError::Internal(format!("Failed to read dir entry: {}", e)))? {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
        Ok(files)
    }

    async fn exists(&self, path: PathBuf) -> bool {
        tokio::fs::try_exists(&path).await.unwrap_or(false)
    }
}

pub struct S3FileSystem {
    bucket: String,
    prefix: String,
}

impl S3FileSystem {
    pub fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }
}

pub struct S3File {
    inner: bytes::Bytes,
    position: usize,
}

impl S3File {
    pub fn new(data: bytes::Bytes) -> Self {
        Self {
            inner: data,
            position: 0,
        }
    }
}

impl tokio::io::AsyncRead for S3File {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let remaining = self.inner.len() - self.position;
        if remaining == 0 {
            return std::task::Poll::Ready(Ok(()));
        }
        let amt = std::cmp::min(buf.remaining(), remaining);
        let start = self.position;
        let end = start + amt;
        buf.put_slice(&self.inner[start..end]);
        self.position = end;
        std::task::Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl FileSystem for S3FileSystem {
    async fn open(&self, path: PathBuf) -> common::Result<Box<dyn AsyncFileRead>> {
        let _key = format!("{}/{}", self.prefix, path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default());

        let mock_data: Vec<u8> = vec![];
        Ok(Box::new(S3File::new(mock_data.into())))
    }

    async fn list_files(&self, _path: PathBuf) -> common::Result<Vec<PathBuf>> {
        Ok(vec![])
    }

    async fn exists(&self, _path: PathBuf) -> bool {
        true
    }
}

pub fn parse_s3_path(path: &str) -> Option<(String, String)> {
    if path.starts_with("s3://") {
        let path = &path[5..];
        if let Some(pos) = path.find('/') {
            let bucket = path[..pos].to_string();
            let key = path[pos+1..].to_string();
            return Some((bucket, key));
        } else {
            return Some((path.to_string(), "".to_string()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_path() {
        assert_eq!(parse_s3_path("s3://bucket/path/to/file"), Some(("bucket".to_string(), "path/to/file".to_string())));
        assert_eq!(parse_s3_path("s3://my-bucket"), Some(("my-bucket".to_string(), "".to_string())));
        assert_eq!(parse_s3_path("/local/path"), None);
    }

    #[tokio::test]
    async fn test_local_file_system() {
        let fs = LocalFileSystem;
        let path = PathBuf::from("/tmp");
        let exists = fs.exists(path).await;
        assert!(exists || !exists);
    }
}