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

pub use server::{McServerConfig, McServerState, start_mc_server};

use axum::{
    http::{StatusCode, header},
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_xml_basic() {
        let xml = error_xml("ODPS-0130161", "Project 'test' not found");
        assert!(
            xml.contains("<Code>ODPS-0130161</Code>"),
            "Should contain error code"
        );
        assert!(
            xml.contains("<Message>Project &apos;test&apos; not found</Message>"),
            "Should contain escaped message"
        );
        assert!(
            xml.contains("<RequestId>"),
            "Should contain RequestId element"
        );
        assert!(xml.contains("</Error>"), "Should close Error element");
        assert!(
            xml.starts_with("<?xml"),
            "Should start with XML declaration"
        );
    }

    #[test]
    fn test_error_xml_escapes_special_chars() {
        let xml = error_xml("E-001", "value < 10 & value > 5");
        assert!(xml.contains("<Message>value &lt; 10 &amp; value &gt; 5</Message>"));
    }

    #[test]
    fn test_error_xml_escapes_quotes() {
        let xml = error_xml("E-002", "it's \"quoted\"");
        assert!(xml.contains("it&apos;s &quot;quoted&quot;"));
    }

    #[test]
    fn test_escape_xml_attr_ampersand() {
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
    }

    #[test]
    fn test_escape_xml_attr_lt_gt() {
        assert_eq!(escape_xml_attr("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_escape_xml_attr_quotes() {
        assert_eq!(escape_xml_attr("\"hello\""), "&quot;hello&quot;");
        assert_eq!(escape_xml_attr("it's"), "it&apos;s");
    }

    #[test]
    fn test_escape_xml_attr_no_change() {
        assert_eq!(escape_xml_attr("hello world"), "hello world");
        assert_eq!(escape_xml_attr("abc123"), "abc123");
        assert_eq!(escape_xml_attr(""), "");
    }

    #[test]
    fn test_escape_xml_attr_all_special_chars() {
        assert_eq!(
            escape_xml_attr("<a b=\"c&d\">"),
            "&lt;a b=&quot;c&amp;d&quot;&gt;"
        );
    }

    #[test]
    fn test_xml_response_into_response_content_type() {
        let response = XmlResponse("<Root><Item/></Root>".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type, "application/xml;charset=UTF-8");
    }

    #[test]
    fn test_xml_response_into_response_body() {
        let body = "<Root><Item>hello</Item></Root>".to_string();
        let response = XmlResponse(body.clone()).into_response();
        // Use axum's body collection to check the body
        let body_bytes = tokio::runtime::Runtime::new().unwrap().block_on(async {
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
        });
        let response_body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(response_body, body);
    }

    #[test]
    fn test_xml_response_into_response_empty_body() {
        let response = XmlResponse(String::new()).into_response();
        let body_bytes = tokio::runtime::Runtime::new().unwrap().block_on(async {
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
        });
        assert!(body_bytes.is_empty());
    }
}
