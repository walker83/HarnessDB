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

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use mysql_protocol::server::QueryResult;

const MAX_INSTANCES: usize = 10000;
const EVICTION_TARGET: usize = 8000; // 80% capacity
const INSTANCE_TTL_SECS: i64 = 3600; // 1 hour

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

    /// Insert a new instance.
    ///
    /// If the registry is at capacity, oldest entries are evicted first.
    pub fn insert(&self, info: InstanceInfo) -> Option<InstanceInfo> {
        // Evict old entries if at capacity
        if self.instances.len() >= MAX_INSTANCES {
            self.evict_oldest(EVICTION_TARGET);
        }
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

    /// Atomically cancel an instance by setting its status to Cancelled.
    ///
    /// Returns `true` if the instance was found and cancelled, `false` otherwise.
    /// This avoids a TOCTOU race between separate `get` and `update_status` calls.
    pub fn cancel(&self, id: &str) -> bool {
        if let Some(mut entry) = self.instances.get_mut(id) {
            entry.status = InstanceStatus::Cancelled;
            entry.end_time = Some(Utc::now());
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

    /// Remove the oldest entries (by start_time) until the map is at `target` entries.
    fn evict_oldest(&self, target: usize) {
        if self.instances.len() <= target {
            return;
        }

        // Collect entries and sort by start_time (oldest first)
        let mut entries: Vec<(String, DateTime<Utc>)> = self
            .instances
            .iter()
            .map(|r| (r.key().clone(), r.value().start_time))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let to_remove = self.instances.len() - target;
        for (id, _) in entries.iter().take(to_remove) {
            self.instances.remove(id);
        }
    }

    /// Remove instances older than the configured TTL (1 hour).
    pub fn cleanup(&self) {
        let cutoff = Utc::now() - Duration::seconds(INSTANCE_TTL_SECS);
        let old_ids: Vec<String> = self
            .instances
            .iter()
            .filter(|r| r.value().start_time < cutoff)
            .map(|r| r.key().clone())
            .collect();

        for id in old_ids {
            self.instances.remove(&id);
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_info(id: &str, start_time: DateTime<Utc>) -> InstanceInfo {
        InstanceInfo {
            id: id.to_string(),
            project: "test".to_string(),
            sql: "SELECT 1".to_string(),
            status: InstanceStatus::Running,
            result: None,
            error: None,
            start_time,
            end_time: None,
        }
    }

    #[test]
    fn test_instance_manager_insert_and_get() {
        let mgr = InstanceManager::new();
        let info = make_info("test-1", Utc::now());
        mgr.insert(info.clone());
        let retrieved = mgr.get("test-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");
    }

    #[test]
    fn test_instance_manager_cancel() {
        let mgr = InstanceManager::new();
        let info = make_info("test-1", Utc::now());
        mgr.insert(info);

        assert!(mgr.cancel("test-1"), "cancel should succeed");
        let retrieved = mgr.get("test-1").unwrap();
        assert_eq!(retrieved.status, InstanceStatus::Cancelled);
        assert!(retrieved.end_time.is_some(), "end_time should be set on cancel");
    }

    #[test]
    fn test_instance_manager_cancel_nonexistent() {
        let mgr = InstanceManager::new();
        assert!(!mgr.cancel("nonexistent"), "cancel should return false for missing id");
    }

    #[test]
    fn test_instance_manager_eviction() {
        // Create a manager and insert more than MAX_INSTANCES entries
        let mgr = InstanceManager::new();
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        // Insert MAX_INSTANCES + 100 entries
        let total = MAX_INSTANCES + 100;
        for i in 0..total {
            let t = base_time + Duration::seconds(i as i64);
            let info = make_info(&format!("id-{:04}", i), t);
            mgr.insert(info);
        }

        // After insertions at capacity, eviction should have occurred
        let len = mgr.len();
        // eviction triggers at MAX_INSTANCES, removing oldest down to EVICTION_TARGET,
        // then remaining entries continue to be inserted without further eviction
        assert!(
            len < total,
            "Eviction should have reduced entries from {} to less than {}",
            total, total
        );

        // The oldest entries should have been evicted
        assert!(mgr.get("id-0000").is_none(), "oldest entry should be evicted");
    }

    #[test]
    fn test_instance_manager_cleanup_removes_old_instances() {
        let mgr = InstanceManager::new();
        let now = Utc::now();

        // Insert an old instance (older than 1 hour)
        let old_time = now - Duration::seconds(INSTANCE_TTL_SECS + 100);
        let info = make_info("old-instance", old_time);
        mgr.insert(info);

        // Insert a recent instance
        let info = make_info("recent-instance", now);
        mgr.insert(info);

        assert_eq!(mgr.len(), 2);

        mgr.cleanup();

        assert_eq!(mgr.len(), 1, "old instance should be cleaned up");
        assert!(mgr.get("old-instance").is_none(), "old instance should be removed");
        assert!(mgr.get("recent-instance").is_some(), "recent instance should remain");
    }
}