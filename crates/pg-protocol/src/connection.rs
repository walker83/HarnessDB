//! PostgreSQL wire protocol connection state machine.
//!
//! Implements the full connection lifecycle: startup, authentication,
//! simple query, and extended query protocols.

use bytes::{Buf, BufMut, BytesMut};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use crate::auth::{generate_salt, validate_password, AuthConfig};
use crate::message::{
    create_error_response, BackendMessage, DescribeTarget, FieldDescription, FrontendMessage,
    PgProtocolError, TransactionStatus, CANCEL_REQUEST_CODE, OID_BOOL, OID_DATE, OID_FLOAT4,
    OID_FLOAT8, OID_INT4, OID_TEXT, OID_TIMESTAMP,
    PG_PROTOCOL_VERSION_3, SSL_REQUEST_CODE, sqlstate,
};
use mysql_protocol::server::{ColumnType, QueryHandler, QueryResult};

/// Maximum size of a single PG protocol message (16 MB).
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Session state for a single PG connection.
struct SessionState {
    database: String,
    username: String,
    prepared_statements: HashMap<String, (String, Vec<u32>, usize)>,
    _start_time: Instant,
}

/// A single PostgreSQL wire protocol connection.
pub struct PgConnection {
    stream: TcpStream,
    conn_id: u32,
    handler: Arc<dyn QueryHandler>,
    read_buf: BytesMut,
    write_buf: BytesMut,
    auth_config: AuthConfig,
    session: SessionState,
    process_id: i32,
    secret_key: i32,
}

