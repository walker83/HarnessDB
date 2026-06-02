//! MongoDB TCP connection handler

use crate::handler::MongoDBCommandHandler;
use crate::wire::Message;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

/// MongoDB connection handler
pub struct MongoDBConnection {
    stream: TcpStream,
    read_buf: BytesMut,
    handler: Arc<dyn MongoDBCommandHandler>,
    conn_id: u32,
}

impl MongoDBConnection {
    pub fn new(
        stream: TcpStream,
        handler: Arc<dyn MongoDBCommandHandler>,
        conn_id: u32,
    ) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            handler,
            conn_id,
        }
    }

    /// Run the connection loop
    pub async fn run(mut self) -> std::io::Result<()> {
        info!("MongoDB connection {} established", self.conn_id);

        loop {
            // Read data from socket
            let n = match self.stream.read_buf(&mut self.read_buf).await {
                Ok(0) => {
                    info!("MongoDB connection {} closed", self.conn_id);
                    return Ok(());
                }
                Ok(n) => n,
                Err(e) => {
                    error!("MongoDB connection {} read error: {}", self.conn_id, e);
                    return Err(e);
                }
            };

            debug!("MongoDB connection {} read {} bytes", self.conn_id, n);

            // Parse and handle messages
            while let Some(message) = Message::parse(&mut self.read_buf) {
                debug!("MongoDB connection {} message: {:?}", self.conn_id, message.header);

                // Handle message
                let response = self.handler.handle_message(&message);

                // Encode response
                let mut response_buf = BytesMut::new();
                response.encode(&mut response_buf);

                // Write response
                self.stream.write_all(&response_buf).await?;
            }
        }
    }
}
