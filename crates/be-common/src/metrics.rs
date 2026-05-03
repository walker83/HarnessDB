use std::sync::atomic::{AtomicU64, Ordering};

pub struct BeMetrics {
    pub queries_total: AtomicU64,
    pub queries_success: AtomicU64,
    pub queries_failed: AtomicU64,
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
    pub rows_read: AtomicU64,
    pub rows_written: AtomicU64,
    pub compaction_num: AtomicU64,
}

impl BeMetrics {
    pub fn new() -> Self {
        Self {
            queries_total: AtomicU64::new(0),
            queries_success: AtomicU64::new(0),
            queries_failed: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            rows_read: AtomicU64::new(0),
            rows_written: AtomicU64::new(0),
            compaction_num: AtomicU64::new(0),
        }
    }

    pub fn inc_queries_total(&self) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_bytes_read(&self, bytes: u64) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn inc_bytes_written(&self, bytes: u64) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }
}

impl Default for BeMetrics {
    fn default() -> Self {
        Self::new()
    }
}
