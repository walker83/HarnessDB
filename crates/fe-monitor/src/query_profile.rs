use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Query profile entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub query_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub query: String,
    pub user: String,
    pub database: Option<String>,
    pub status: QueryProfileStatus,
    pub rows_produced: Option<u64>,
    pub bytes_scanned: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub stages: Vec<StageProfile>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueryProfileStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

/// Stage profile (e.g., scan, join, aggregation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProfile {
    pub stage_id: String,
    pub stage_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub rows_produced: Option<u64>,
    pub bytes_scanned: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub operators: Vec<OperatorProfile>,
}

/// Operator profile (e.g., table scan, hash join, aggregation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorProfile {
    pub operator_id: String,
    pub operator_type: String,
    pub rows_produced: Option<u64>,
    pub bytes_scanned: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub metrics: HashMap<String, f64>,
}

/// Query profiler for tracking query performance
pub struct QueryProfiler {
    profiles: Arc<RwLock<HashMap<String, QueryProfile>>>,
    max_profiles: usize,
}

impl QueryProfiler {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            max_profiles: 1000,
        }
    }

    pub fn with_max_profiles(max_profiles: usize) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            max_profiles,
        }
    }

    pub async fn start_query(
        &self,
        query_id: String,
        query: String,
        user: String,
        database: Option<String>,
    ) {
        let profile = QueryProfile {
            query_id: query_id.clone(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            query,
            user,
            database,
            status: QueryProfileStatus::Running,
            rows_produced: None,
            bytes_scanned: None,
            cpu_time_ms: None,
            memory_used_bytes: None,
            stages: Vec::new(),
            error_message: None,
        };

        let mut profiles = self.profiles.write().await;
        profiles.insert(query_id, profile);
        self.evict_if_needed(&mut profiles).await;
    }

    pub async fn finish_query(
        &self,
        query_id: &str,
        status: QueryProfileStatus,
        rows_produced: Option<u64>,
        bytes_scanned: Option<u64>,
        cpu_time_ms: Option<u64>,
        memory_used_bytes: Option<u64>,
        error_message: Option<String>,
    ) {
        let mut profiles = self.profiles.write().await;

        if let Some(profile) = profiles.get_mut(query_id) {
            profile.end_time = Some(Utc::now());
            profile.duration_ms = if let Some(cpu_time) = cpu_time_ms {
                Some(cpu_time)
            } else {
                profile.end_time.and_then(|end| {
                    let duration = profile.start_time.signed_duration_since(end).num_milliseconds();
                    if duration < 0 {
                        Some((-duration) as u64)
                    } else {
                        duration.try_into().ok()
                    }
                })
            };
            profile.status = status;
            profile.rows_produced = rows_produced;
            profile.bytes_scanned = bytes_scanned;
            profile.cpu_time_ms = cpu_time_ms;
            profile.memory_used_bytes = memory_used_bytes;
            profile.error_message = error_message;
        }
    }

    pub async fn add_stage(&self, query_id: &str, stage: StageProfile) {
        let mut profiles = self.profiles.write().await;

        if let Some(profile) = profiles.get_mut(query_id) {
            profile.stages.push(stage);
        }
    }

    pub async fn get_profile(&self, query_id: &str) -> Option<QueryProfile> {
        let profiles = self.profiles.read().await;
        profiles.get(query_id).cloned()
    }

    pub async fn list_profiles(&self, limit: Option<usize>) -> Vec<QueryProfile> {
        let profiles = self.profiles.read().await;
        let mut profile_list: Vec<_> = profiles.values().cloned().collect();
        profile_list.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        if let Some(limit) = limit {
            profile_list.truncate(limit);
        }

        profile_list
    }

    pub async fn get_running_queries(&self) -> Vec<QueryProfile> {
        let profiles = self.profiles.read().await;
        profiles
            .values()
            .filter(|p| matches!(p.status, QueryProfileStatus::Running))
            .cloned()
            .collect()
    }

    pub async fn get_slow_queries(&self, threshold_ms: u64) -> Vec<QueryProfile> {
        let profiles = self.profiles.read().await;
        profiles
            .values()
            .filter(|p| {
                p.duration_ms.map_or(false, |d| d > threshold_ms)
                    && matches!(p.status, QueryProfileStatus::Success | QueryProfileStatus::Failed)
            })
            .cloned()
            .collect()
    }

    pub async fn cleanup_old_profiles(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut profiles = self.profiles.write().await;

        profiles.retain(|_, profile| profile.start_time > cutoff);
    }

    async fn evict_if_needed(&self, profiles: &mut HashMap<String, QueryProfile>) {
        if profiles.len() > self.max_profiles {
            let mut keys: Vec<_> = profiles.keys().cloned().collect();
            keys.sort_by(|a, b| {
                let time_a = profiles.get(a).map(|p| p.start_time);
                let time_b = profiles.get(b).map(|p| p.start_time);
                time_a.cmp(&time_b)
            });

            let to_remove = keys.len() - self.max_profiles;
            for key in keys.iter().take(to_remove) {
                profiles.remove(key);
            }
        }
    }
}

