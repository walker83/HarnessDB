use bytes::{Buf, Bytes, BytesMut};
use rand::RngCore;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::auth::{AuthError, AuthUser};
use crate::packet::{
    self, command, column_type, CapabilityFlags, Column, HandshakeResponse, HandshakeV10,
};
use crate::server::{QueryHandler, QueryResult, ColumnDef, ColumnType};

/// The connection state machine phases.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Handshake,
    #[allow(dead_code)]
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
    auth_user: Option<AuthUser>,
    /// The 20-byte auth salt sent in handshake (used for AuthSwitchRequest)
    auth_salt: [u8; 20],
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
            auth_user: None,
            auth_salt: [0u8; 20],
            prepared_statements: Vec::new(),
            next_stmt_id: 1,
            read_buf: BytesMut::with_capacity(16 * 1024),
        }
    }

    /// Run the connection through all phases until closed.
    pub async fn run(&mut self) -> std::io::Result<()> {
        // Phase 1: Send handshake
        self.send_handshake().await?;

        // Phase 2: Receive auth response (with timeout)
        let auth_result = timeout(Duration::from_secs(1), self.handle_auth_response()).await;
        match auth_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                warn!("Auth failed for conn {}: {}", self.conn_id, e);
                return Err(e);
            }
            Err(_) => {
                warn!("Auth timeout for conn {}", self.conn_id);
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "auth timeout"));
            }
        }

        // Phase 3: Command loop
        self.command_loop().await
    }

    // -----------------------------------------------------------------------
    // Handshake phase
    // -----------------------------------------------------------------------

    async fn send_handshake(&mut self) -> std::io::Result<()> {
        let mut salt = [0u8; 20];
        rand::thread_rng().fill_bytes(&mut salt);
        self.auth_salt = salt; // Save for potential AuthSwitchRequest
        let handshake = HandshakeV10::new(self.conn_id, self.auth_salt);
        let packet = handshake.encode(self.seq_id);
        self.seq_id = self.seq_id.wrapping_add(1);
        self.write_all(&packet).await
    }

    // -----------------------------------------------------------------------
    // Auth phase
    // -----------------------------------------------------------------------

    async fn handle_auth_response(&mut self) -> std::io::Result<()> {
        let seq_before = self.seq_id;
        debug!("handle_auth_response: reading auth packet, current seq_id={}", seq_before);
        let payload = self.read_packet().await?;
        debug!("handle_auth_response: received {} bytes, seq_id now={}", payload.len(), self.seq_id);

        let response = match HandshakeResponse::parse(&payload) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse handshake response ({} bytes): {}", payload.len(), e);
                debug!("Raw payload: {:02x?}", &payload[..payload.len().min(128)]);
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
        };

        self.capability_flags = response.capability_flags & packet::DEFAULT_CAPABILITIES;
        self.charset = response.charset;
        self.username = response.username.clone();
        self.database = response.database.clone();

        debug!(
            "Auth: user={}, db={:?}, charset={}, auth_plugin={:?}, our seq_id={}",
            self.username,
            self.database,
            crate::charset::charset_name(self.charset),
            response.auth_plugin_name,
            self.seq_id
        );

        let auth_result = self.authenticate_user(
            &self.username,
            &response.auth_response,
            response.auth_plugin_name.as_deref(),
        ).await;

        match auth_result {
            Ok(auth_user) => {
                self.auth_user = Some(auth_user);
                self.send_ok(0, 0).await?;
                debug!("handle_auth_response: sent OK, seq_id now={}", self.seq_id);
                self.phase = Phase::Command;
                Ok(())
            }
            Err(auth_err) => {
                let err_msg = auth_err.to_string();
                debug!("Auth failed for user {}: {}", self.username, err_msg);
                self.send_auth_failure(1045, &err_msg).await
            }
        }
    }

    async fn authenticate_user(
        &self,
        username: &str,
        auth_response: &[u8],
        auth_plugin_name: Option<&str>,
    ) -> Result<AuthUser, AuthError> {
        use crate::auth::{NativePasswordAuth, TokenAuth, TokenConfig, AuthPlugin};

        if username.is_empty() {
            return Err(AuthError::Failed("Empty username".to_string()));
        }

        let plugin_name = auth_plugin_name.unwrap_or("mysql_native_password");

        match plugin_name {
            "mysql_native_password" => {
                let auth = NativePasswordAuth::new();
                auth.authenticate(username, auth_response, &self.auth_salt).await
            }
            "auth_token" => {
                let config = TokenConfig::new(
                    "default_secret_key".to_string(),
                    3600,
                    "rorisdb".to_string(),
                );
                let auth = TokenAuth::new(config);
                auth.authenticate(username, auth_response, &self.auth_salt).await
            }
            _ => Err(AuthError::PluginNotSupported(plugin_name.to_string())),
        }
    }

    async fn send_auth_failure(&mut self, error_code: u16, message: &str) -> std::io::Result<()> {
        let pkt = packet::make_general_err(self.seq_id, error_code, message);
        self.write_all(&pkt).await?;
        self.seq_id = self.seq_id.wrapping_add(1);
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            message,
        ))
    }

    #[allow(dead_code)]
    async fn send_auth_switch_request(&mut self) -> std::io::Result<()> {
        // Build AuthSwitchRequest packet
        // Format: 0xFE (status) + plugin name + null + auth plugin data
        let mut pb = packet::PacketBuilder::new(self.seq_id);
        pb.put_u8(0xFE); // Status byte for auth switch
        pb.put_slice(b"mysql_native_password");
        pb.put_u8(0); // null terminator
        // Auth plugin data (20 bytes auth salt)
        pb.put_slice(&self.auth_salt);
        pb.put_u8(0); // null terminator

        let (pkt, next) = pb.finish();
        self.write_all(&pkt).await?;
        self.seq_id = next;
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
                    if let Err(e) = self.handle_query(&sql).await {
                        error!("Query handler error: {}", e);
                        return Err(e);
                    }
                }
                command::COM_PING => {
                    debug!("COM_PING");
                    self.send_ok(0, 0).await?;
                }
                command::COM_INIT_DB => {
                    let db = String::from_utf8_lossy(data).to_string();
                    debug!("COM_INIT_DB: {}", db);
                    self.database = Some(db.clone());
                    self.handler.set_database(&db);
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
                command::COM_SET_OPTION => {
                    debug!("COM_SET_OPTION (ignored)");
                    self.send_ok(0, 0).await?;
                }
                command::COM_RESET_CONNECTION => {
                    debug!("COM_RESET_CONNECTION");
                    self.send_ok(0, 0).await?;
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
        if trimmed.starts_with("set ") {
            self.send_ok(0, 0).await?;
            return Ok(());
        }

        // Handle common MySQL client initialization queries
        // These are sent automatically by the mysql CLI on connect
        if trimmed.starts_with("select") {
            // select @@version_comment limit 1
            if trimmed.contains("@@version_comment") {
                let result = QueryResult::with_rows(
                    vec![ColumnDef { name: "@@version_comment".to_string(), col_type: ColumnType::String }],
                    vec![vec![Some("RorisDB".to_string())]],
                );
                return self.send_result_set(result).await;
            }
            // select database()
            if trimmed.contains("database()") {
                let db = self.database.clone().unwrap_or_default();
                let result = QueryResult::with_rows(
                    vec![ColumnDef { name: "database()".to_string(), col_type: ColumnType::String }],
                    vec![vec![if db.is_empty() { None } else { Some(db) }]],
                );
                return self.send_result_set(result).await;
            }
            // select @@variable (various system variables)
            if trimmed.contains("@@") {
                let result = QueryResult::with_rows(
                    vec![ColumnDef { name: "@@variable".to_string(), col_type: ColumnType::String }],
                    vec![vec![Some(String::new())]],
                );
                return self.send_result_set(result).await;
            }
            // Simple expressions without FROM (e.g., "SELECT 1", "SELECT 1+1")
            if !trimmed.contains("from") {
                let result = QueryResult::with_rows(
                    vec![ColumnDef { name: "1".to_string(), col_type: ColumnType::Int }],
                    vec![vec![Some("1".to_string())]],
                );
                return self.send_result_set(result).await;
            }
        }

        // Handle SHOW commands
        if trimmed.starts_with("show ") {
            if trimmed.contains("warnings") || trimmed.contains("errors") {
                let result = QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Level".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Code".to_string(), col_type: ColumnType::Int },
                        ColumnDef { name: "Message".to_string(), col_type: ColumnType::String },
                    ],
                    vec![],
                );
                return self.send_result_set(result).await;
            }
            if trimmed.contains("variables") {
                let result = QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Variable_name".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Value".to_string(), col_type: ColumnType::String },
                    ],
                    vec![],
                );
                return self.send_result_set(result).await;
            }
            if trimmed.contains("status") {
                let result = QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Variable_name".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Value".to_string(), col_type: ColumnType::String },
                    ],
                    vec![],
                );
                return self.send_result_set(result).await;
            }
            if trimmed.contains("processlist") {
                let result = QueryResult::with_rows(
                    vec![
                        ColumnDef { name: "Id".to_string(), col_type: ColumnType::Int },
                        ColumnDef { name: "User".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Host".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "db".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Command".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Time".to_string(), col_type: ColumnType::Int },
                        ColumnDef { name: "State".to_string(), col_type: ColumnType::String },
                        ColumnDef { name: "Info".to_string(), col_type: ColumnType::String },
                    ],
                    vec![],
                );
                return self.send_result_set(result).await;
            }
        }

        let result = self.handler.handle_query(sql);
        // Sync connection database state for USE commands
        let trimmed_lc = sql.trim().to_lowercase();
        if trimmed_lc.starts_with("use ") {
            // Extract database name from USE statement
            if let Some(pos) = sql.trim().find("USE ") {
                let after_use = &sql.trim()[pos + 4..].trim().trim_end_matches(';').trim();
                let db_name = after_use.split_whitespace().next().unwrap_or(after_use);
                if !db_name.is_empty() {
                    self.database = Some(db_name.to_string());
                    self.handler.set_database(db_name);
                }
            }
        }
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
    /// Updates self.seq_id so the next write uses the correct sequence ID.
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
                    let recv_seq = self.read_buf[3];
                    // Sync sequence ID: next packet we send should be recv_seq + 1
                    self.seq_id = recv_seq.wrapping_add(1);
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
