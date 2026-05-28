//! Route assembly and shared types for MaxCompute REST API handlers.
//!
//! All MaxCompute endpoints are mounted under `/api/` by the server.
//! This module builds the router tree and exports shared types
//! (InstanceManager) used by individual handler modules.

use axum::Router;

pub mod instances;
pub mod projects;
pub mod tables;

/// Build the axum Router with all MaxCompute API endpoints.
///
/// The returned router is parameterized by `Arc<crate::server::McServerState>`
/// so the outer server can call `.with_state(...)` or `.nest("/api", router)`.
pub fn build_router() -> Router<std::sync::Arc<crate::server::McServerState>> {
    Router::new()
        // Project endpoints
        .route("/projects/{project}", axum::routing::get(projects::get_project))
        // Table endpoints
        .route(
            "/projects/{project}/tables",
            axum::routing::get(tables::list_tables),
        )
        .route(
            "/projects/{project}/tables/{table}",
            axum::routing::get(tables::get_table),
        )
        // Instance endpoints
        .route(
            "/projects/{project}/instances",
            axum::routing::post(instances::submit_instance),
        )
        .route(
            "/projects/{project}/instances/{id}",
            axum::routing::get(instances::get_instance),
        )
        .route(
            "/projects/{project}/instances/{id}",
            axum::routing::put(instances::stop_instance),
        )
}

// ---------------------------------------------------------------------------
// InstanceManager – shared, concurrent state for tracking SQL instances
// ---------------------------------------------------------------------------

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mysql_protocol::server::QueryResult;

/// Thread-safe registry of submitted SQL instances.
pub struct InstanceManager {
    instances: DashMap<String, InstanceInfo>,
}

/// The lifecycle status of a submitted SQL instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

impl InstanceStatus {
    /// Return the status string used in MaxCompute XML responses.
    pub fn as_xml_str(&self) -> &'static str {
        match self {
            InstanceStatus::Running => "Running",
            InstanceStatus::Success => "Success",
            InstanceStatus::Failed => "Failed",
            InstanceStatus::Cancelled => "Cancelled",
        }
    }
}

/// Metadata and result of a single submitted SQL instance.
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    pub id: String,
    pub project: String,
    pub sql: String,
    pub status: InstanceStatus,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
}

impl InstanceManager {
    /// Create an empty instance registry.
    pub fn new() -> Self {
        Self {
            instances: DashMap::new(),
        }
    }

    /// Insert a new instance and return the previous value if the ID already existed.
    pub fn insert(&self, info: InstanceInfo) -> Option<InstanceInfo> {
        self.instances.insert(info.id.clone(), info)
    }

    /// Look up an instance by its ID.
    pub fn get(&self, id: &str) -> Option<InstanceInfo> {
        self.instances.get(id).map(|r| r.clone())
    }

    /// Remove an instance from the registry.
    pub fn remove(&self, id: &str) -> Option<(String, InstanceInfo)> {
        self.instances.remove(id)
    }

    /// Update the status of an existing instance.
    pub fn update_status(
        &self,
        id: &str,
        status: InstanceStatus,
        end_time: Option<DateTime<Utc>>,
    ) -> bool {
        if let Some(mut entry) = self.instances.get_mut(id) {
            entry.status = status;
            if let Some(et) = end_time {
                entry.end_time = Some(et);
            }
            true
        } else {
            false
        }
    }

    /// Store the query result for a completed instance.
    pub fn set_result(&self, id: &str, result: QueryResult) -> bool {
        if let Some(mut entry) = self.instances.get_mut(id) {
            entry.result = Some(result);
            entry.status = InstanceStatus::Success;
            entry.end_time = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Store an error for a failed instance.
    pub fn set_error(&self, id: &str, error: String) -> bool {
        if let Some(mut entry) = self.instances.get_mut(id) {
            entry.error = Some(error);
            entry.status = InstanceStatus::Failed;
            entry.end_time = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Number of tracked instances.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Returns `true` if no instances are tracked.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}