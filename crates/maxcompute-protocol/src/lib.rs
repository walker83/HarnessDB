//! MaxCompute (ODPS) REST API protocol compatibility layer for RorisDB.
//!
//! This crate implements the MaxCompute REST API protocol, allowing standard
//! MaxCompute SDKs (pyodps, Java SDK, odpscmd) to connect to RorisDB.
//!
//! # Protocol Overview
//! - Transport: HTTP/REST
//! - Authentication: HMAC-SHA1 (V2) or HMAC-SHA256 (V4) signature
//! - Request/Response: XML for metadata, JSON for some operations
//! - SQL Execution: Async job submission with polling
//!
//! # Key Endpoints
//! - `GET /api/projects/{project}` - Project info
//! - `GET /api/projects/{project}/tables` - List tables
//! - `GET /api/projects/{project}/tables/{table}` - Table details
//! - `POST /api/projects/{project}/instances` - Submit SQL job
//! - `GET /api/projects/{project}/instances/{id}` - Instance status
//! - `GET /api/projects/{project}/instances/{id}?result` - Query results

pub mod auth;
pub mod error;
pub mod handlers;
pub mod server;
pub mod sql;
pub mod xml_models;

pub use server::{start_mc_server, McServerConfig, McServerState};

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

/// Wrapper for XML responses with correct Content-Type.
pub struct XmlResponse(pub String);

impl IntoResponse for XmlResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml;charset=UTF-8")],
            self.0,
        )
            .into_response()
    }
}

/// Build a MaxCompute-style XML error response.
pub fn error_xml(code: &str, message: &str) -> String {
    let request_id = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>{code}</Code>
  <Message>{message}</Message>
  <RequestId>{request_id}</RequestId>
</Error>"#,
        code = escape_xml_attr(code),
        message = escape_xml_attr(message),
        request_id = request_id,
    )
}

fn escape_xml_attr(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '\'' => "&apos;".to_string(),
            '"' => "&quot;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
