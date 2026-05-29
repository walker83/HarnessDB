//! Connection tracking and server metrics

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Information about an active connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: u32,
    pub user: String,
    pub host: String,
    pub db: Option<String>,
    pub command: String,
    pub state: String,
    pub connected_at: Instant,
    pub current_sql: Option<String>,
    pub query_start: Option<Instant>,
}

/// Tracks active connections and server-wide metrics
pub struct ConnectionTracker {
    connections: RwLock<HashMap<u32, ConnectionInfo>>,
    total_connections: AtomicU64,
    active_queries: AtomicU64,
    total_queries: AtomicU64,
    slow_queries: AtomicU64,
    peak_connections: AtomicU32,
    rejected_queries: AtomicU64,
    rejected_connections: AtomicU64,
    startup_time: Instant,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            total_connections: AtomicU64::new(0),
            active_queries: AtomicU64::new(0),
            total_queries: AtomicU64::new(0),
            slow_queries: AtomicU64::new(0),
            peak_connections: AtomicU32::new(0),
            rejected_queries: AtomicU64::new(0),
            rejected_connections: AtomicU64::new(0),
            startup_time: Instant::now(),
        }
    }

    /// Register a new connection with a given ID (from MySQL protocol layer)
    pub fn register(&self, conn_id: u32, user: &str, host: &str, db: String) {
        let info = ConnectionInfo {
            id: conn_id,
            user: user.to_string(),
            host: host.to_string(),
            db: if db.is_empty() { None } else { Some(db) },
            command: "Sleep".to_string(),
            state: "".to_string(),
            connected_at: Instant::now(),
            current_sql: None,
            query_start: None,
        };

        let mut conns = self.connections.write();
        conns.insert(conn_id, info);
        let current_count = conns.len() as u32;
        drop(conns);

        self.total_connections.fetch_add(1, Ordering::SeqCst);

        // Update peak
        let mut peak = self.peak_connections.load(Ordering::Relaxed);
        while current_count > peak {
            match self.peak_connections.compare_exchange_weak(
                peak,
                current_count,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        // Update Prometheus active connections gauge
        crate::metrics::set_active_connections(current_count as f64);
    }

    /// Unregister a connection
    pub fn unregister(&self, id: u32) {
        let mut conns = self.connections.write();
        conns.remove(&id);
        let current_count = conns.len() as u32;
        drop(conns);

        // Update Prometheus active connections gauge
        crate::metrics::set_active_connections(current_count as f64);
    }

    /// Update the current SQL being executed by a connection
    pub fn update_sql(&self, id: u32, sql: Option<&str>) {
        let mut conns = self.connections.write();
        if let Some(info) = conns.get_mut(&id) {
            info.current_sql = sql.map(|s| s.to_string());
            info.command = if sql.is_some() { "Query" } else { "Sleep" }.to_string();
            info.query_start = if sql.is_some() {
                Some(Instant::now())
            } else {
                None
            };
            info.state = if sql.is_some() { "executing" } else { "" }.to_string();
        }
    }

    /// Set the current database for a connection
    pub fn set_database(&self, id: u32, db: &str) {
        let mut conns = self.connections.write();
        if let Some(info) = conns.get_mut(&id) {
            info.db = Some(db.to_string());
        }
    }

    /// Increment total query count
    pub fn record_query(&self) {
        self.total_queries.fetch_add(1, Ordering::SeqCst);
    }

    /// Increment active query count
    pub fn query_start(&self) {
        self.active_queries.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active query count
    pub fn query_end(&self) {
        self.active_queries.fetch_sub(1, Ordering::SeqCst);
    }

    /// Record a slow query
    pub fn record_slow_query(&self) {
        self.slow_queries.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a rejected query (concurrency limit exceeded)
    pub fn record_rejected_query(&self) {
        self.rejected_queries.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a rejected connection (max connections exceeded)
    pub fn record_rejected_connection(&self) {
        self.rejected_connections.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark a connection for kill (sets command to "Killed")
    pub fn kill(&self, id: u32) -> bool {
        let mut conns = self.connections.write();
        if let Some(info) = conns.get_mut(&id) {
            info.command = "Killed".to_string();
            true
        } else {
            false
        }
    }

    /// List all active connections
    pub fn list(&self) -> Vec<ConnectionInfo> {
        let conns = self.connections.read();
        conns.values().cloned().collect()
    }

    // ---- Metrics accessors ----

    pub fn uptime_seconds(&self) -> u64 {
        self.startup_time.elapsed().as_secs()
    }

    pub fn total_queries(&self) -> u64 {
        self.total_queries.load(Ordering::SeqCst)
    }

    pub fn active_queries(&self) -> u64 {
        self.active_queries.load(Ordering::SeqCst)
    }

    pub fn total_connections(&self) -> u64 {
        self.total_connections.load(Ordering::SeqCst)
    }

    pub fn active_connections(&self) -> u32 {
        self.connections.read().len() as u32
    }

    pub fn peak_connections(&self) -> u32 {
        self.peak_connections.load(Ordering::SeqCst)
    }

    pub fn slow_queries(&self) -> u64 {
        self.slow_queries.load(Ordering::SeqCst)
    }

    pub fn rejected_queries(&self) -> u64 {
        self.rejected_queries.load(Ordering::SeqCst)
    }

    pub fn rejected_connections(&self) -> u64 {
        self.rejected_connections.load(Ordering::SeqCst)
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}
