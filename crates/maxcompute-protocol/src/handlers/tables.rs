//! Handlers for table listing and detail endpoints.
//!
//! - `GET /api/projects/{project}/tables`       → list_tables
//! - `GET /api/projects/{project}/tables/{table}` → get_table
//!
//! Both delegate to the `QueryHandler` and format results as MaxCompute XML.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

use crate::server::McServerState;
use crate::XmlResponse;

/// Optional query parameters for `list_tables`.
#[derive(Debug, Deserialize)]
pub struct ListTablesParams {
    #[serde(rename = "maxitem")]
    pub max_item: Option<u32>,
    #[serde(rename = "marker")]
    pub marker: Option<String>,
    #[serde(rename = "prefix")]
    pub prefix: Option<String>,
}

/// List all tables in the given project (database).
///
/// Uses `SHOW TABLES FROM <project>` internally.
pub async fn list_tables(
    State(state): State<Arc<McServerState>>,
    Path(project): Path<String>,
    Query(_params): Query<ListTablesParams>,
) -> impl IntoResponse {
    info!("GET /api/projects/{}/tables", project);

    // Validate the project.
    if !project.eq_ignore_ascii_case(&state.config.default_project) {
        return (
            StatusCode::NOT_FOUND,
            crate::error_xml(
                "ODPS-0130161",
                &format!("Project '{}' not found", project),
            ),
        )
            .into_response();
    }

    // Execute SHOW TABLES and extract table names.
    let result = state.handler.handle_query(0, "SHOW TABLES");

    let table_names: Vec<String> = result
        .rows
        .iter()
        .map(|r| r.first().and_then(|v| v.clone()).unwrap_or_default())
        .filter(|n| !n.is_empty())
        .collect();

    // Build the MaxCompute Tables XML.
    let tables_xml: String = table_names
        .iter()
        .map(|name| {
            format!(
                r#"  <Table>
    <Name>{name}</Name>
    <Owner>root</Owner>
    <Type>MANAGED_TABLE</Type>
    <Comment />
  </Table>"#,
                name = crate::handlers::projects::escape_xml(name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Tables>
{tables}
</Tables>"#,
        tables = tables_xml
    );

    (StatusCode::OK, XmlResponse(xml)).into_response()
}

/// Describe a single table in detail.
///
/// Uses `DESCRIBE <table>` internally and maps columns/types to MaxCompute format.
pub async fn get_table(
    State(state): State<Arc<McServerState>>,
    Path((project, table)): Path<(String, String)>,
) -> impl IntoResponse {
    info!("GET /api/projects/{}/tables/{}", project, table);

    // Validate the project.
    if !project.eq_ignore_ascii_case(&state.config.default_project) {
        return (
            StatusCode::NOT_FOUND,
            crate::error_xml(
                "ODPS-0130161",
                &format!("Project '{}' not found", project),
            ),
        )
            .into_response();
    }

    // Execute DESCRIBE to get column info.
    let describe_result = state.handler.handle_query(0, &format!("DESCRIBE {}", table));

    if describe_result.columns.is_empty() && describe_result.rows.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            crate::error_xml(
                "ODPS-0130161",
                &format!("Table '{}' not found in project '{}'", table, project),
            ),
        )
            .into_response();
    }

    // Parse DESCRIBE output. Typically columns are: Field, Type, Null, Key, Default, Extra
    // We map MySQL types to MaxCompute types.
    let columns_xml: String = describe_result
        .rows
        .iter()
        .map(|row| {
            let col_name = row.first().and_then(|v| v.as_deref()).unwrap_or("");
            let col_type = row.get(1).and_then(|v| v.as_deref()).unwrap_or("string");
            let mc_type = mysql_type_to_maxcompute(col_type);
            format!(
                r#"    <Column>
      <Name>{name}</Name>
      <Type>{mctype}</Type>
      <Comment />
    </Column>"#,
                name = crate::handlers::projects::escape_xml(col_name),
                mctype = mc_type
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Table>
  <Name>{table}</Name>
  <Owner>root</Owner>
  <Type>MANAGED_TABLE</Type>
  <Columns>
{columns}
  </Columns>
</Table>"#,
        table = crate::handlers::projects::escape_xml(&table),
        columns = columns_xml
    );

    (StatusCode::OK, XmlResponse(xml)).into_response()
}

/// Map a MySQL type name to its MaxCompute equivalent.
fn mysql_type_to_maxcompute(mysql_type: &str) -> &'static str {
    let lower = mysql_type.to_lowercase();
    match lower.as_str() {
        // Numeric
        "tinyint" | "smallint" | "int" | "integer" | "bigint" => "bigint",
        "float" | "double" | "decimal" | "numeric" | "real" => "double",
        // String
        "char" | "varchar" | "tinytext" | "text" | "mediumtext" | "longtext"
        | "enum" | "set" => "string",
        "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => "binary",
        // Date/Time
        "date" => "datetime",
        "datetime" | "timestamp" => "datetime",
        "time" | "year" => "string",
        // Boolean
        "bool" | "boolean" => "boolean",
        // Default
        _ => "string",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_type_to_maxcompute() {
        assert_eq!(mysql_type_to_maxcompute("int"), "bigint");
        assert_eq!(mysql_type_to_maxcompute("VARCHAR"), "string");
        assert_eq!(mysql_type_to_maxcompute("datetime"), "datetime");
        assert_eq!(mysql_type_to_maxcompute("bool"), "boolean");
        assert_eq!(mysql_type_to_maxcompute("blob"), "binary");
        assert_eq!(mysql_type_to_maxcompute("unknown_type"), "string");
    }

    #[tokio::test]
    async fn test_list_tables_project_not_found() {
        use crate::server::{McServerConfig, McServerState, MockQueryHandler};
        let state = Arc::new(McServerState::new(
            Arc::new(MockQueryHandler::new()),
            McServerConfig {
                default_project: "myproject".to_string(),
                ..McServerConfig::default()
            },
        ));
        let resp = list_tables(
            State(state),
            Path("wrong".to_string()),
            Query(ListTablesParams {
                max_item: None,
                marker: None,
                prefix: None,
            }),
        )
        .await;
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_table_project_not_found() {
        use crate::server::{McServerConfig, McServerState, MockQueryHandler};
        let state = Arc::new(McServerState::new(
            Arc::new(MockQueryHandler::new()),
            McServerConfig {
                default_project: "myproject".to_string(),
                ..McServerConfig::default()
            },
        ));
        let resp = get_table(
            State(state),
            Path(("wrong".to_string(), "mytable".to_string())),
        )
        .await;
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}