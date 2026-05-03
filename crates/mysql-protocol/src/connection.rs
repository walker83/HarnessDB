use bytes::{Buf, Bytes, BytesMut};
use rand::RngCore;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use crate::packet::{
    self, command, column_type, CapabilityFlags, Column, HandshakeResponse, HandshakeV10,
};
use crate::server::{QueryHandler, QueryResult};

/// The connection state machine phases.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Handshake,
    Auth,
    Command,
    Closed,
}

/// Represents a single MySQL client connection.
pub struct Connection {
    stream: TcpStream,
    conn_id: u32,
    handler: Arc<dyn QueryHandler>,
    seq_id: u8,
    phase: Phase,
    capability_flags: u32,
    charset: u8,
    username: String,
    database: Option<String>,
    /// Prepared statements: stmt_id -> (sql, num_params, num_columns)
    prepared_statements: Vec<(String, u16, u16)>,
    next_stmt_id: u32,
    read_buf: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream, conn_id: u32, handler: Arc<dyn QueryHandler>) -> Self {
        Self {
            stream,
            conn_id,
            handler,
            seq_id: 0,
            phase: Phase::Handshake,
            capability_flags: 0,
            charset: 0,
            username: String::new(),
            database: None,
            prepared_statements: Vec::new(),
            next_stmt_id: 1,
            read_buf: BytesMut::with_capacity(16 * 1024),
        }
    }

    /// Run the connection through all phases until closed.
    pub async fn run(&mut self) -> std::io::Result<()> {
        // Phase 1: Send handshake
        self.send_handshake().await?;

        // Phase 2: Receive auth response
        self.handle_auth_response().await?;

        // Phase 3: Command loop
        self.command_loop().await
    }

    // -----------------------------------------------------------------------
    // Handshake phase
    // -----------------------------------------------------------------------

    async fn send_handshake(&mut self) -> std::io::Result<()> {
        let mut salt = [0u8; 20];
        rand::thread_rng().fill_bytes(&mut salt);
        let handshake = HandshakeV10::new(self.conn_id, salt);
        let packet = handshake.encode(self.seq_id);
        self.seq_id = self.seq_id.wrapping_add(1);
        self.write_all(&packet).await
    }

    // -----------------------------------------------------------------------
    // Auth phase
    // -----------------------------------------------------------------------

    async fn handle_auth_response(&mut self) -> std::io::Result<()> {
        let payload = self.read_packet().await?;
        let response = HandshakeResponse::parse(&payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.capability_flags = response.capability_flags;
        self.charset = response.charset;
        self.username = response.username;
        self.database = response.database;

        debug!(
            "Auth: user={}, db={:?}, charset={}",
            self.username,
            self.database,
            crate::charset::charset_name(self.charset)
        );

        // For now, accept any auth. Send OK.
        self.send_ok(0, 0).await?;
        self.phase = Phase::Command;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Command loop
    // -----------------------------------------------------------------------

    async fn command_loop(&mut self) -> std::io::Result<()> {
        loop {
            let payload = match self.read_packet().await {
                Ok(p) => p,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("Client disconnected");
                        break;
                    }
                    error!("Read error: {}", e);
                    break;
                }
            };

            if payload.is_empty() {
                warn!("Empty command packet");
                continue;
            }

            let cmd = payload[0];
            let data = &payload[1..];

            debug!("Command: {:#x} ({} bytes payload)", cmd, data.len());

            match cmd {
                command::COM_QUIT => {
                    info!("COM_QUIT");
                    let _ = self.send_ok(0, 0).await;
                    break;
                }
                command::COM_QUERY => {
                    let sql = String::from_utf8_lossy(data).to_string();
                    debug!("COM_QUERY: {}", sql);
                    self.handle_query(&sql).await?;
                }
                command::COM_PING => {
                    debug!("COM_PING");
                    self.send_ok(0, 0).await?;
                }
                command::COM_INIT_DB => {
                    let db = String::from_utf8_lossy(data).to_string();
                    debug!("COM_INIT_DB: {}", db);
                    self.database = Some(db);
                    self.send_ok(0, 0).await?;
                }
                command::COM_FIELD_LIST => {
                    debug!("COM_FIELD_LIST (returning empty)");
                    // Send EOF
                    self.write_all(&packet::make_eof_packet(
                        self.seq_id,
                        0,
                        packet::SERVER_STATUS_AUTOCOMMIT,
                    ))
                    .await?;
                    self.seq_id = self.seq_id.wrapping_add(1);
                }
                command::COM_STMT_PREPARE => {
                    let sql = String::from_utf8_lossy(data).to_string();
                    debug!("COM_STMT_PREPARE: {}", sql);
                    self.handle_stmt_prepare(&sql).await?;
                }
                command::COM_STMT_EXECUTE => {
                    debug!("COM_STMT_EXECUTE");
                    self.handle_stmt_execute(data).await?;
                }
                command::COM_STMT_CLOSE => {
                    debug!("COM_STMT_CLOSE");
                    self.handle_stmt_close(data);
                }
                command::COM_STATISTICS => {
                    debug!("COM_STATISTICS");
                    let stats = b"Uptime: 0  Threads: 1  Questions: 0  Slow queries: 0  Opens: 0  Flush tables: 0  Open tables: 0  Queries per second avg: 0.000";
                    let mut pb = packet::PacketBuilder::new(self.seq_id);
                    pb.put_slice(stats);
                    let (pkt, next) = pb.finish();
                    self.write_all(&pkt).await?;
                    self.seq_id = next;
                }
                _ => {
                    warn!("Unsupported command: {:#x}", cmd);
                    self.send_general_err(1047, format!("Unknown command {:#x}", cmd))
                        .await?;
                }
            }
        }

        self.phase = Phase::Closed;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Query handling
    // -----------------------------------------------------------------------

    async fn handle_query(&mut self, sql: &str) -> std::io::Result<()> {
        // Check for special commands that some clients send
        let trimmed = sql.trim().to_lowercase();

        // Handle SET commands as no-ops (commonly sent by mysql client)
        if trimmed.starts_with("set ") || trimmed.starts_with("use ") {
            if trimmed.starts_with("use ") {
                let db = sql.trim()[4..].trim().trim_end_matches(';').trim().to_string();
                self.database = Some(db);
            }
            self.send_ok(0, 0).await?;
            return Ok(());
        }

        let result = self.handler.handle_query(sql);
        self.send_result_set(result).await
    }

    // -----------------------------------------------------------------------
    // Result set encoding
    // -----------------------------------------------------------------------

    async fn send_result_set(&mut self, result: QueryResult) -> std::io::Result<()> {
        // If no columns, it's an OK-style result (e.g. DDL)
        if result.columns.is_empty() {
            self.send_ok(0, 0).await?;
            return Ok(());
        }

        let use_eof = (self.capability_flags & CapabilityFlags::DEPRECATE_EOF) == 0;

        // 1. Column count packet (lenenc int)
        let mut pb = packet::PacketBuilder::new(self.seq_id);
        pb.lenenc_int(result.columns.len() as u64);
        let (pkt, next) = pb.finish();
        self.write_all(&pkt).await?;
        self.seq_id = next;

        // 2. Column definition packets
        let columns: Vec<Column> = result.columns.iter().map(|c| c.into()).collect();
        for col in &columns {
            let pkt = col.encode(self.seq_id);
            self.write_all(&pkt).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        // 3. EOF (if not deprecated)
        if use_eof {
            let eof = packet::make_eof_packet(self.seq_id, 0, packet::SERVER_STATUS_AUTOCOMMIT);
            self.write_all(&eof).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        // 4. Row data packets
        for row in &result.rows {
            let values: Vec<Option<Vec<u8>>> = row
                .iter()
                .map(|v| v.as_ref().map(|s| s.as_bytes().to_vec()))
                .collect();
            let pkt = packet::encode_text_row(self.seq_id, &values);
            self.write_all(&pkt).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        // 5. Final EOF or OK
        if use_eof {
            let eof = packet::make_eof_packet(self.seq_id, 0, packet::SERVER_STATUS_AUTOCOMMIT);
            self.write_all(&eof).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        } else {
            let ok = packet::make_ok_packet(
                self.seq_id,
                result.rows.len() as u64,
                0,
                packet::SERVER_STATUS_AUTOCOMMIT,
                0,
            );
            self.write_all(&ok).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Prepared statement handling
    // -----------------------------------------------------------------------

    async fn handle_stmt_prepare(&mut self, sql: &str) -> std::io::Result<()> {
        // For now, parse the statement but don't actually bind parameters.
        // We store it and assign a statement ID.
        let stmt_id = self.next_stmt_id;
        self.next_stmt_id += 1;

        // Parse placeholders (?) for parameter count
        let num_params = count_placeholders(sql) as u16;

        // Execute the query to determine columns (with empty params)
        let result = self.handler.handle_query(sql);
        let num_columns = result.columns.len() as u16;

        // Store the statement
        self.prepared_statements
            .push((sql.to_string(), num_params, num_columns));

        let use_eof = (self.capability_flags & CapabilityFlags::DEPRECATE_EOF) == 0;

        // Send COM_STMT_PREPARE_OK
        let pkt = packet::make_stmt_prepare_ok(self.seq_id, stmt_id, num_columns, num_params, 0);
        self.write_all(&pkt).await?;
        self.seq_id = self.seq_id.wrapping_add(1);

        // Send parameter column definitions (if any)
        for i in 0..num_params {
            let col = Column::new(&format!("param_{}", i), column_type::VAR_STRING);
            let pkt = col.encode(self.seq_id);
            self.write_all(&pkt).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        if num_params > 0 && use_eof {
            let eof = packet::make_eof_packet(self.seq_id, 0, packet::SERVER_STATUS_AUTOCOMMIT);
            self.write_all(&eof).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        // Send result column definitions (if any)
        for col_def in &result.columns {
            let col: Column = col_def.into();
            let pkt = col.encode(self.seq_id);
            self.write_all(&pkt).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        if num_columns > 0 && use_eof {
            let eof = packet::make_eof_packet(self.seq_id, 0, packet::SERVER_STATUS_AUTOCOMMIT);
            self.write_all(&eof).await?;
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        Ok(())
    }

    async fn handle_stmt_execute(&mut self, data: &[u8]) -> std::io::Result<()> {
        if data.len() < 9 {
            self.send_general_err(1045, "Malformed COM_STMT_EXECUTE".to_string()).await?;
            return Ok(());
        }

        // Parse statement ID (4 bytes LE)
        let stmt_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if stmt_id == 0 || stmt_id > self.prepared_statements.len() {
            self.send_general_err(1045, "Unknown prepared statement".to_string())
                .await?;
            return Ok(());
        }

        let (sql, _num_params, _num_cols) = &self.prepared_statements[stmt_id - 1];

        // For simplicity, just execute the SQL directly (ignoring bound params)
        let result = self.handler.handle_query(sql);
        self.send_result_set(result).await
    }

    fn handle_stmt_close(&mut self, data: &[u8]) {
        if data.len() >= 4 {
            let stmt_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if stmt_id > 0 && stmt_id <= self.prepared_statements.len() {
                self.prepared_statements[stmt_id - 1] = (String::new(), 0, 0);
            }
        }
        // COM_STMT_CLOSE has no response
    }

    // -----------------------------------------------------------------------
    // Packet I/O helpers
    // -----------------------------------------------------------------------

    /// Read one complete MySQL packet from the stream.
    /// Returns just the payload (without header).
    async fn read_packet(&mut self) -> std::io::Result<Bytes> {
        loop {
            // Try to parse a complete packet from the buffer
            if self.read_buf.len() >= 4 {
                let (payload_len, _seq) = packet::read_packet_header(&self.read_buf)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "bad header")
                    })?;

                let total = 4 + payload_len;
                if self.read_buf.len() >= total {
                    let _seq_id = self.read_buf[3];
                    // Extract payload bytes
                    let payload: Bytes = self.read_buf[4..total].to_vec().into();
                    // Remove consumed bytes from the buffer: advance past the full packet
                    self.read_buf.advance(total);
                    return Ok(payload);
                }
            }

            // Need more data
            let n = self.stream.read_buf(&mut self.read_buf).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "connection closed",
                ));
            }
        }
    }

    /// Send an OK packet.
    async fn send_ok(
        &mut self,
        affected_rows: u64,
        last_insert_id: u64,
    ) -> std::io::Result<()> {
        let pkt = packet::make_ok_packet(
            self.seq_id,
            affected_rows,
            last_insert_id,
            packet::SERVER_STATUS_AUTOCOMMIT,
            0,
        );
        self.write_all(&pkt).await?;
        self.seq_id = self.seq_id.wrapping_add(1);
        Ok(())
    }

    /// Send a general error packet.
    async fn send_general_err(
        &mut self,
        error_code: u16,
        message: String,
    ) -> std::io::Result<()> {
        let pkt = packet::make_general_err(self.seq_id, error_code, &message);
        self.write_all(&pkt).await?;
        self.seq_id = self.seq_id.wrapping_add(1);
        Ok(())
    }

    /// Write all bytes to the stream.
    async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.stream.write_all(data).await?;
        self.stream.flush().await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count `?` placeholders in a SQL string (simple, non-recursive).
fn count_placeholders(sql: &str) -> usize {
    let mut count = 0;
    let mut in_string = false;
    let mut string_char = b'\0';
    let mut prev = b'\0';

    for &b in sql.as_bytes() {
        if in_string {
            if b == string_char && prev != b'\\' {
                in_string = false;
            }
        } else if b == b'\'' || b == b'"' {
            in_string = true;
            string_char = b;
        } else if b == b'?' {
            count += 1;
        }
        prev = b;
    }
    count
}
