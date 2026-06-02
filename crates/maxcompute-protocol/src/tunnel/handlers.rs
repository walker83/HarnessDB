//! MaxCompute Tunnel endpoint handlers.
//!
//! Implements all 7 Tunnel protocol endpoints.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum::extract::Query as QueryExtract;
use tracing::{info, warn};

use crate::server::McServerState;
use crate::tunnel::json::*;

fn json_resp(req_id: &str) -> [(HeaderName, HeaderValue); 2] {
    [
        (header::CONTENT_TYPE, HeaderValue::from_static("application/json")),
        (HeaderName::from_static("x-odps-request-id"), HeaderValue::from_str(req_id).unwrap()),
    ]
}

// ============================================================================
// Dispatcher helpers
// ============================================================================

pub async fn tunnel_endpoint_handler(
    State(state): State<Arc<McServerState>>,
    Path(project): Path<String>,
) -> Response {
    if project != state.config.default_project {
        return error_response(
            StatusCode::NOT_FOUND,
            "ODPS-0130161",
            &format!("Project '{}' not found", project),
        );
    }
    let body = tunnel_endpoint_response(&state.config.bind_addr, state.config.port);
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain")], body).into_response()
}

// ============================================================================
// Create Upload Session
// ============================================================================

pub async fn create_upload_session(
    State(state): State<Arc<McServerState>>,
    Path((project, table)): Path<(String, String)>,
    _headers: HeaderMap,
) -> Response {
    if project != state.config.default_project {
        return error_response(StatusCode::NOT_FOUND, "ODPS-0130161",
            &format!("Project '{}' not found", project));
    }
    if !validate_sql_identifier(&table) {
        return error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "Invalid table name");
    }

    let conn_id = state.next_conn_id();
    match state.tunnel_session_manager.create_upload_session(&project, &table, &state.handler, conn_id) {
        Ok(session) => {
            let resp = CreateUploadResponse::new(session.upload_id.clone(), session.schema);
            let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
            let req_id = uuid::Uuid::new_v4().to_string();
            (StatusCode::OK, json_resp(&req_id), body).into_response()
        }
        Err(e) => {
            warn!("Failed to create upload session for {}.{}: {}", project, table, e);
            error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &e)
        }
    }
}

// ============================================================================
// Upload Block
// ============================================================================

pub async fn upload_block(
    State(state): State<Arc<McServerState>>,
    Path((_project, _table)): Path<(String, String)>,
    headers: HeaderMap,
    query: QueryExtract<std::collections::HashMap<String, String>>,
    body: Body,
) -> Response {
    let params = query.0;
    let upload_id = match params.get("uploadid") {
        Some(id) => id.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "missing uploadid parameter"),
    };
    let block_id: u64 = match params.get("blockid").and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => return error_response(StatusCode::BAD_REQUEST, "InvalidParameter", "invalid blockid parameter"),
    };

    let body_bytes = match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "InvalidRequest", "failed to read body"),
    };

    let content_encoding = headers.get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok()).unwrap_or("");

    let data = if content_encoding.eq_ignore_ascii_case("deflate") {
        match crate::tunnel::compression::decompress_deflate(&body_bytes) {
            Ok(d) => d,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidRequest", &format!("decompression failed: {}", e)),
        }
    } else {
        body_bytes.to_vec()
    };

    let schema = match state.tunnel_session_manager.get_upload_session_schema(&upload_id) {
        Some(s) => s,
        None => return error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &format!("Upload session not found: {}", upload_id)),
    };

    let mut reader = crate::tunnel::io::TunnelReader::new(&data);
    let mut rows = Vec::new();
    loop {
        match reader.read_row(&schema) {
            Ok(Some(row)) => rows.push(row),
            Ok(None) => break,
            Err(e) => {
                warn!("Failed to decode upload block: {}", e);
                return error_response(StatusCode::BAD_REQUEST, "InvalidRequest", &format!("failed to decode block data: {}", e));
            }
        }
    }

    info!("Upload block {} to session {}: {} records", block_id, upload_id, rows.len());

    if let Err(e) = state.tunnel_session_manager.upload_block(&upload_id, block_id, rows) {
        return error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &e);
    }

    (StatusCode::OK, [(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))], "{}").into_response()
}

// ============================================================================
// Commit Upload
// ============================================================================

pub async fn commit_upload(
    State(state): State<Arc<McServerState>>,
    Path((_project, _table)): Path<(String, String)>,
    _headers: HeaderMap,
    query: QueryExtract<std::collections::HashMap<String, String>>,
) -> Response {
    let params = query.0;
    let upload_id = match params.get("uploadid") {
        Some(id) => id.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "missing uploadid parameter"),
    };

    let conn_id = state.next_conn_id();
    match state.tunnel_session_manager.commit_upload(&upload_id, &state.handler, conn_id) {
        Ok(block_ids) => {
            let resp = CommitUploadResponse::new(block_ids);
            let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
            let req_id = uuid::Uuid::new_v4().to_string();
            (StatusCode::OK, json_resp(&req_id), body).into_response()
        }
        Err(e) => {
            warn!("Commit failed for upload session {}: {}", upload_id, e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "MetaTransactionFailed", &format!("Commit failed: {}", e))
        }
    }
}

// ============================================================================
// Create Download Session
// ============================================================================