impl PgConnection {
    pub fn new(
        stream: TcpStream,
        conn_id: u32,
        handler: Arc<dyn QueryHandler>,
        auth_config: AuthConfig,
    ) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            stream,
            conn_id,
            handler,
            read_buf: BytesMut::with_capacity(8192),
            write_buf: BytesMut::with_capacity(8192),
            auth_config,
            session: SessionState {
                database: String::new(),
                username: String::new(),
                prepared_statements: HashMap::new(),
                _start_time: Instant::now(),
            },
            process_id: rng.gen_range(10000..99999),
            secret_key: rng.gen_range(100000..999999),
        }
    }

    pub async fn run(&mut self) -> Result<(), PgProtocolError> {
        let peer_addr = self
            .stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        self.handler.on_connect(self.conn_id, "root", &peer_addr);
        let result = self.run_inner().await;
        self.handler.on_disconnect(self.conn_id);
        result
    }

    async fn run_inner(&mut self) -> Result<(), PgProtocolError> {
        self.handle_startup().await?;
        self.handle_auth().await?;
        self.send_parameter_status().await?;
        self.send_backend_key_data().await?;
        self.send_ready_for_query().await?;
        self.handle_messages().await
    }

    // ======================================================================
    // Startup
    // ======================================================================

    async fn handle_startup(&mut self) -> Result<(), PgProtocolError> {
        loop {
            self.read_buf_ensure(4).await?;
            let len = (&self.read_buf[..4]).get_i32() as usize;
            if len < 4 {
                return Err(PgProtocolError::ProtocolViolation(
                    "startup message length too small".to_string(),
                ));
            }
            self.read_buf_ensure(len).await?;
            if len < 8 {
                return Err(PgProtocolError::ProtocolViolation(
                    "startup message too short".to_string(),
                ));
            }
            let version = (&self.read_buf[4..8]).get_i32();

            match version {
                PG_PROTOCOL_VERSION_3 => {
                    let msg = FrontendMessage::decode_startup(&mut self.read_buf)?.ok_or_else(
                        || {
                            PgProtocolError::ProtocolViolation(
                                "incomplete startup message".to_string(),
                            )
                        },
                    )?;
                    match msg {
                        FrontendMessage::StartupMessage { params, .. } => {
                            self.session.username =
                                params.get("user").cloned().unwrap_or_else(|| "root".to_string());
                            self.session.database = params
                                .get("database")
                                .cloned()
                                .unwrap_or_default();
                            info!(
                                "PG conn {}: startup user='{}' database='{}'",
                                self.conn_id, self.session.username, self.session.database
                            );
                            return Ok(());
                        }
                        _ => unreachable!(),
                    }
                }
                SSL_REQUEST_CODE => {
                    debug!("PG conn {}: SSL declined", self.conn_id);
                    self.read_buf.advance(len);
                    self.write_buf.put_u8(b'N');
                    self.flush_write().await?;
                    continue;
                }
                CANCEL_REQUEST_CODE => {
                    if len >= 16 {
                        let pid = (&self.read_buf[8..12]).get_i32();
                        let key = (&self.read_buf[12..16]).get_i32();
                        warn!("PG conn {}: cancel request pid={} key={}", self.conn_id, pid, key);
                    }
                    return Err(PgProtocolError::CancelRequest);
                }
                _ => {
                    return Err(PgProtocolError::ProtocolViolation(format!(
                        "unknown protocol version: {}",
                        version
                    )));
                }
            }
        }
    }

    // ======================================================================
    // Authentication
    // ======================================================================

    async fn handle_auth(&mut self) -> Result<(), PgProtocolError> {
        let salt = generate_salt();

        BackendMessage::AuthenticationMD5Password { salt }
            .encode(&mut self.write_buf);
        self.flush_write().await?;

        self.read_buf_ensure(5).await?;
        let msg = FrontendMessage::decode(&mut self.read_buf)?.ok_or_else(|| {
            PgProtocolError::ProtocolViolation("incomplete password message".to_string())
        })?;

        let password = match msg {
            FrontendMessage::PasswordMessage { password } => password,
            other => {
                return Err(PgProtocolError::ProtocolViolation(format!(
                    "expected PasswordMessage, got {:?}",
                    other
                )));
            }
        };

        if !validate_password(&self.auth_config, &self.session.username, &password, &salt) {
            warn!("PG conn {}: auth failed for user '{}'", self.conn_id, self.session.username);
            return Err(PgProtocolError::AuthenticationFailed(
                "password authentication failed".to_string(),
            ));
        }

        info!("PG conn {}: auth OK for user '{}'", self.conn_id, self.session.username);
        BackendMessage::AuthenticationOk.encode(&mut self.write_buf);
        self.flush_write().await
    }

    // ======================================================================
    // Post-auth initialization
    // ======================================================================

    async fn send_parameter_status(&mut self) -> Result<(), PgProtocolError> {
        let params = [
            ("server_version", "15.0"),
            ("server_encoding", "UTF8"),
            ("client_encoding", "UTF8"),
            ("DateStyle", "ISO, MDY"),
            ("TimeZone", "UTC"),
            ("integer_datetimes", "on"),
            ("standard_conforming_strings", "on"),
            ("application_name", ""),
            ("default_transaction_read_only", "off"),
            ("in_hot_standby", "off"),
            ("is_superuser", "on"),
            ("session_authorization", &self.session.username),
        ];
        for (key, value) in &params {
            BackendMessage::ParameterStatus {
                key: key.to_string(),
                value: value.to_string(),
            }
            .encode(&mut self.write_buf);
        }
        self.flush_write().await
    }

    async fn send_backend_key_data(&mut self) -> Result<(), PgProtocolError> {
        BackendMessage::BackendKeyData {
            pid: self.process_id,
            secret_key: self.secret_key,
        }
        .encode(&mut self.write_buf);
        self.flush_write().await
    }

    async fn send_ready_for_query(&mut self) -> Result<(), PgProtocolError> {
        BackendMessage::ReadyForQuery {
            status: TransactionStatus::Idle,
        }
        .encode(&mut self.write_buf);
        self.flush_write().await
    }

    // ======================================================================
    // Message loop
    // ======================================================================

    async fn handle_messages(&mut self) -> Result<(), PgProtocolError> {
        loop {
            self.read_buf_ensure(5).await?;
            let msg_len = (&self.read_buf[1..5]).get_i32() as usize;
            if msg_len > MAX_MESSAGE_SIZE || msg_len < 4 {
                return Err(PgProtocolError::ProtocolViolation(format!(
                    "invalid message length: {}",
                    msg_len
                )));
            }
            self.read_buf_ensure(1 + msg_len).await?;

            let msg = match FrontendMessage::decode(&mut self.read_buf) {
                Ok(Some(m)) => m,
                Ok(None) => continue,
                Err(e) => {
                    error!("PG conn {}: decode error: {:?}", self.conn_id, e);
                    create_error_response(
                        "ERROR",
                        sqlstate::PROTOCOL_VIOLATION,
                        &format!("message decode error: {}", e),
                    )
                    .encode(&mut self.write_buf);
                    self.flush_write().await?;
                    self.send_ready_for_query().await?;
                    continue;
                }
            };

            match msg {
                FrontendMessage::Query { sql } => self.handle_query(&sql).await?,
                FrontendMessage::Terminate => {
                    info!("PG conn {}: Terminate", self.conn_id);
                    return Ok(());
                }
                FrontendMessage::Parse { name, query, param_types } => {
                    self.handle_parse(&name, &query, &param_types).await;
                }
                FrontendMessage::Bind { portal, statement, .. } => {
                    self.handle_bind(&portal, &statement).await;
                }
                FrontendMessage::Describe { target, name } => {
                    self.handle_describe(target, &name).await;
                }
                FrontendMessage::Execute { portal, .. } => self.handle_execute(&portal).await,
                FrontendMessage::Close { target, name } => self.handle_close(target, &name).await,
                FrontendMessage::Sync => self.send_ready_for_query().await?,
                other => {
                    debug!("PG conn {}: unhandled: {:?}", self.conn_id, other);
                    self.send_ready_for_query().await?;
                }
            }
        }
    }

    // ======================================================================
    // Simple Query
    // ======================================================================

    async fn handle_query(&mut self, sql: &str) -> Result<(), PgProtocolError> {
        let trimmed = sql.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            BackendMessage::EmptyQueryResponse.encode(&mut self.write_buf);
            self.send_ready_for_query().await?;
            return Ok(());
        }

        let result = self.handler.handle_query(self.conn_id, trimmed);

        if result.columns.is_empty() {
            let tag = infer_command_tag(trimmed, 0);
            BackendMessage::CommandComplete { tag }.encode(&mut self.write_buf);
        } else {
            self.send_query_result(&result, trimmed).await?;
        }

        self.send_ready_for_query().await
    }

    async fn send_query_result(&mut self, result: &QueryResult, sql: &str) -> Result<(), PgProtocolError> {
        let fields: Vec<FieldDescription> = result
            .columns
            .iter()
            .map(|col| {
                let type_oid = map_column_type_to_oid(col.col_type);
                let type_size = map_column_type_to_size(col.col_type);
                FieldDescription::new(&col.name, type_oid, type_size)
            })
            .collect();

        BackendMessage::RowDescription {
            fields: fields.clone(),
        }
        .encode(&mut self.write_buf);

        for row in &result.rows {
            let values: Vec<Option<Vec<u8>>> = row
                .iter()
                .zip(fields.iter())
                .map(|(val, field)| {
                    val.as_ref().map(|s| {
                        if field.type_oid == OID_BOOL {
                            match s.to_lowercase().as_str() {
                                "true" | "1" | "yes" => b"t".to_vec(),
                                _ => b"f".to_vec(),
                            }
                        } else {
                            s.as_bytes().to_vec()
                        }
                    })
                })
                .collect();
            BackendMessage::DataRow { values }.encode(&mut self.write_buf);
        }

        let tag = infer_command_tag(sql, result.rows.len() as i64);
        BackendMessage::CommandComplete { tag }.encode(&mut self.write_buf);
        self.flush_write().await
    }

    // ======================================================================
    // Extended Query
    // ======================================================================

    async fn handle_parse(&mut self, name: &str, query: &str, param_types: &[u32]) {
        let stmt_name = if name.is_empty() {
            format!("_pg3_{}", self.conn_id)
        } else {
            name.to_string()
        };
        let trimmed = query.trim().trim_end_matches(';');
        let result = self.handler.handle_query(self.conn_id, trimmed);
        self.session.prepared_statements.insert(
            stmt_name,
            (query.to_string(), param_types.to_vec(), result.columns.len()),
        );
        if !param_types.is_empty() {
            BackendMessage::ParameterDescription {
                type_oids: param_types.to_vec(),
            }
            .encode(&mut self.write_buf);
        }
        BackendMessage::ParseComplete.encode(&mut self.write_buf);
        if let Err(e) = self.flush_write().await {
            error!("PG conn {}: flush error in handle_parse: {}", self.conn_id, e);
        }
    }

    async fn handle_bind(&mut self, _portal: &str, _statement: &str) {
        BackendMessage::BindComplete.encode(&mut self.write_buf);
        BackendMessage::NoData.encode(&mut self.write_buf);
        if let Err(e) = self.flush_write().await {
            error!("PG conn {}: flush error in handle_bind: {}", self.conn_id, e);
        }
    }

    async fn handle_describe(&mut self, target: DescribeTarget, name: &str) {
        match target {
            DescribeTarget::Statement => {
                let stmt_name = if name.is_empty() {
                    format!("_pg3_{}", self.conn_id)
                } else {
                    name.to_string()
                };
                if let Some((_sql, param_types, _num_cols)) =
                    self.session.prepared_statements.get(&stmt_name)
                {
                    if !param_types.is_empty() {
                        BackendMessage::ParameterDescription {
                            type_oids: param_types.clone(),
                        }
                        .encode(&mut self.write_buf);
                    }
                }
                BackendMessage::NoData.encode(&mut self.write_buf);
            }
            DescribeTarget::Portal => {
                BackendMessage::NoData.encode(&mut self.write_buf);
            }
        }
        if let Err(e) = self.flush_write().await {
            error!("PG conn {}: flush error in handle_describe: {}", self.conn_id, e);
        }
    }

    async fn handle_execute(&mut self, _portal: &str) {
        BackendMessage::EmptyQueryResponse.encode(&mut self.write_buf);
        self.send_ready_for_query().await.ok();
    }

    async fn handle_close(&mut self, target: DescribeTarget, name: &str) {
        let stmt_name = if name.is_empty() {
            format!("_pg3_{}", self.conn_id)
        } else {
            name.to_string()
        };
        match target {
            DescribeTarget::Statement => {
                self.session.prepared_statements.remove(&stmt_name);
            }
            DescribeTarget::Portal => {}
        }
        BackendMessage::CloseComplete.encode(&mut self.write_buf);
        if let Err(e) = self.flush_write().await {
            error!("PG conn {}: flush error in handle_close: {}", self.conn_id, e);
        }
    }

    // ======================================================================
    // I/O
    // ======================================================================

    async fn read_buf_ensure(&mut self, n: usize) -> Result<(), PgProtocolError> {
        while self.read_buf.len() < n {
            let mut tmp = vec![0u8; 8192];
            let nread = self.stream.read(&mut tmp).await?;
            if nread == 0 {
                return Err(PgProtocolError::ConnectionClosed);
            }
            self.read_buf.extend_from_slice(&tmp[..nread]);
        }
        Ok(())
    }

    async fn flush_write(&mut self) -> Result<(), PgProtocolError> {
        if !self.write_buf.is_empty() {
            self.stream.write_all(&self.write_buf).await?;
            self.write_buf.clear();
        }
        Ok(())
    }
}

