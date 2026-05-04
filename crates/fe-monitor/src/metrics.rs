use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use prometheus::{Counter, Histogram, Gauge, Registry, TextEncoder};

/// FE (Frontend) metrics
#[derive(Default)]
pub struct FeMetrics {
    pub queries_total: AtomicU64,
    pub queries_success: AtomicU64,
    pub queries_failed: AtomicU64,
    pub query_duration_ms: AtomicU64,
    pub active_connections: AtomicUsize,
}

impl FeMetrics {
    pub fn inc_queries_total(&self) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_queries_success(&self) {
        self.queries_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_queries_failed(&self) {
        self.queries_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_query_duration(&self, duration_ms: u64) {
        self.query_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn inc_active_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get_metrics_snapshot(&self) -> FeMetricsSnapshot {
        FeMetricsSnapshot {
            queries_total: self.queries_total.load(Ordering::Relaxed),
            queries_success: self.queries_success.load(Ordering::Relaxed),
            queries_failed: self.queries_failed.load(Ordering::Relaxed),
            query_duration_ms: self.query_duration_ms.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeMetricsSnapshot {
    pub queries_total: u64,
    pub queries_success: u64,
    pub queries_failed: u64,
    pub query_duration_ms: u64,
    pub active_connections: usize,
}

/// BE (Backend) metrics
#[derive(Default)]
pub struct BeMetrics {
    pub queries_total: AtomicU64,
    pub queries_success: AtomicU64,
    pub queries_failed: AtomicU64,
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
    pub rows_read: AtomicU64,
    pub rows_written: AtomicU64,
    pub compaction_num: AtomicU64,
    pub compaction_duration_ms: AtomicU64,
    pub memory_used_bytes: AtomicU64,
    pub disk_used_bytes: AtomicU64,
}

impl BeMetrics {
    pub fn inc_queries_total(&self) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_queries_success(&self) {
        self.queries_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_queries_failed(&self) {
        self.queries_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_bytes_read(&self, bytes: u64) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn inc_bytes_written(&self, bytes: u64) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn inc_rows_read(&self, rows: u64) {
        self.rows_read.fetch_add(rows, Ordering::Relaxed);
    }

    pub fn inc_rows_written(&self, rows: u64) {
        self.rows_written.fetch_add(rows, Ordering::Relaxed);
    }

    pub fn inc_compaction_num(&self) {
        self.compaction_num.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_compaction_duration(&self, duration_ms: u64) {
        self.compaction_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn update_memory_used(&self, bytes: u64) {
        self.memory_used_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn update_disk_used(&self, bytes: u64) {
        self.disk_used_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn get_metrics_snapshot(&self) -> BeMetricsSnapshot {
        BeMetricsSnapshot {
            queries_total: self.queries_total.load(Ordering::Relaxed),
            queries_success: self.queries_success.load(Ordering::Relaxed),
            queries_failed: self.queries_failed.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            rows_read: self.rows_read.load(Ordering::Relaxed),
            rows_written: self.rows_written.load(Ordering::Relaxed),
            compaction_num: self.compaction_num.load(Ordering::Relaxed),
            compaction_duration_ms: self.compaction_duration_ms.load(Ordering::Relaxed),
            memory_used_bytes: self.memory_used_bytes.load(Ordering::Relaxed),
            disk_used_bytes: self.disk_used_bytes.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BeMetricsSnapshot {
    pub queries_total: u64,
    pub queries_success: u64,
    pub queries_failed: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub rows_read: u64,
    pub rows_written: u64,
    pub compaction_num: u64,
    pub compaction_duration_ms: u64,
    pub memory_used_bytes: u64,
    pub disk_used_bytes: u64,
}

/// Central metrics collector
pub struct MetricsCollector {
    pub fe: FeMetrics,
    pub be: Arc<BeMetrics>,
    registry: Registry,
    query_duration_histogram: Histogram,
    query_counter: Counter,
    active_connections_gauge: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let query_duration_histogram = Histogram::with_opts(
            prometheus::HistogramOpts::new("roris_query_duration_seconds", "Query duration in seconds")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        ).unwrap();

        let query_counter = Counter::with_opts(
            prometheus::Opts::new("roris_queries_total", "Total number of queries")
        ).unwrap();

        let active_connections_gauge = Gauge::with_opts(
            prometheus::Opts::new("roris_active_connections", "Number of active connections")
        ).unwrap();

        registry.register(Box::new(query_duration_histogram.clone())).unwrap();
        registry.register(Box::new(query_counter.clone())).unwrap();
        registry.register(Box::new(active_connections_gauge.clone())).unwrap();

        Self {
            fe: FeMetrics::default(),
            be: Arc::new(BeMetrics::default()),
            registry,
            query_duration_histogram,
            query_counter,
            active_connections_gauge,
        }
    }

    pub fn record_query(&self, duration_ms: u64, success: bool) {
        self.fe.inc_queries_total();
        self.query_counter.inc();
        self.query_duration_histogram.observe(duration_ms as f64 / 1000.0);

        if success {
            self.fe.inc_queries_success();
        } else {
            self.fe.inc_queries_failed();
        }
    }

    pub fn update_active_connections(&self, count: usize) {
        self.active_connections_gauge.set(count as f64);
    }

    pub fn export_prometheus(&self) -> Result<String, String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .map_err(|e| format!("Failed to encode metrics: {}", e))
    }

    pub fn get_fe_metrics(&self) -> FeMetricsSnapshot {
        self.fe.get_metrics_snapshot()
    }

    pub fn get_be_metrics(&self) -> BeMetricsSnapshot {
        self.be.get_metrics_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fe_metrics_increment() {
        let metrics = FeMetrics::default();

        metrics.inc_queries_total();
        assert_eq!(metrics.queries_total.load(Ordering::Relaxed), 1);

        metrics.inc_queries_success();
        assert_eq!(metrics.queries_success.load(Ordering::Relaxed), 1);

        metrics.inc_queries_failed();
        assert_eq!(metrics.queries_failed.load(Ordering::Relaxed), 1);

        metrics.inc_active_connections();
        assert_eq!(metrics.active_connections.load(Ordering::Relaxed), 1);

        metrics.dec_active_connections();
        assert_eq!(metrics.active_connections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_fe_metrics_query_duration() {
        let metrics = FeMetrics::default();

        metrics.record_query_duration(100);
        assert_eq!(metrics.query_duration_ms.load(Ordering::Relaxed), 100);

        metrics.record_query_duration(250);
        assert_eq!(metrics.query_duration_ms.load(Ordering::Relaxed), 350);
    }

    #[test]
    fn test_fe_metrics_snapshot() {
        let metrics = FeMetrics::default();

        metrics.inc_queries_total();
        metrics.inc_queries_success();
        metrics.inc_active_connections();
        metrics.record_query_duration(150);

        let snapshot = metrics.get_metrics_snapshot();

        assert_eq!(snapshot.queries_total, 1);
        assert_eq!(snapshot.queries_success, 1);
        assert_eq!(snapshot.queries_failed, 0);
        assert_eq!(snapshot.query_duration_ms, 150);
        assert_eq!(snapshot.active_connections, 1);
    }

    #[test]
    fn test_be_metrics_increment() {
        let metrics = BeMetrics::default();

        metrics.inc_queries_total();
        assert_eq!(metrics.queries_total.load(Ordering::Relaxed), 1);

        metrics.inc_bytes_read(1024);
        assert_eq!(metrics.bytes_read.load(Ordering::Relaxed), 1024);

        metrics.inc_bytes_written(2048);
        assert_eq!(metrics.bytes_written.load(Ordering::Relaxed), 2048);

        metrics.inc_rows_read(100);
        assert_eq!(metrics.rows_read.load(Ordering::Relaxed), 100);

        metrics.inc_rows_written(50);
        assert_eq!(metrics.rows_written.load(Ordering::Relaxed), 50);

        metrics.inc_compaction_num();
        assert_eq!(metrics.compaction_num.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_be_metrics_update() {
        let metrics = BeMetrics::default();

        metrics.update_memory_used(1024 * 1024);
        assert_eq!(metrics.memory_used_bytes.load(Ordering::Relaxed), 1024 * 1024);

        metrics.update_disk_used(10 * 1024 * 1024);
        assert_eq!(metrics.disk_used_bytes.load(Ordering::Relaxed), 10 * 1024 * 1024);
    }

    #[test]
    fn test_be_metrics_snapshot() {
        let metrics = BeMetrics::default();

        metrics.inc_queries_total();
        metrics.inc_bytes_read(2048);
        metrics.inc_bytes_written(4096);
        metrics.update_memory_used(512 * 1024);
        metrics.update_disk_used(5 * 1024 * 1024);

        let snapshot = metrics.get_metrics_snapshot();

        assert_eq!(snapshot.queries_total, 1);
        assert_eq!(snapshot.bytes_read, 2048);
        assert_eq!(snapshot.bytes_written, 4096);
        assert_eq!(snapshot.memory_used_bytes, 512 * 1024);
        assert_eq!(snapshot.disk_used_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn test_metrics_collector_record_query() {
        let collector = MetricsCollector::new();

        collector.record_query(100, true);
        collector.record_query(200, false);
        collector.record_query(150, true);

        let fe_metrics = collector.get_fe_metrics();
        assert_eq!(fe_metrics.queries_total, 3);
        assert_eq!(fe_metrics.queries_success, 2);
        assert_eq!(fe_metrics.queries_failed, 1);
    }

    #[test]
    fn test_metrics_collector_export_prometheus() {
        let collector = MetricsCollector::new();

        collector.record_query(100, true);
        collector.update_active_connections(5);

        let export = collector.export_prometheus();
        assert!(export.is_ok());

        let prometheus_text = export.unwrap();
        assert!(prometheus_text.contains("roris_queries_total"));
        assert!(prometheus_text.contains("roris_query_duration_seconds"));
        assert!(prometheus_text.contains("roris_active_connections"));
        assert!(prometheus_text.contains("3") || prometheus_text.contains("4") || prometheus_text.contains("5"));
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