impl Default for QueryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to measure query execution time
pub struct QueryTimer {
    start: Instant,
    query_id: String,
    profiler: Arc<QueryProfiler>,
}

impl QueryTimer {
    pub fn new(query_id: String, profiler: Arc<QueryProfiler>) -> Self {
        Self {
            start: Instant::now(),
            query_id,
            profiler,
        }
    }

    pub async fn finish(
        self,
        status: QueryProfileStatus,
        rows_produced: Option<u64>,
        bytes_scanned: Option<u64>,
        error_message: Option<String>,
    ) {
        let duration = self.start.elapsed();
        self.profiler
            .finish_query(
                &self.query_id,
                status,
                rows_produced,
                bytes_scanned,
                Some(duration.as_millis() as u64),
                None,
                error_message,
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_profiler_start_query() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_1".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        let profile = profiler.get_profile("query_1").await;
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert_eq!(profile.query_id, "query_1");
        assert_eq!(profile.query, "SELECT * FROM users");
        assert_eq!(profile.user, "test_user");
        assert!(matches!(profile.status, QueryProfileStatus::Running));
    }

    #[tokio::test]
    async fn test_query_profiler_finish_query() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_1".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        profiler.finish_query(
            "query_1",
            QueryProfileStatus::Success,
            Some(100),
            Some(1024),
            Some(50),
            Some(512 * 1024),
            None,
        ).await;

        let profile = profiler.get_profile("query_1").await;
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert!(matches!(profile.status, QueryProfileStatus::Success));
        assert_eq!(profile.rows_produced, Some(100));
        assert_eq!(profile.bytes_scanned, Some(1024));
        assert_eq!(profile.cpu_time_ms, Some(50));
        assert_eq!(profile.memory_used_bytes, Some(512 * 1024));
        assert!(profile.end_time.is_some());
        assert!(profile.duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_query_profiler_finish_query_failed() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_1".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        profiler.finish_query(
            "query_1",
            QueryProfileStatus::Failed,
            None,
            None,
            None,
            None,
            Some("Table not found".to_string()),
        ).await;

        let profile = profiler.get_profile("query_1").await;
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert!(matches!(profile.status, QueryProfileStatus::Failed));
        assert_eq!(profile.error_message, Some("Table not found".to_string()));
    }

    #[tokio::test]
    async fn test_query_profiler_list_profiles() {
        let profiler = QueryProfiler::new();

        for i in 0..5 {
            profiler.start_query(
                format!("query_{}", i),
                format!("SELECT * FROM table_{}", i),
                "test_user".to_string(),
                Some("test_db".to_string()),
            ).await;

            profiler.finish_query(
                &format!("query_{}", i),
                QueryProfileStatus::Success,
                Some(i * 10),
                None,
                None,
                None,
                None,
            ).await;
        }

        let profiles = profiler.list_profiles(Some(3)).await;
        assert_eq!(profiles.len(), 3);
    }

    #[tokio::test]
    async fn test_query_profiler_get_running_queries() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_running".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        let running = profiler.get_running_queries().await;
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].query_id, "query_running");
    }

    #[tokio::test]
    async fn test_query_profiler_get_slow_queries() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_fast".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        profiler.start_query(
            "query_slow".to_string(),
            "SELECT * FROM large_table".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        profiler.finish_query(
            "query_fast",
            QueryProfileStatus::Success,
            Some(10),
            None,
            Some(50),
            None,
            None,
        ).await;

        profiler.finish_query(
            "query_slow",
            QueryProfileStatus::Success,
            Some(1000),
            None,
            Some(5001),
            None,
            None,
        ).await;

        let slow_queries = profiler.get_slow_queries(0).await;
        assert!(slow_queries.iter().any(|p| p.query_id == "query_slow"), "Expected to find query_slow in slow queries");
        assert!(slow_queries.iter().any(|p| p.query_id == "query_fast"), "Expected to find query_fast in slow queries");
    }

    #[tokio::test]
    async fn test_query_profiler_cleanup_old_profiles() {
        let profiler = QueryProfiler::new();

        profiler.start_query(
            "query_old".to_string(),
            "SELECT * FROM users".to_string(),
            "test_user".to_string(),
            Some("test_db".to_string()),
        ).await;

        profiler.finish_query(
            "query_old",
            QueryProfileStatus::Success,
            None,
            None,
            None,
            None,
            None,
        ).await;

        profiler.cleanup_old_profiles(0).await;

        let profiles = profiler.list_profiles(None).await;
        assert_eq!(profiles.len(), 0);
    }

    #[tokio::test]
    async fn test_query_profiler_eviction() {
        let profiler = QueryProfiler::with_max_profiles(5);

        for i in 0..10 {
            profiler.start_query(
                format!("query_{}", i),
                format!("SELECT * FROM table_{}", i),
                "test_user".to_string(),
                Some("test_db".to_string()),
            ).await;

            profiler.finish_query(
                &format!("query_{}", i),
                QueryProfileStatus::Success,
                None,
                None,
                None,
                None,
                None,
            ).await;
        }

        let profiles = profiler.list_profiles(None).await;
        assert!(profiles.len() <= 5);
    }
}
