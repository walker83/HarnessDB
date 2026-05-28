//! MaxCompute REST API server using axum.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use mysql_protocol::server::QueryHandler;
use tokio::net::TcpListener;
use tracing::info;

use crate::handlers::InstanceManager;

/// Configuration for the MaxCompute protocol server.
#[derive(Debug, Clone)]
pub struct McServerConfig {
    pub bind_addr: String,
    pub port: u16,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub default_project: String,
    pub region: Option<String>,
}

impl Default for McServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 9031,
            access_key_id: "roris".to_string(),
            access_key_secret: "roris-secret".to_string(),
            default_project: "default".to_string(),
            region: None,
        }
    }
}

/// Shared state for the MaxCompute server.
pub struct McServerState {
    pub handler: Arc<dyn QueryHandler>,
    pub config: McServerConfig,
    pub instance_manager: Arc<InstanceManager>,
    conn_id_counter: AtomicU32,
}

impl McServerState {
    pub fn new(handler: Arc<dyn QueryHandler>, config: McServerConfig) -> Self {
        Self {
            handler,
            config,
            instance_manager: Arc::new(InstanceManager::new()),
            conn_id_counter: AtomicU32::new(1_000_000),
        }
    }

    /// Get the next connection ID for query handling.
    pub fn next_conn_id(&self) -> u32 {
        self.conn_id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

/// Mock query handler for tests.
#[cfg(test)]
pub struct MockQueryHandler;

#[cfg(test)]
impl MockQueryHandler {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
impl QueryHandler for MockQueryHandler {
    fn handle_query(&self, _conn_id: u32, sql: &str) -> mysql_protocol::server::QueryResult {
        let upper = sql.trim().to_uppercase();
        if upper.starts_with("SHOW TABLES") {
            mysql_protocol::server::QueryResult::with_rows(
                vec![mysql_protocol::server::ColumnDef {
                    name: "table_name".to_string(),
                    col_type: mysql_protocol::server::ColumnType::String,
                }],
                vec![vec![Some("test_table".to_string())]],
            )
        } else if upper.starts_with("DESCRIBE") || upper.starts_with("DESC ") {
            mysql_protocol::server::QueryResult::with_rows(
                vec![
                    mysql_protocol::server::ColumnDef {
                        name: "col_name".to_string(),
                        col_type: mysql_protocol::server::ColumnType::String,
                    },
                    mysql_protocol::server::ColumnDef {
                        name: "data_type".to_string(),
                        col_type: mysql_protocol::server::ColumnType::String,
                    },
                ],
                vec![
                    vec![Some("id".to_string()), Some("BIGINT".to_string())],
                    vec![Some("name".to_string()), Some("STRING".to_string())],
                ],
            )
        } else {
            mysql_protocol::server::QueryResult::ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ======================================================================
    // health_check unit test
    // ======================================================================

    #[tokio::test]
    async fn test_health_check_response() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type, "application/json");

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("ok"));
        assert!(body_str.contains("maxcompute"));
    }

    // ======================================================================
    // McServerState
    // ======================================================================

    #[test]
    fn test_mc_server_state_conn_id() {
        let handler = Arc::new(MockQueryHandler::new());
        let config = McServerConfig::default();
        let state = McServerState::new(handler, config);

        let id1 = state.next_conn_id();
        let id2 = state.next_conn_id();
        assert_eq!(id2, id1 + 1, "Connection IDs should be sequential");
    }

    #[test]
    fn test_mc_server_state_instance_manager_initialized() {
        let handler = Arc::new(MockQueryHandler::new());
        let config = McServerConfig::default();
        let state = McServerState::new(handler, config);

        assert!(state.instance_manager.is_empty(), "Instance manager should be empty initially");
    }

    // ======================================================================
    // McServerConfig
    // ======================================================================

    #[test]
    fn test_mc_server_config_default() {
        let config = McServerConfig::default();
        assert_eq!(config.bind_addr, "127.0.0.1");
        assert_eq!(config.port, 9031);
        assert_eq!(config.access_key_id, "roris");
        assert_eq!(config.access_key_secret, "roris-secret");
        assert_eq!(config.default_project, "default");
        assert!(config.region.is_none());
    }

    // ======================================================================
    // MockQueryHandler
    // ======================================================================

    #[test]
    fn test_mock_query_handler_show_tables() {
        let handler = MockQueryHandler::new();
        let result = handler.handle_query(1, "SHOW TABLES");
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.columns[0].name, "table_name");
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0].as_deref(), Some("test_table"));
    }

    #[test]
    fn test_mock_query_handler_describe() {
        let handler = MockQueryHandler::new();
        let result = handler.handle_query(1, "DESCRIBE mytable");
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_mock_query_handler_ok() {
        let handler = MockQueryHandler::new();
        let result = handler.handle_query(1, "SELECT 1");
        assert!(result.columns.is_empty() && result.rows.is_empty(),
                "Unrecognized queries should return QueryResult::ok()");
    }
}

// ---------------------------------------------------------------------------
async fn auth_middleware(
    State(state): State<Arc<McServerState>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    let uri = &parts.uri;
    let path = uri.path();

    // Skip auth for health check
    if path == "/health" || path == "/" {
        let request = Request::from_parts(parts, Body::from(body));
        return Ok(next.run(request).await);
    }

    let auth_config = crate::auth::McAuthConfig {
        access_key_id: state.config.access_key_id.clone(),
        access_key_secret: state.config.access_key_secret.clone(),
        region: state.config.region.clone(),
    };

    let query = uri.query().unwrap_or("");

    // Read body for V4 signature verification
    let body_bytes = axum::body::to_bytes(body, 5 * 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_for_auth = body_bytes.clone();

    match crate::auth::verify_request(
        &auth_config,
        parts.method.as_str(),
        path,
        query,
        &parts.headers,
        &body_for_auth,
    ) {
        Ok(true) => {
            let request = Request::from_parts(parts, Body::from(body_bytes));
            Ok(next.run(request).await)
        }
        Ok(false) => {
            info!("Auth signature mismatch for {} {}", parts.method, path);
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            info!("Auth error for {} {}: {}", parts.method, path, e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Start the MaxCompute REST API server.
pub async fn start_mc_server(
    handler: Arc<dyn QueryHandler>,
    config: McServerConfig,
) -> anyhow::Result<()> {
    let bind_addr = format!("{}:{}", config.bind_addr, config.port);
    info!("Starting MaxCompute REST API server on {}", bind_addr);

    let state = Arc::new(McServerState::new(handler, config));
    let router = build_router_with_state(state);

    let listener = TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

/// Build the router with the given state (public API for integration).
pub fn build_router_with_state(state: Arc<McServerState>) -> Router {
    let api_router = crate::handlers::build_router();

    Router::new()
        .route("/health", axum::routing::get(health_check))
        .nest("/api", api_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"status":"ok","protocol":"maxcompute"}"#,
    )
}
