//! Prometheus metrics for HarnessDB observability
//!
//! Provides key metrics: query latency distribution, query counts by type,
//! active connections, slow queries, and memory usage.

use lazy_static::lazy_static;
use prometheus::{
    CounterVec, Gauge, HistogramVec, register_counter_vec, register_gauge, register_histogram_vec,
};

lazy_static! {
    /// Query latency distribution in milliseconds, bucketed by query type
    pub static ref QUERY_DURATION_MS: HistogramVec = register_histogram_vec!(
        "harness_query_duration_ms",
        "Query latency distribution in milliseconds",
        &["query_type"],
        vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            30000.0,
        ],
    )
    .unwrap();

    /// Total number of queries processed, by type and status
    pub static ref QUERIES_TOTAL: CounterVec = register_counter_vec!(
        "harness_queries_total",
        "Total number of queries processed",
        &["query_type", "status"],
    )
    .unwrap();

    /// Currently active (open) connections
    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "harness_active_connections",
        "Number of currently active connections",
    )
    .unwrap();

    /// Total number of slow queries (above configurable threshold), by type
    pub static ref SLOW_QUERIES_TOTAL: CounterVec = register_counter_vec!(
        "harness_slow_queries_total",
        "Total number of slow queries (exceeding slow_query_threshold)",
        &["query_type"],
    )
    .unwrap();

    /// Current process memory usage in bytes
    pub static ref PROCESS_MEMORY_BYTES: Gauge = register_gauge!(
        "harness_process_memory_bytes",
        "Current process memory usage in bytes",
    )
    .unwrap();

    /// Server info metric (constant value with version label)
    pub static ref RORIS_SERVER_INFO: Gauge = register_gauge!(
        "harness_server_info",
        "HarnessDB server info (version encoded in label value)",
    )
    .unwrap();
}

/// Classify a SQL query into a metric label: SELECT, INSERT, UPDATE, DELETE, or DDL
pub fn classify_query(sql: &str) -> &'static str {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return "DDL";
    }
    let upper = trimmed.to_uppercase();
    if upper.starts_with("SELECT") || upper.starts_with("WITH") {
        "SELECT"
    } else if upper.starts_with("INSERT") {
        "INSERT"
    } else if upper.starts_with("UPDATE") {
        "UPDATE"
    } else if upper.starts_with("DELETE") {
        "DELETE"
    } else {
        "DDL"
    }
}

/// Record a completed query's metrics
pub fn record_query(sql: &str, duration_ms: f64, is_slow: bool, has_error: bool) {
    let query_type = classify_query(sql);
    let status = if has_error { "error" } else { "success" };

    QUERY_DURATION_MS
        .with_label_values(&[query_type])
        .observe(duration_ms);

    QUERIES_TOTAL.with_label_values(&[query_type, status]).inc();

    if is_slow {
        SLOW_QUERIES_TOTAL.with_label_values(&[query_type]).inc();
    }
}

/// Update active connections gauge
pub fn set_active_connections(count: f64) {
    ACTIVE_CONNECTIONS.set(count);
}

/// Update process memory gauge by reading /proc/self/status (Linux only, best-effort)
pub fn update_process_memory() {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            PROCESS_MEMORY_BYTES.set(kb * 1024.0);
                            return;
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Non-Linux platforms: skip process memory collection
        // (Prometheus process feature provides basic process metrics)
    }
}
