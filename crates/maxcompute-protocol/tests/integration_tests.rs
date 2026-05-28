//! HTTP-level integration tests for the MaxCompute REST API protocol.
//!
//! Starts a real axum HTTP server with an in-memory mock query handler
//! and sends signed HTTP requests using `reqwest`.
//!
//! Each test creates its own server on a random port to avoid port conflicts
//! when tests run in parallel.

use std::sync::Arc;

use maxcompute_protocol::auth::{sign_request, McAuthConfig};
use maxcompute_protocol::server::{build_router_with_state, McServerConfig, McServerState};
use mysql_protocol::server::{ColumnDef, ColumnType, QueryHandler, QueryResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TEST_DATE: &str = "Mon, 01 Jan 2024 00:00:00 GMT";
const TEST_ACCESS_KEY: &str = "roris";
const TEST_SECRET_KEY: &str = "roris-secret";

// ---------------------------------------------------------------------------
// Mock query handler
// ---------------------------------------------------------------------------

struct TestHandler;

impl QueryHandler for TestHandler {
    fn handle_query(&self, _conn_id: u32, sql: &str) -> QueryResult {
        let upper = sql.trim().to_uppercase();
        if upper.starts_with("SHOW TABLES") {
            QueryResult::with_rows(
                vec![ColumnDef {
                    name: "table_name".to_string(),
                    col_type: ColumnType::String,
                }],
                vec![
                    vec![Some("test_table".to_string())],
                    vec![Some("users".to_string())],
                ],
            )
        } else if upper.starts_with("DESCRIBE") || upper.starts_with("DESC ") {
            QueryResult::with_rows(
                vec![
                    ColumnDef {
                        name: "Field".to_string(),
                        col_type: ColumnType::String,
                    },
                    ColumnDef {
                        name: "Type".to_string(),
                        col_type: ColumnType::String,
                    },
                ],
                vec![
                    vec![Some("id".to_string()), Some("bigint".to_string())],
                    vec![Some("name".to_string()), Some("string".to_string())],
                ],
            )
        } else if upper.starts_with("SELECT") {
            QueryResult::with_rows(
                vec![ColumnDef {
                    name: "1".to_string(),
                    col_type: ColumnType::String,
                }],
                vec![vec![Some("1".to_string())]],
            )
        } else {
            QueryResult::ok()
        }
    }

    fn set_database(&self, _conn_id: u32, _db: &str) {}

    fn on_connect(&self, _conn_id: u32, _user: &str, _host: &str) {}

    fn on_disconnect(&self, _conn_id: u32) {}
}

// ---------------------------------------------------------------------------
// Test server setup
// ---------------------------------------------------------------------------

/// Start a test server on a random port, returning the port and an HTTP client.
async fn setup_server() -> (u16, reqwest::Client) {
    let handler = Arc::new(TestHandler);
    let config = McServerConfig {
        default_project: "default".to_string(),
        ..McServerConfig::default()
    };
    let state = Arc::new(McServerState::new(handler, config));
    let router = build_router_with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let client = reqwest::Client::new();
    (port, client)
}

// ---------------------------------------------------------------------------
// V2 signing helpers
// ---------------------------------------------------------------------------

/// Build signed headers for a GET request.
fn signed_get_headers(path: &str) -> Vec<(String, String)> {
    let config = McAuthConfig::new_v2(TEST_ACCESS_KEY, TEST_SECRET_KEY);
    let auth = sign_request(&config, "GET", path, "", "", TEST_DATE);
    vec![
        ("Authorization".to_string(), auth),
        ("Date".to_string(), TEST_DATE.to_string()),
    ]
}

/// Build signed headers for a POST request with XML content.
fn signed_post_headers(path: &str) -> Vec<(String, String)> {
    let config = McAuthConfig::new_v2(TEST_ACCESS_KEY, TEST_SECRET_KEY);
    let auth = sign_request(&config, "POST", path, "", "application/xml", TEST_DATE);
    vec![
        ("Authorization".to_string(), auth),
        ("Date".to_string(), TEST_DATE.to_string()),
        ("Content-Type".to_string(), "application/xml".to_string()),
    ]
}

/// Build signed headers with a wrong secret (for auth failure tests).
fn signed_wrong_key_headers(path: &str) -> Vec<(String, String)> {
    let config = McAuthConfig::new_v2("wrong_key", "wrong_secret");
    let auth = sign_request(&config, "GET", path, "", "", TEST_DATE);
    vec![
        ("Authorization".to_string(), auth),
        ("Date".to_string(), TEST_DATE.to_string()),
    ]
}

/// Apply headers from a Vec to a reqwest request builder.
fn apply_headers(
    req: reqwest::RequestBuilder,
    headers: &[(String, String)],
) -> reqwest::RequestBuilder {
    let mut req = req;
    for (name, value) in headers {
        req = req.header(name.as_str(), value.as_str());
    }
    req
}

/// Build a URL for a test server.
fn url(port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{}{}", port, path)
}

// ---------------------------------------------------------------------------
// Test: health endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_endpoint() {
    let (port, client) = setup_server().await;

    let resp = client.get(url(port, "/health")).send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("ok"));
}

