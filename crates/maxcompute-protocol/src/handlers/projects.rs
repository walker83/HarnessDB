//! Handler for `GET /api/projects/{project}`.
//!
//! Returns project metadata as XML. The MaxCompute project concept maps to a
//! HarnessDB database. If the project name differs from the configured default
//! project, a 404 error is returned.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::info;

use crate::XmlResponse;
use crate::server::McServerState;

/// Retrieve project (database) information.
///
/// MaxCompute equivalent: `GetProject` / `DescribeProject`.
pub async fn get_project(
    State(state): State<Arc<McServerState>>,
    Path(project): Path<String>,
) -> impl IntoResponse {
    info!("GET /api/projects/{}", project);

    // Validate the project exists. For now, require it matches the default project.
    // In the future this could be extended to discover databases from the catalog.
    if !project.eq_ignore_ascii_case(&state.config.default_project) {
        return (
            StatusCode::NOT_FOUND,
            crate::error_xml(
                "ODPS-0130161",
                &format!("Project '{}' not found or does not exist", project),
            ),
        )
            .into_response();
    }

    // Build the MaxCompute project XML response.
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <Name>{project}</Name>
  <Owner>{owner}</Owner>
  <ProjectGroupName>default</ProjectGroupName>
  <Properties>
    <Property>
      <Name>odps.sql.allow.fullscan</Name>
      <Value>true</Value>
    </Property>
  </Properties>
</Project>"#,
        project = escape_xml(&project),
        owner = "root",
    );

    (StatusCode::OK, XmlResponse(xml)).into_response()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{McServerConfig, McServerState, MockQueryHandler};
    use std::sync::Arc;

    fn make_state() -> Arc<McServerState> {
        let config = McServerConfig {
            default_project: "test_project".to_string(),
            ..McServerConfig::default()
        };
        let handler = Arc::new(MockQueryHandler::new());
        Arc::new(McServerState::new(handler, config))
    }

    #[tokio::test]
    async fn test_get_project_found() {
        let state = make_state();
        let resp = get_project(State(state), Path("test_project".to_string())).await;
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_project_not_found() {
        let state = make_state();
        let resp = get_project(State(state), Path("nonexistent".to_string())).await;
        let resp = resp.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Escapes special XML characters in a string.
pub(crate) fn escape_xml(s: &str) -> String {
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
