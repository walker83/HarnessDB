//! Handlers for instance (SQL job) MaxCompute REST API endpoints.

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::server::McServerState;
use crate::xml_models;
use crate::XmlResponse;

/// Check if a QueryResult indicates an error by looking at the first column name.
///
/// Matches "Error", "ERROR", and "PARSE ERROR" (case-insensitive).
fn is_error_result(result: &mysql_protocol::server::QueryResult) -> bool {
    if result.columns.is_empty() {
        return false;
    }
    let name = result.columns[0].name.to_uppercase();
    name == "ERROR" || name == "PARSE ERROR"
}

/// Escape `]]>` sequences inside CDATA sections so the XML remains well-formed.
fn escape_cdata(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
}

/// `POST /api/projects/{project}/instances` — submit a SQL job.
pub async fn submit_instance(
    State(state): State<Arc<McServerState>>,
    Path(project): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    info!("POST /api/projects/{}/instances", project);

    if !project.eq_ignore_ascii_case(&state.config.default_project) {
        return (
            StatusCode::NOT_FOUND,
            crate::error_xml("ODPS-0420111", &format!("Project '{}' not found", project)),
        )
            .into_response();
    }

    // Extract SQL from the XML body
    let sql = xml_models::extract_sql_from_body(&body);
    if sql.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            crate::error_xml("ODPS-0130011", "Empty SQL query"),
        )
            .into_response();
    }

    info!("MaxCompute SQL: {}", sql);

    // Translate MaxCompute SQL to RorisDB SQL
    let (translated_sql, is_noop) = crate::sql::translate_mc_sql(&sql);

    if is_noop {
        // Return an immediately-completed instance for no-op statements
        let instance_id = Uuid::new_v4().to_string();
        let info = crate::handlers::InstanceInfo {
            id: instance_id.clone(),
            project: project.clone(),
            sql: sql.clone(),
            status: crate::handlers::InstanceStatus::Success,
            result: Some(mysql_protocol::server::QueryResult::ok()),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let location = format!("/api/projects/{}/instances/{}", project, instance_id);
        return (
            StatusCode::CREATED,
            [(header::LOCATION, location)],
            XmlResponse(String::new()),
        )
            .into_response();
    }

    // Generate instance ID
    let instance_id = Uuid::new_v4().to_string();

    // Execute in a blocking thread to avoid blocking the async worker
    let handler = state.handler.clone();
    let conn_id = state.next_conn_id();
    let result = tokio::task::spawn_blocking(move || {
        handler.handle_query(conn_id, &translated_sql)
    })
    .await
    .unwrap_or_else(|join_err| {
        error!("Blocking task join error: {}", join_err);
        mysql_protocol::server::QueryResult::ok()
    });

    // Check if the result is an error
    let is_error = is_error_result(&result);

    let (status, error_msg) = if is_error {
        let msg = result
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_deref())
            .unwrap_or("Unknown error")
            .to_string();
        error!("MaxCompute SQL failed: {}", msg);
        (crate::handlers::InstanceStatus::Failed, Some(msg))
    } else {
        (crate::handlers::InstanceStatus::Success, None)
    };

    let now = chrono::Utc::now();
    let info = crate::handlers::InstanceInfo {
        id: instance_id.clone(),
        project: project.clone(),
        sql: sql.clone(),
        status: status.clone(),
        result: Some(result),
        error: error_msg,
        start_time: now,
        end_time: Some(now),
    };
    state.instance_manager.insert(info);

    let location = format!("/api/projects/{}/instances/{}", project, instance_id);

    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        XmlResponse(String::new()),
    )
        .into_response()
}

/// `GET /api/projects/{project}/instances/{id}` — get instance status, task status, or results.
///
/// MaxCompute uses query parameters to distinguish operations:
/// - No params → instance status
/// - `?taskstatus` → task status
/// - `?result` → task results
/// - `?instancestatus` → instance status (may block, we return immediately)
pub async fn get_instance(
    State(state): State<Arc<McServerState>>,
    Path((project, id)): Path<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    info!("GET /api/projects/{}/instances/{} params={:?}", project, id, params.keys().collect::<Vec<_>>());

    let instance = match state.instance_manager.get(&id) {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                crate::error_xml("ODPS-0120035", &format!("Instance '{}' not found", id)),
            )
                .into_response();
        }
    };

    let now = xml_models::now_iso8601();
    let start_time = instance.start_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let end_time = instance
        .end_time
        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| now.clone());
    let status_str = instance.status.as_xml_str();

    // Check which query parameter is present
    if params.contains_key("result") {
        return build_result_response(&instance, &now);
    }

    if params.contains_key("taskstatus") {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Tasks>
    <Task Name="AnonymousSQLTask" Type="SQL" Status="{status}"/>
  </Tasks>
</Instance>"#,
            status = status_str,
        );
        return (StatusCode::OK, XmlResponse(xml)).into_response();
    }

    if params.contains_key("instancestatus") {
        // Return just the status (used for blocking poll)
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Status>{status}</Status>
</Instance>"#,
            status = status_str,
        );
        return (StatusCode::OK, XmlResponse(xml)).into_response();
    }

    // Default: full instance info
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Name>{id}</Name>
  <Owner>root</Owner>
  <StartTime>{start}</StartTime>
  <EndTime>{end}</EndTime>
  <Status>{status}</Status>
