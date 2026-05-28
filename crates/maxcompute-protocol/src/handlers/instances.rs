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
