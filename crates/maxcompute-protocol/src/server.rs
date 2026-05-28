//! MaxCompute REST API server using axum.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, Method, StatusCode},
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
}

impl McServerState {
    pub fn new(handler: Arc<dyn QueryHandler>, config: McServerConfig) -> Self {
        Self {
            handler,
            config,
            instance_manager: Arc::new(InstanceManager::new()),
        }
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

/// Authentication middleware for MaxCompute REST API.
async fn auth_middleware(
    State(state): State<Arc<McServerState>>,
    method: Method,
    uri: axum::http::Uri,
    headers: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = uri.path();

    // Skip auth for health check
    if path == "/health" || path == "/" {
        return Ok(next.run(request).await);
    }

    let auth_config = crate::auth::McAuthConfig {
        access_key_id: state.config.access_key_id.clone(),
        access_key_secret: state.config.access_key_secret.clone(),
        region: state.config.region.clone(),
    };

    let query = uri.query().unwrap_or("");

    match crate::auth::verify_request(&auth_config, method.as_str(), path, query, &headers) {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => {
            info!("Auth signature mismatch for {} {}", method, path);
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            info!("Auth error for {} {}: {}", method, path, e);
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
