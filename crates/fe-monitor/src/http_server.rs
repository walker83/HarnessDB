use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

use super::MonitoringManager;

/// HTTP server for monitoring endpoints
pub struct MonitoringHttpServer {
    port: u16,
    monitoring: Arc<MonitoringManager>,
}

impl MonitoringHttpServer {
    pub fn new(port: u16, monitoring: Arc<MonitoringManager>) -> Self {
        Self { port, monitoring }
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/api/v1/health", get(health_handler))
            .route("/api/v1/metrics", get(api_metrics_handler))
            .route("/api/v1/queries", get(queries_handler))
            .route("/api/v1/queries/running", get(running_queries_handler))
            .route("/api/v1/queries/slow", get(slow_queries_handler))
            .with_state(self.monitoring);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Monitoring HTTP server listening on {}", self.port);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(monitoring): State<Arc<MonitoringManager>>) -> Response {
    let metrics = monitoring.metrics.export_prometheus();

    match metrics {
        Ok(text) => (
            [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            text,
        )
            .into_response(),
        Err(e) => {
            error!("Failed to export metrics: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
}

/// Health check endpoint
async fn health_handler(State(monitoring): State<Arc<MonitoringManager>>) -> Response {
    let fe_metrics = monitoring.metrics.get_fe_metrics();
    let be_metrics = monitoring.metrics.get_be_metrics();

    let health = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "fe": {
            "queries_total": fe_metrics.queries_total,
            "queries_success": fe_metrics.queries_success,
            "queries_failed": fe_metrics.queries_failed,
            "active_connections": fe_metrics.active_connections,
        },
        "be": {
            "queries_total": be_metrics.queries_total,
            "bytes_read": be_metrics.bytes_read,
            "bytes_written": be_metrics.bytes_written,
            "memory_used_bytes": be_metrics.memory_used_bytes,
            "disk_used_bytes": be_metrics.disk_used_bytes,
        }
    });

    axum::Json(health).into_response()
}

/// API metrics endpoint (JSON format)
async fn api_metrics_handler(State(monitoring): State<Arc<MonitoringManager>>) -> Response {
    let fe_metrics = monitoring.metrics.get_fe_metrics();
    let be_metrics = monitoring.metrics.get_be_metrics();

    let metrics = serde_json::json!({
        "fe": fe_metrics,
        "be": be_metrics,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    axum::Json(metrics).into_response()
}

/// List recent queries
async fn queries_handler(
    State(monitoring): State<Arc<MonitoringManager>>,
    axum::extract::Query(params): axum::extract::Query<QueryParams>,
) -> Response {
    let limit = params.limit.unwrap_or(100);
    let profiles = monitoring.query_profiler.list_profiles(Some(limit)).await;

    let queries: Vec<_> = profiles
        .into_iter()
        .map(|p| serde_json::json!({
            "query_id": p.query_id,
            "query": p.query,
            "user": p.user,
            "database": p.database,
            "status": format!("{:?}", p.status),
            "start_time": p.start_time.to_rfc3339(),
            "end_time": p.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": p.duration_ms,
            "rows_produced": p.rows_produced,
            "bytes_scanned": p.bytes_scanned,
        }))
        .collect();

    axum::Json(queries).into_response()
}

/// List running queries
async fn running_queries_handler(State(monitoring): State<Arc<MonitoringManager>>) -> Response {
    let queries = monitoring.query_profiler.get_running_queries().await;

    let running: Vec<_> = queries
        .into_iter()
        .map(|p| serde_json::json!({
            "query_id": p.query_id,
            "query": p.query,
            "user": p.user,
            "database": p.database,
            "start_time": p.start_time.to_rfc3339(),
            "duration_ms": p.duration_ms,
        }))
        .collect();

    axum::Json(running).into_response()
}

/// List slow queries
async fn slow_queries_handler(
    State(monitoring): State<Arc<MonitoringManager>>,
    axum::extract::Query(params): axum::extract::Query<SlowQueryParams>,
) -> Response {
    let threshold_ms = params.threshold.unwrap_or(1000);
    let queries = monitoring.query_profiler.get_slow_queries(threshold_ms).await;

    let slow: Vec<_> = queries
        .into_iter()
        .map(|p| serde_json::json!({
            "query_id": p.query_id,
            "query": p.query,
            "user": p.user,
            "database": p.database,
            "start_time": p.start_time.to_rfc3339(),
            "end_time": p.end_time.map(|t| t.to_rfc3339()),
            "duration_ms": p.duration_ms,
            "rows_produced": p.rows_produced,
            "bytes_scanned": p.bytes_scanned,
        }))
        .collect();

    axum::Json(slow).into_response()
}

#[derive(serde::Deserialize)]
struct QueryParams {
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct SlowQueryParams {
    threshold: Option<u64>,
}
