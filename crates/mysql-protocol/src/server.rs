use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, info_span, Instrument};

use crate::auth::AuthPluginType;
use crate::connection::Connection;
use crate::packet::Column;

/// Configuration for the MySQL server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub port: u16,
    pub default_auth_plugin: AuthPluginType,
    /// Authentication timeout in seconds. Default: 30 seconds.
    /// Connection pools often need more time for handshake.
    pub auth_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 9030,
            default_auth_plugin: AuthPluginType::NativePassword,
            auth_timeout_secs: 30,
        }
    }
}

/// Result of a query execution, returned by the QueryHandler.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Vec<Option<String>>>,
}

/// Column definition for query results.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
}

/// Simplified column type for query result descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    String,
    Int,
    Float,
    Double,
    Date,
    DateTime,
    Blob,
}

impl QueryResult {
    /// Create an empty result set with the given columns.
    pub fn new(columns: Vec<ColumnDef>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    /// Create a result from columns and rows.
    pub fn with_rows(columns: Vec<ColumnDef>, rows: Vec<Vec<Option<String>>>) -> Self {
        Self { columns, rows }
    }

    /// Create an empty OK-style result (e.g. for DDL statements).
    pub fn ok() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }
}

/// Trait that callers implement to handle SQL queries from MySQL clients.
pub trait QueryHandler: Send + Sync + 'static {
    fn handle_query(&self, sql: &str) -> QueryResult;
    /// Called when client changes database (USE command). Default: do nothing.
    fn set_database(&self, _db: &str) {}
}

/// The MySQL protocol server.
pub struct MysqlServer {
    config: ServerConfig,
    handler: Arc<dyn QueryHandler>,
    connection_counter: AtomicU32,
}

impl MysqlServer {
    pub fn new(config: ServerConfig, handler: Arc<dyn QueryHandler>) -> Self {
        Self {
            config,
            handler,
            connection_counter: AtomicU32::new(1),
        }
    }

    /// Start accepting connections. Runs until the server is shut down.
    pub async fn run(&self) -> std::io::Result<()> {
        let addr = format!("{}:{}", self.config.bind_addr, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("MySQL server listening on {}", addr);

        let auth_timeout_secs = self.config.auth_timeout_secs;

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let conn_id = self.connection_counter.fetch_add(1, Ordering::Relaxed);
            let handler = self.handler.clone();

            info!("Accepted connection {} from {}", conn_id, peer_addr);

            tokio::spawn(
                async move {
                    if let Err(e) = handle_connection(stream, conn_id, handler, auth_timeout_secs).await {
                        error!("Connection {} error: {}", conn_id, e);
                    }
                    info!("Connection {} closed", conn_id);
                }
                .instrument(info_span!("mysql_conn", cid = conn_id)),
            );
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    conn_id: u32,
    handler: Arc<dyn QueryHandler>,
    auth_timeout_secs: u64,
) -> std::io::Result<()> {
    let mut conn = Connection::new(stream, conn_id, handler, auth_timeout_secs);
    conn.run().await
}

/// Convert a ColumnDef (from QueryResult) to a Column (for packet encoding).
impl From<&ColumnDef> for Column {
    fn from(def: &ColumnDef) -> Self {
        let col_type = match def.col_type {
            ColumnType::String => crate::packet::column_type::VAR_STRING,
            ColumnType::Int => crate::packet::column_type::LONGLONG,
            ColumnType::Float => crate::packet::column_type::FLOAT,
            ColumnType::Double => crate::packet::column_type::DOUBLE,
            ColumnType::Date => crate::packet::column_type::DATE,
            ColumnType::DateTime => crate::packet::column_type::DATETIME,
            ColumnType::Blob => crate::packet::column_type::BLOB,
        };
        Column::new(&def.name, col_type)
    }
}