</Instance>"#,
        id = id,
        start = start_time,
        end = end_time,
        status = status_str,
    );

    (StatusCode::OK, XmlResponse(xml)).into_response()
}

/// `PUT /api/projects/{project}/instances/{id}` — stop/cancel an instance.
pub async fn stop_instance(
    State(state): State<Arc<McServerState>>,
    Path((project, id)): Path<(String, String)>,
) -> impl IntoResponse {
    info!("PUT /api/projects/{}/instances/{} (stop)", project, id);

    if state.instance_manager.cancel(&id) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            crate::error_xml("ODPS-0120035", &format!("Instance '{}' not found", id)),
        )
            .into_response()
    }
}

/// Build the task result XML response.
fn build_result_response(
    instance: &crate::handlers::InstanceInfo,
    _now: &str,
) -> Response {
    if instance.status == crate::handlers::InstanceStatus::Failed {
        let error_msg = instance
            .error
            .as_deref()
            .unwrap_or("Query execution failed");
        return (
            StatusCode::OK,
            XmlResponse(format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Tasks>
    <Task Type="SQL">
      <Name>AnonymousSQLTask</Name>
      <Status>Failed</Status>
      <Result><![CDATA[{}]]></Result>
    </Task>
  </Tasks>
</Instance>"#, escape_cdata(error_msg)
            )),
        )
            .into_response();
    }

    let result = match &instance.result {
        Some(r) => r,
        None => {
            return (
                StatusCode::OK,
                XmlResponse(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Tasks>
    <Task Type="SQL">
      <Name>AnonymousSQLTask</Name>
      <Status>Success</Status>
      <Result><![CDATA[]]></Result>
    </Task>
  </Tasks>
</Instance>"#
                    .to_string(),
                ),
            )
                .into_response();
        }
    };

    // Check if it's a DDL (no columns)
    if result.columns.is_empty() {
        return (
            StatusCode::OK,
            XmlResponse(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Tasks>
    <Task Type="SQL">
      <Name>AnonymousSQLTask</Name>
      <Status>Success</Status>
      <Result><![CDATA[]]></Result>
      <Result>
        <SelectResultStatus>OK</SelectResultStatus>
        <IsSelect>false</IsSelect>
      </Result>
    </Task>
  </Tasks>
</Instance>"#
                .to_string(),
            ),
        )
            .into_response();
    }

    // Build CSV result
    let mut csv = String::new();
    // Header
    let header: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    csv.push_str(&header.join(","));
    csv.push('\n');
    // Rows
    for row in &result.rows {
        let vals: Vec<String> = row
            .iter()
            .map(|v| {
                v.as_deref()
                    .unwrap_or("NULL")
                    .replace(',', "\\,")
                    .replace('\n', "\\n")
            })
            .collect();
        csv.push_str(&vals.join(","));
        csv.push('\n');
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Tasks>
    <Task Type="SQL">
      <Name>AnonymousSQLTask</Name>
      <Status>Success</Status>
      <Result><![CDATA[{csv}]]></Result>
      <Result>
        <SelectResultStatus>OK</SelectResultStatus>
        <IsSelect>true</IsSelect>
      </Result>
    </Task>
  </Tasks>
</Instance>"#,
        csv = escape_cdata(&csv),
    );

    (StatusCode::OK, XmlResponse(xml)).into_response()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::{InstanceInfo, InstanceStatus, InstanceManager};
    use crate::server::{McServerConfig, McServerState, MockQueryHandler};
    use axum::extract::Query;
    use std::collections::HashMap;

    fn make_state(default_project: &str) -> Arc<McServerState> {
        let config = McServerConfig {
            default_project: default_project.to_string(),
            ..McServerConfig::default()
        };
        let handler = Arc::new(MockQueryHandler::new());
        Arc::new(McServerState::new(handler, config))
    }

    // ======================================================================
    // submit_instance tests
    // ======================================================================

    #[tokio::test]
    async fn test_submit_instance_sql_ok() {
        let state = make_state("test_project");
        let body = Bytes::from(
            r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"#,
        );

        let response = submit_instance(
            State(state),
            Path("test_project".to_string()),
            body,
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED, "Submit should return 201");
        let location = response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok());
        assert!(location.is_some(), "Should have Location header");
        assert!(location.unwrap().starts_with("/api/projects/test_project/instances/"));
    }

    #[tokio::test]
    async fn test_submit_instance_wrong_project() {
        let state = make_state("test_project");
        let body = Bytes::from(
            r#"<Instance><Job><Tasks><SQL><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"#,
        );

        let response = submit_instance(
            State(state),
            Path("nonexistent".to_string()),
            body,
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_submit_instance_empty_sql() {
        let state = make_state("test_project");
        let body = Bytes::from(
            r#"<Instance><Job><Tasks><SQL><Name>AnonymousSQLTask</Name><Query></Query></SQL></Tasks></Job></Instance>"#,
        );

        let response = submit_instance(
            State(state),
            Path("test_project".to_string()),
            body,
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_submit_instance_show_tables_is_noop() {
        let state = make_state("test_project");
        let body = Bytes::from(
            r#"<Instance><Job><Priority>9</Priority><Tasks><SQL><Name>AnonymousSQLTask</Name><Query>SHOW TABLES</Query></SQL></Tasks></Job></Instance>"#,
        );

        let response = submit_instance(
            State(state.clone()),
            Path("test_project".to_string()),
            body,
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let location = response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()).unwrap().to_string();

        // The instance should be immediately completed with Success
        let instance_id = location.rsplit('/').next().unwrap();
        let info = state.instance_manager.get(instance_id);
        assert!(info.is_some(), "Instance should exist in manager");
        assert_eq!(info.unwrap().status, InstanceStatus::Success);
    }

    // ======================================================================
    // get_instance tests
    // ======================================================================

    #[tokio::test]
    async fn test_get_instance_not_found() {
        let state = make_state("test_project");

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), "nonexistent-id".to_string())),
            Query(HashMap::new()),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_instance_full_info() {
        let state = make_state("test_project");
        let instance_id = "test-instance-1";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Success,
            result: Some(mysql_protocol::server::QueryResult::ok()),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(HashMap::new()),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Instance>"), "Should contain Instance element");
        assert!(body_str.contains("<Status>Success</Status>"), "Should contain status");
    }

    #[tokio::test]
    async fn test_get_instance_with_result_param() {
        let state = make_state("test_project");
        let instance_id = "test-instance-2";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Success,
            result: Some(mysql_protocol::server::QueryResult::with_rows(
                vec![mysql_protocol::server::ColumnDef {
                    name: "result".to_string(),
                    col_type: mysql_protocol::server::ColumnType::String,
                }],
                vec![vec![Some("1".to_string())]],
            )),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("result".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Result>"), "Should contain Result element");
        assert!(body_str.contains("<IsSelect>true</IsSelect>"), "Should indicate SELECT query");
    }

    #[tokio::test]
    async fn test_get_instance_with_taskstatus_param() {
        let state = make_state("test_project");
        let instance_id = "test-instance-taskstatus";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Running,
            result: None,
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("taskstatus".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Task Name=\"AnonymousSQLTask\""));
        assert!(body_str.contains("Status=\"Running\""), "TaskStatus XML should contain Status=Running: {}", body_str);
    }

    #[tokio::test]
    async fn test_get_instance_with_instancestatus_param() {
        let state = make_state("test_project");
        let instance_id = "test-instance-istatus";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Failed,
            result: None,
            error: Some("syntax error".to_string()),
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("instancestatus".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Status>Failed</Status>"));
    }

    // ======================================================================
    // stop_instance tests
    // ======================================================================

    #[tokio::test]
    async fn test_stop_instance_found() {
        let state = make_state("test_project");
        let instance_id = "test-instance-stop";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Running,
            result: None,
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
        };
        state.instance_manager.insert(info);

        let response = stop_instance(
            State(state.clone()),
            Path(("test_project".to_string(), instance_id.to_string())),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let retrieved = state.instance_manager.get(instance_id).unwrap();
        assert_eq!(retrieved.status, InstanceStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_stop_instance_not_found() {
        let state = make_state("test_project");

        let response = stop_instance(
            State(state),
            Path(("test_project".to_string(), "nonexistent-id".to_string())),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ======================================================================
    // build_result_response tests (via get_instance with ?result)
    // ======================================================================

    #[tokio::test]
    async fn test_get_instance_result_failed_instance() {
        let state = make_state("test_project");
        let instance_id = "failed-instance";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "BAD SQL".to_string(),
            status: InstanceStatus::Failed,
            result: None,
            error: Some("Syntax error near 'BAD'".to_string()),
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("result".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Status>Failed</Status>"));
        assert!(body_str.contains("Syntax error near"));
    }

    #[tokio::test]
    async fn test_get_instance_result_ddl_no_columns() {
        let state = make_state("test_project");
        let instance_id = "ddl-instance";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "CREATE TABLE t (id INT)".to_string(),
            status: InstanceStatus::Success,
            result: Some(mysql_protocol::server::QueryResult::ok()),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("result".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<IsSelect>false</IsSelect>"), "DDL should have IsSelect=false");
    }

    #[tokio::test]
    async fn test_get_instance_result_no_result_none() {
        let state = make_state("test_project");
        let instance_id = "no-result-instance";
        let info = InstanceInfo {
            id: instance_id.to_string(),
            project: "test_project".to_string(),
            sql: "SET x = y".to_string(),
            status: InstanceStatus::Success,
            result: None,
            error: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
        };
        state.instance_manager.insert(info);

        let mut params = HashMap::new();
        params.insert("result".to_string(), String::new());

        let response = get_instance(
            State(state),
            Path(("test_project".to_string(), instance_id.to_string())),
            Query(params),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<Result><![CDATA[]]></Result>"), "No-result instance should have empty CDATA");
    }

    // ======================================================================
    // is_error_result tests (existing)
    // ======================================================================

    #[test]
    fn test_is_error_result_detects_error() {
        let result = mysql_protocol::server::QueryResult::with_rows(
            vec![mysql_protocol::server::ColumnDef {
                name: "Error".to_string(),
                col_type: mysql_protocol::server::ColumnType::String,
            }],
            vec![vec![Some("syntax error".to_string())]],
        );
        assert!(is_error_result(&result));
    }

    #[test]
    fn test_is_error_result_detects_parse_error() {
        let result = mysql_protocol::server::QueryResult::with_rows(
            vec![mysql_protocol::server::ColumnDef {
                name: "PARSE ERROR".to_string(),
                col_type: mysql_protocol::server::ColumnType::String,
            }],
            vec![vec![Some("parse error".to_string())]],
        );
        assert!(is_error_result(&result));
    }

    #[test]
    fn test_is_error_result_case_insensitive() {
        let result = mysql_protocol::server::QueryResult::with_rows(
            vec![mysql_protocol::server::ColumnDef {
                name: "error".to_string(),
                col_type: mysql_protocol::server::ColumnType::String,
            }],
            vec![vec![Some("err".to_string())]],
        );
        assert!(is_error_result(&result));
    }

    #[test]
    fn test_is_error_result_no_columns() {
        let result = mysql_protocol::server::QueryResult::ok();
        assert!(!is_error_result(&result));
    }

    #[test]
    fn test_is_error_result_normal_query() {
        let result = mysql_protocol::server::QueryResult::with_rows(
            vec![mysql_protocol::server::ColumnDef {
                name: "id".to_string(),
                col_type: mysql_protocol::server::ColumnType::String,
            }],
            vec![vec![Some("1".to_string())]],
        );
        assert!(!is_error_result(&result));
    }

    #[test]
    fn test_escape_cdata_no_change() {
        assert_eq!(escape_cdata("hello world"), "hello world");
        assert_eq!(escape_cdata(""), "");
        assert_eq!(escape_cdata("normal data"), "normal data");
    }

    #[test]
    fn test_escape_cdata_with_terminator() {
        let input = "data with ]]> inside";
        let expected = "data with ]]]]><![CDATA[> inside";
        assert_eq!(escape_cdata(input), expected);
    }

    #[test]
    fn test_escape_cdata_multiple_terminators() {
        let input = "a]]>b]]>c";
        let expected = "a]]]]><![CDATA[>b]]]]><![CDATA[>c";
        assert_eq!(escape_cdata(input), expected);
    }

    #[test]
    fn test_escape_cdata_partial_match() {
        // Should not escape partial sequences
        assert_eq!(escape_cdata("]]"), "]]");
        assert_eq!(escape_cdata("]>"), "]>");
        assert_eq!(escape_cdata("]"), "]");
    }
}