// ---------------------------------------------------------------------------
// Test: authenticated get project
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_project_authenticated() {
    let (port, client) = setup_server().await;

    let headers = signed_get_headers("/api/projects/default");
    let resp = apply_headers(client.get(url(port, "/api/projects/default")), &headers)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("application/xml") || content_type.contains("xml"));

    let body = resp.text().await.unwrap();
    assert!(body.contains("<Project>"), "Response should contain Project element: {}", body);
    assert!(body.contains("<Name>default</Name>"), "Response should contain project name: {}", body);
}

// ---------------------------------------------------------------------------
// Test: list tables (authenticated)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_tables_authenticated() {
    let (port, client) = setup_server().await;

    let headers = signed_get_headers("/api/projects/default/tables");
    let resp = apply_headers(client.get(url(port, "/api/projects/default/tables")), &headers)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(body.contains("<Tables>"), "Response should contain Tables element: {}", body);
    assert!(
        body.contains("<Name>test_table</Name>"),
        "Response should list test_table: {}",
        body
    );
    assert!(
        body.contains("<Name>users</Name>"),
        "Response should list users: {}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test: submit instance (SQL job POST)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_submit_instance() {
    let (port, client) = setup_server().await;

    let xml_body = r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"#;

    let headers = signed_post_headers("/api/projects/default/instances");
    let resp = apply_headers(
        client.post(url(port, "/api/projects/default/instances")),
        &headers,
    )
    .body(xml_body)
    .send()
    .await
    .unwrap();

    assert_eq!(
        resp.status(),
        201,
        "Submit instance should return 201 Created"
    );

    // Check Location header
    let location = resp.headers().get("location").and_then(|v| v.to_str().ok());
    assert!(
        location.is_some(),
        "Response should have Location header"
    );
    let location = location.unwrap();
    assert!(
        location.starts_with("/api/projects/default/instances/"),
        "Location should point to instance: {}",
        location
    );
}

// ---------------------------------------------------------------------------
// Test: get instance status
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_instance_status() {
    let (port, client) = setup_server().await;

    // First submit an instance
    let xml_body = r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"#;
    let headers = signed_post_headers("/api/projects/default/instances");
    let submit_resp = apply_headers(
        client.post(url(port, "/api/projects/default/instances")),
        &headers,
    )
    .body(xml_body)
    .send()
    .await
    .unwrap();

    assert_eq!(submit_resp.status(), 201);
    let location = submit_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    // GET the instance status (no query params = full instance info)
    let headers = signed_get_headers(&location);
    let resp = apply_headers(client.get(url(port, &location)), &headers)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(body.contains("<Instance>"), "Response should contain Instance element: {}", body);
    assert!(
        body.contains("<Status>Success</Status>"),
        "Instance status should be Success: {}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test: get instance result
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_instance_result() {
    let (port, client) = setup_server().await;

    // Submit an instance first
    let xml_body = r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"#;
    let headers = signed_post_headers("/api/projects/default/instances");
    let submit_resp = apply_headers(
        client.post(url(port, "/api/projects/default/instances")),
        &headers,
    )
    .body(xml_body)
    .send()
    .await
    .unwrap();

    assert_eq!(submit_resp.status(), 201);
    let location = submit_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    // GET result with ?result query parameter
    let result_path = format!("{}?result", location);
    let headers = signed_get_headers(&result_path);
    let resp = apply_headers(
        client.get(url(port, &result_path)),
        &headers,
    )
    .send()
    .await
    .unwrap();

    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("<Result>"),
        "Result response should contain Result element: {}",
        body
    );
    assert!(
        body.contains("<Status>Success</Status>"),
        "Result should have Success status: {}",
        body
    );
    // SELECT 1 returns CSV: "1\n1\n"
    assert!(
        body.contains("1"),
        "Result should contain query data: {}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test: auth failure with wrong key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_auth_failure() {
    let (port, client) = setup_server().await;

    let headers = signed_wrong_key_headers("/api/projects/default");
    let resp = apply_headers(client.get(url(port, "/api/projects/default")), &headers)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401, "Wrong key should return 401");
}

// ---------------------------------------------------------------------------
// Test: unknown project
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_unknown_project() {
    let (port, client) = setup_server().await;

    let headers = signed_get_headers("/api/projects/nonexistent");
    let resp = apply_headers(
        client.get(url(port, "/api/projects/nonexistent")),
        &headers,
    )
    .send()
    .await
    .unwrap();

    assert_eq!(resp.status(), 404, "Nonexistent project should return 404");

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("<Error>"),
        "404 response should contain Error XML: {}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test: invalid table name (SQL injection prevention)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_invalid_table_name() {
    let (port, client) = setup_server().await;

    let headers = signed_get_headers("/api/projects/default/tables/bad;name");
    let resp = apply_headers(
        client.get(url(port, "/api/projects/default/tables/bad;name")),
        &headers,
    )
    .send()
    .await
    .unwrap();

    assert_eq!(
        resp.status(),
        400,
        "Invalid table name should return 400"
    );

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("<Error>"),
        "400 response should contain Error XML: {}",
        body
    );
}