//! Web-based SQL Editor for RorisDB

pub mod routes;

use std::sync::Arc;
use crate::handler_struct::RorisQueryHandler;
use crate::connection_tracker::ConnectionTracker;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as TokioRwLock;

pub struct WebState {
    pub handler: Arc<RorisQueryHandler>,
    pub connection_tracker: Arc<ConnectionTracker>,
    pub query_history: TokioRwLock<Vec<QueryHistoryEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: u64,
    pub sql: String,
    pub database: Option<String>,
    pub timestamp: String,
    pub duration_ms: u64,
    pub row_count: usize,
    pub error: Option<String>,
}

impl WebState {
    pub fn new(
        handler: Arc<RorisQueryHandler>,
        connection_tracker: Arc<ConnectionTracker>,
    ) -> Self {
        Self {
            handler,
            connection_tracker,
            query_history: TokioRwLock::new(Vec::new()),
        }
    }

    pub async fn add_history(&self, entry: QueryHistoryEntry) {
        let mut history = self.query_history.write().await;
        history.push(entry);
        // Keep only last 100 entries
        if history.len() > 100 {
            history.remove(0);
        }
    }
}

pub async fn start_web_server(state: Arc<WebState>, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::{Router, routing::{get, post}};
    use tower_http::cors::CorsLayer;
    use std::net::SocketAddr;

    let app = Router::new()
        .route("/", get(routes::serve_editor))
        .route("/api/query", post(routes::api_query))
        .route("/api/databases", get(routes::api_databases))
        .route("/api/tables/{db}", get(routes::api_tables))
        .route("/api/schema/{db}/{table}", get(routes::api_schema))
        .route("/api/history", get(routes::api_history))
        .route("/api/status", get(routes::api_status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Web SQL Editor starting on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
