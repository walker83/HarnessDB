//! Vector database HTTP server

use crate::handler::VectorHandler;
use crate::storage::VectorStorage;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};

pub struct VectorServer {
    port: u16,
    storage: Arc<VectorStorage>,
}

impl VectorServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            storage: Arc::new(VectorStorage::new()),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("Vector database server listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("Vector client connected: {}", addr);

            let storage = self.storage.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, storage).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(stream: TcpStream, storage: Arc<VectorStorage>) -> anyhow::Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let handler = VectorHandler::new(storage);

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break;
            }

            // Simple protocol: METHOD PATH\nBODY\n
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let method = parts[0];
                let path = parts[1];

                // Read body
                let mut body = String::new();
                reader.read_line(&mut body).await?;

                let response = handler.handle_request(method, path, &body);
                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
        }

        Ok(())
    }
}
