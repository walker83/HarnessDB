//! REST API routes for the SQL Editor

use std::sync::Arc;
use std::time::Instant;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use mysql_protocol::QueryHandler;
use serde::{Deserialize, Serialize};

use crate::web::{WebState, QueryHistoryEntry};

// ---- Response types ----

#[derive(Serialize)]
pub struct QueryResponse {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<Option<String>>>,
    pub duration_ms: u64,
    pub row_count: usize,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub col_type: String,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub sql: String,
    pub database: Option<String>,
}

#[derive(Serialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub comment: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
    pub total_queries: u64,
    pub active_connections: u32,
    pub total_connections: u64,
    pub databases: usize,
}

// ---- Embedded HTML ----

const EDITOR_HTML: &str = include_str!("editor.html");

pub async fn serve_editor() -> (StatusCode, [(String, String); 2], String) {
    (
        StatusCode::OK,
        [("Content-Type".to_string(), "text/html".to_string()), ("Charset".to_string(), "utf-8".to_string())],
        EDITOR_HTML.to_string(),
    )
}

/// Prometheus /metrics endpoint — returns all registered metrics in text format
pub async fn metrics_handler() -> (StatusCode, [(&'static str, &'static str); 1], String) {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let body = String::from_utf8(buffer).unwrap_or_default();
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        body,
    )
}

// ---- API handlers ----

pub async fn api_query(
    State(state): State<Arc<WebState>>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    // Use conn_id=0 for web editor (single-user context)
    let web_conn_id = 0u32;

    // Switch database if specified
    if let Some(ref db) = req.database {
        state.handler.set_database(web_conn_id, db);
    }

    let start = Instant::now();
    let result = state.handler.handle_query(web_conn_id, &req.sql);
    let duration_ms = start.elapsed().as_millis() as u64;

    let has_error = result.columns.iter().any(|c| c.name == "Error");
    let error = if has_error {
        result.rows.first().and_then(|r| r.first()).and_then(|v| v.clone())
    } else {
        None
    };

    let columns: Vec<ColumnInfo> = result.columns.iter().map(|c| ColumnInfo {
        name: c.name.clone(),
        col_type: format!("{:?}", c.col_type),
    }).collect();

    let row_count = result.rows.len();

    // Record in history
    let history_entry = QueryHistoryEntry {
        id: chrono::Utc::now().timestamp_millis() as u64,
        sql: req.sql.clone(),
        database: req.database.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        duration_ms,
        row_count,
        error: error.clone(),
    };
    state.add_history(history_entry).await;

    Json(QueryResponse {
        columns,
        rows: result.rows,
        duration_ms,
        row_count,
        error,
    })
}

pub async fn api_databases(
    State(state): State<Arc<WebState>>,
) -> Json<Vec<String>> {
    let dbs = state.handler.catalog.list_databases();
    Json(dbs)
}

pub async fn api_tables(
    State(state): State<Arc<WebState>>,
    Path(db): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    match state.handler.catalog.list_tables(&db) {
        Some(tables) => Ok(Json(tables)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn api_schema(
    State(state): State<Arc<WebState>>,
    Path((db, table)): Path<(String, String)>,
) -> Result<Json<Vec<ColumnSchema>>, StatusCode> {
    let tbl = state.handler.catalog.get_table(&db, &table)
        .ok_or(StatusCode::NOT_FOUND)?;
    let columns = tbl.columns.iter().map(|c| ColumnSchema {
        name: c.name.clone(),
        data_type: format!("{:?}", c.data_type),
        nullable: c.nullable,
        default_value: c.default_value.as_ref().map(|v| format!("{:?}", v)),
        comment: c.comment.clone(),
    }).collect();
    Ok(Json(columns))
}

pub async fn api_history(
    State(state): State<Arc<WebState>>,
) -> Json<Vec<QueryHistoryEntry>> {
    let history = state.query_history.read().await;
    Json(history.clone())
}

pub async fn api_status(
    State(state): State<Arc<WebState>>,
) -> Json<StatusResponse> {
    let version = state.handler.sys_vars.get("version", None).unwrap_or_else(|| "0.3.0".to_string());
    let db_count = state.handler.catalog.list_databases().len();

    Json(StatusResponse {
        version,
        uptime_seconds: state.connection_tracker.uptime_seconds(),
        total_queries: state.connection_tracker.total_queries(),
        active_connections: state.connection_tracker.active_connections(),
        total_connections: state.connection_tracker.total_connections(),
        databases: db_count,
    })
}