pub async fn create_download_session(
    State(state): State<Arc<McServerState>>,
    Path((project, table)): Path<(String, String)>,
    _headers: HeaderMap,
) -> Response {
    if project != state.config.default_project {
        return error_response(StatusCode::NOT_FOUND, "ODPS-0130161",
            &format!("Project '{}' not found", project));
    }
    if !validate_sql_identifier(&table) {
        return error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "Invalid table name");
    }

    let conn_id = state.next_conn_id();
    match state.tunnel_session_manager.create_download_session(&project, &table, &state.handler, conn_id) {
        Ok(session) => {
            let resp = CreateDownloadResponse::new(session.download_id.clone(), session.schema, session.record_count);
            let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
            let req_id = uuid::Uuid::new_v4().to_string();
            (StatusCode::OK, json_resp(&req_id), body).into_response()
        }
        Err(e) => {
            warn!("Failed to create download session for {}.{}: {}", project, table, e);
            error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &e)
        }
    }
}

// ============================================================================
// Download Data
// ============================================================================

pub async fn download_data(
    State(state): State<Arc<McServerState>>,
    Path((_project, _table)): Path<(String, String)>,
    headers: HeaderMap,
    query: QueryExtract<std::collections::HashMap<String, String>>,
) -> Response {
    let params = query.0;
    let download_id = match params.get("downloadid") {
        Some(id) => id.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "missing downloadid parameter"),
    };

    let (row_start, row_count) = match parse_rowrange(params.get("rowrange").map(|s| s.as_str())) {
        Some(v) => v,
        None => return error_response(StatusCode::BAD_REQUEST, "InvalidParameter", "invalid rowrange, expected (start,count)"),
    };

    let (rows, schema) = {
        match state.tunnel_session_manager.get_download_session(&download_id) {
            Some(session) => {
                let start = row_start as usize;
                let end = (start + row_count as usize).min(session.cached_data.len());
                let rows = if start >= session.cached_data.len() { vec![] } else { session.cached_data[start..end].to_vec() };
                (rows, session.schema)
            }
            None => return error_response(StatusCode::NOT_FOUND, "ObjectNotFound", "Download session not found"),
        }
    };

    let data = crate::tunnel::io::TunnelWriter::new().finish(&rows, &schema);

    let accept_encoding = headers.get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok()).unwrap_or("");

    let (data, content_encoding): (Vec<u8>, &str) = if accept_encoding.contains("deflate") && data.len() > 100 {
        match crate::tunnel::compression::compress_deflate(&data) {
            Ok(c) => (c, "deflate"),
            Err(_) => (data, ""),
        }
    } else {
        (data, "")
    };

    let req_id = uuid::Uuid::new_v4().to_string();
    let mut response_headers = axum::http::HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    response_headers.insert(HeaderName::from_static("x-odps-request-id"), HeaderValue::from_str(&req_id).unwrap());
    if !content_encoding.is_empty() {
        response_headers.insert(header::CONTENT_ENCODING, HeaderValue::from_str(content_encoding).unwrap());
    }

    (StatusCode::OK, response_headers, data).into_response()
}

// ============================================================================
// Reload Session
// ============================================================================

pub async fn reload_session(
    State(state): State<Arc<McServerState>>,
    Path((_project, _table)): Path<(String, String)>,
    query: QueryExtract<std::collections::HashMap<String, String>>,
) -> Response {
    let params = query.0;

    if let Some(upload_id) = params.get("uploadid") {
        match state.tunnel_session_manager.reload_upload_session(upload_id) {
            Some((session, block_ids)) => {
                let resp = ReloadUploadResponse::new(session.upload_id, session.schema, block_ids);
                let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
                let req_id = uuid::Uuid::new_v4().to_string();
                (StatusCode::OK, json_resp(&req_id), body).into_response()
            }
            None => error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &format!("Upload session not found: {}", upload_id)),
        }
    } else if let Some(download_id) = params.get("downloadid") {
        match state.tunnel_session_manager.reload_download_session(download_id) {
            Some(session) => {
                let resp = ReloadDownloadResponse::new(session.download_id, session.schema, session.record_count);
                let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
                let req_id = uuid::Uuid::new_v4().to_string();
                (StatusCode::OK, json_resp(&req_id), body).into_response()
            }
            None => error_response(StatusCode::NOT_FOUND, "ObjectNotFound", &format!("Download session not found: {}", download_id)),
        }
    } else {
        error_response(StatusCode::BAD_REQUEST, "InvalidArgument", "missing uploadid or downloadid parameter")
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    let err = TunnelError::new(code, message);
    let body = err.to_json();
    let req_id = HeaderValue::from_str(&err.request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown"));
    (
        status,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("application/json")),
            (HeaderName::from_static("x-odps-request-id"), req_id),
        ],
        body,
    ).into_response()
}

fn parse_rowrange(param: Option<&str>) -> Option<(u64, u64)> {
    let p = param?;
    let trimmed = p.trim_matches(|c: char| c == '(' || c == ')');
    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.len() != 2 { return None; }
    let start = parts[0].trim().parse::<u64>().ok()?;
    let count = parts[1].trim().parse::<u64>().ok()?;
    Some((start, count))
}

fn validate_sql_identifier(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 { return false; }
    name.chars().enumerate().all(|(i, c)| {
        if i == 0 { c.is_alphabetic() || c == '_' || c == '$' }
        else { c.is_alphanumeric() || c == '_' || c == '$' }
    })
}