// ============================================================================
// Standalone functions
// ============================================================================

/// Infer a PostgreSQL command tag from SQL text (e.g., "SELECT 5", "INSERT 0 1").
fn infer_command_tag(sql: &str, row_count: i64) -> String {
    let upper = sql.trim().to_uppercase();
    if upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("VALUES")
    {
        format!("SELECT {}", row_count)
    } else if upper.starts_with("INSERT") {
        format!("INSERT 0 {}", row_count)
    } else if upper.starts_with("UPDATE") {
        format!("UPDATE {}", row_count)
    } else if upper.starts_with("DELETE") {
        format!("DELETE {}", row_count)
    } else if upper.starts_with("CREATE TABLE") {
        "CREATE TABLE".to_string()
    } else if upper.starts_with("CREATE DATABASE") {
        "CREATE DATABASE".to_string()
    } else if upper.starts_with("CREATE") {
        "CREATE".to_string()
    } else if upper.starts_with("DROP TABLE") {
        "DROP TABLE".to_string()
    } else if upper.starts_with("DROP DATABASE") {
        "DROP DATABASE".to_string()
    } else if upper.starts_with("DROP") {
        "DROP".to_string()
    } else if upper.starts_with("ALTER") {
        "ALTER TABLE".to_string()
    } else if upper.starts_with("TRUNCATE") {
        "TRUNCATE TABLE".to_string()
    } else if upper.starts_with("BEGIN") {
        "BEGIN".to_string()
    } else if upper.starts_with("COMMIT") {
        "COMMIT".to_string()
    } else if upper.starts_with("ROLLBACK") {
        "ROLLBACK".to_string()
    } else if upper.starts_with("SET") {
        "SET".to_string()
    } else if upper.starts_with("SHOW") {
        "SHOW".to_string()
    } else if upper.starts_with("USE") {
        "SET".to_string()
    } else if upper.starts_with("EXPLAIN") || upper.starts_with("DESCRIBE")
        || upper.starts_with("DESC")
    {
        format!("EXPLAIN {}", row_count)
    } else {
        format!("OK {}", row_count)
    }
}

