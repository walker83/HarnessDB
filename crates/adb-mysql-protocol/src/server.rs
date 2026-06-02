//! AnalyticDB MySQL TCP server (MySQL-compatible with MPP extensions)

use crate::handler::AdbMysqlHandler;
use crate::storage::AdbMysqlStorage;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};

pub struct AdbMysqlServer {
    port: u16,
    storage: Arc<AdbMysqlStorage>,
}

impl AdbMysqlServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            storage: Arc::new(AdbMysqlStorage::new()),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("AnalyticDB MySQL server listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("AnalyticDB MySQL client connected: {}", addr);

            let storage = self.storage.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, storage).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(stream: TcpStream, storage: Arc<AdbMysqlStorage>) -> anyhow::Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let handler = AdbMysqlHandler::new(storage);
        let mut current_db = "default".to_string();

        // Send greeting
        writer.write_all(b"Welcome to AnalyticDB for MySQL\n").await?;
        writer.flush().await?;

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break;
            }

            let query = line.trim();
            if query.is_empty() {
                continue;
            }

            if query.to_uppercase().starts_with("USE ") {
                current_db = query[4..].trim().trim_end_matches(';').to_string();
                writer.write_all(b"OK\n").await?;
            } else {
                let result = handler.handle_query(&current_db, query);
                writer.write_all(result.as_bytes()).await?;
            }
            writer.flush().await?;
        }

        Ok(())
    }
}