/// Map MySQL ColumnType to PostgreSQL type OID.
fn map_column_type_to_oid(col_type: ColumnType) -> i32 {
    match col_type {
        ColumnType::String => OID_TEXT,
        ColumnType::Int => OID_INT4,
        ColumnType::Float => OID_FLOAT4,
        ColumnType::Double => OID_FLOAT8,
        ColumnType::Date => OID_DATE,
        ColumnType::DateTime => OID_TIMESTAMP,
        ColumnType::Blob => OID_TEXT,
    }
}

/// Map MySQL ColumnType to PG type size (-1 = variable length).
fn map_column_type_to_size(col_type: ColumnType) -> i16 {
    match col_type {
        ColumnType::String => -1,
        ColumnType::Int => 4,
        ColumnType::Float => 4,
        ColumnType::Double => 8,
        ColumnType::Date => 4,
        ColumnType::DateTime => 8,
        ColumnType::Blob => -1,
    }
}

/// Standalone entry point: create and run a PG connection.
pub async fn run_connection(
    stream: TcpStream,
    conn_id: u32,
    handler: Arc<dyn QueryHandler>,
    auth_config: AuthConfig,
) -> Result<(), PgProtocolError> {
    let mut conn = PgConnection::new(stream, conn_id, handler, auth_config);
    conn.run().await
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_command_tag_select() {
        assert_eq!(infer_command_tag("SELECT * FROM t", 5), "SELECT 5");
    }

    #[test]
    fn test_infer_command_tag_insert() {
        assert_eq!(infer_command_tag("INSERT INTO t VALUES (1)", 1), "INSERT 0 1");
    }

    #[test]
    fn test_infer_command_tag_update() {
        assert_eq!(infer_command_tag("UPDATE t SET x=1", 3), "UPDATE 3");
    }

    #[test]
    fn test_infer_command_tag_delete() {
        assert_eq!(infer_command_tag("DELETE FROM t", 2), "DELETE 2");
    }

    #[test]
    fn test_infer_command_tag_create_table() {
        assert_eq!(infer_command_tag("CREATE TABLE t (id INT)", 0), "CREATE TABLE");
    }

    #[test]
    fn test_infer_command_tag_drop() {
        assert_eq!(infer_command_tag("DROP TABLE t", 0), "DROP TABLE");
    }

    #[test]
    fn test_infer_command_tag_begin() {
        assert_eq!(infer_command_tag("BEGIN", 0), "BEGIN");
    }

    #[test]
    fn test_infer_command_tag_commit() {
        assert_eq!(infer_command_tag("COMMIT", 0), "COMMIT");
    }

    #[test]
    fn test_infer_command_tag_set() {
        assert_eq!(infer_command_tag("SET x = 1", 0), "SET");
    }

    #[test]
    fn test_infer_command_tag_show() {
        assert_eq!(infer_command_tag("SHOW x", 0), "SHOW");
    }

    #[test]
    fn test_infer_command_tag_use() {
        assert_eq!(infer_command_tag("USE mydb", 0), "SET");
    }

    #[test]
    fn test_infer_command_tag_with_cte() {
        assert_eq!(infer_command_tag("WITH t AS (SELECT 1) SELECT * FROM t", 3), "SELECT 3");
    }

    #[test]
    fn test_infer_command_tag_values() {
        assert_eq!(infer_command_tag("VALUES (1), (2)", 2), "SELECT 2");
    }

    #[test]
    fn test_infer_command_tag_unknown() {
        assert_eq!(infer_command_tag("SOME_COMMAND args", 0), "OK 0");
    }

    #[test]
    fn test_map_column_type_oid() {
        assert_eq!(map_column_type_to_oid(ColumnType::String), OID_TEXT);
        assert_eq!(map_column_type_to_oid(ColumnType::Int), OID_INT4);
        assert_eq!(map_column_type_to_oid(ColumnType::Float), OID_FLOAT4);
        assert_eq!(map_column_type_to_oid(ColumnType::Double), OID_FLOAT8);
        assert_eq!(map_column_type_to_oid(ColumnType::Date), OID_DATE);
        assert_eq!(map_column_type_to_oid(ColumnType::DateTime), OID_TIMESTAMP);
        assert_eq!(map_column_type_to_oid(ColumnType::Blob), OID_TEXT);
    }

    #[test]
    fn test_map_column_type_size() {
        assert_eq!(map_column_type_to_size(ColumnType::String), -1);
        assert_eq!(map_column_type_to_size(ColumnType::Int), 4);
        assert_eq!(map_column_type_to_size(ColumnType::Float), 4);
        assert_eq!(map_column_type_to_size(ColumnType::Double), 8);
        assert_eq!(map_column_type_to_size(ColumnType::Date), 4);
        assert_eq!(map_column_type_to_size(ColumnType::DateTime), 8);
        assert_eq!(map_column_type_to_size(ColumnType::Blob), -1);
    }
}