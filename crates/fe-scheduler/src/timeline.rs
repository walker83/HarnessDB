use common::DrorisError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use types::Block;

/// Helper to get the current time as milliseconds since Unix epoch.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Globally unique query identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryId {
    pub value: String,
}

impl QueryId {
    /// Generate a new unique query ID using a node prefix and a monotonic counter.
    pub fn generate(node_id: &str, seq: u64) -> Self {
        Self {
            value: format!("{}-{}", node_id, seq),
        }
    }
}

impl std::fmt::Display for QueryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// State machine states for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryState {
    /// Query has been accepted but not yet scheduled.
    Pending,
    /// Query is actively executing on one or more BE nodes.
    Running,
    /// Query completed successfully.
    Finished,
    /// Query failed with an error.
    Failed,
    /// Query was cancelled by the user or system.
    Cancelled,
}

impl QueryState {
    /// Returns true if the query is in a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Finished | Self::Failed | Self::Cancelled)
    }
}

/// Metrics captured over the lifetime of a query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryMetrics {
    /// Timestamp (unix millis) when execution began.
    pub execution_start: Option<u64>,
    /// Timestamp (unix millis) when execution ended.
    pub execution_end: Option<u64>,
    /// Total rows scanned across all scan operators.
    pub rows_scanned: u64,
    /// Total bytes read from storage.
    pub bytes_processed: u64,
    /// Total rows produced as output.
    pub rows_produced: u64,
    /// Peak memory usage in bytes.
    pub peak_memory_bytes: u64,
    /// Number of CPU-core-seconds consumed.
    pub cpu_time_ns: u64,
}

impl QueryMetrics {
    /// Elapsed wall-clock time between execution start and end (or now if still running).
    pub fn elapsed(&self) -> std::time::Duration {
        match (self.execution_start, self.execution_end) {
            (Some(start), Some(end)) => std::time::Duration::from_millis(end.saturating_sub(start)),
            (Some(start), None) => std::time::Duration::from_millis(now_millis().saturating_sub(start)),
            _ => std::time::Duration::ZERO,
        }
    }
}

/// A handle that tracks the full lifecycle of a single query.
#[derive(Debug)]
pub struct QueryHandle {
    pub query_id: QueryId,
    pub sql: String,
    pub state: QueryState,
    pub metrics: QueryMetrics,
    /// Partial result blocks collected so far.
    pub result_blocks: Vec<Block>,
    /// Error message if the query failed.
    pub error_message: Option<String>,
}

impl QueryHandle {
    pub fn new(query_id: QueryId, sql: String) -> Self {
        Self {
            query_id,
            sql,
            state: QueryState::Pending,
            metrics: QueryMetrics::default(),
            result_blocks: Vec::new(),
            error_message: None,
        }
    }

    /// Transition to Running state. Returns an error if the transition is invalid.
    pub fn start(&mut self) -> Result<(), DrorisError> {
        match self.state {
            QueryState::Pending => {
                self.state = QueryState::Running;
                self.metrics.execution_start = Some(now_millis());
                Ok(())
            }
            other => Err(DrorisError::Internal(format!(
                "cannot start query {}: current state is {:?}",
                self.query_id, other
            ))),
        }
    }

    /// Transition to Finished state and record final metrics.
    pub fn finish(&mut self, blocks: Vec<Block>) -> Result<(), DrorisError> {
        match self.state {
            QueryState::Running => {
                self.state = QueryState::Finished;
                self.metrics.execution_end = Some(now_millis());
                self.result_blocks = blocks;
                self.metrics.rows_produced = self
                    .result_blocks
                    .iter()
                    .map(|b| b.num_rows() as u64)
                    .sum();
                Ok(())
            }
            other => Err(DrorisError::Internal(format!(
                "cannot finish query {}: current state is {:?}",
                self.query_id, other
            ))),
        }
    }

    /// Transition to Failed state.
    pub fn fail(&mut self, message: String) -> Result<(), DrorisError> {
        match self.state {
            QueryState::Running | QueryState::Pending => {
                self.state = QueryState::Failed;
                self.metrics.execution_end = Some(now_millis());
                self.error_message = Some(message);
                Ok(())
            }
            other => Err(DrorisError::Internal(format!(
                "cannot fail query {}: current state is {:?}",
                self.query_id, other
            ))),
        }
    }

    /// Transition to Cancelled state.
    pub fn cancel(&mut self) -> Result<(), DrorisError> {
        match self.state {
            QueryState::Pending | QueryState::Running => {
                self.state = QueryState::Cancelled;
                self.metrics.execution_end = Some(now_millis());
                Ok(())
            }
            other => Err(DrorisError::Internal(format!(
                "cannot cancel query {}: current state is {:?}",
                self.query_id, other
            ))),
        }
    }

    /// Accumulate metrics from a partial execution report.
    pub fn accumulate_metrics(&mut self, rows_scanned: u64, bytes_processed: u64, cpu_time_ns: u64) {
        self.metrics.rows_scanned += rows_scanned;
        self.metrics.bytes_processed += bytes_processed;
        self.metrics.cpu_time_ns += cpu_time_ns;
    }
}

/// Global query ID generator and registry of all active/past queries.
pub struct QueryTimeline {
    node_id: String,
    next_seq: AtomicU64,
    /// Tracks all queries that are currently Pending or Running.
    /// Finished/Failed/Cancelled queries are retained until explicitly removed.
    handles: std::collections::HashMap<QueryId, QueryHandle>,
}

impl QueryTimeline {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            next_seq: AtomicU64::new(1),
            handles: std::collections::HashMap::new(),
        }
    }

    /// Allocate a new unique QueryId.
    pub fn allocate_id(&self) -> QueryId {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        QueryId::generate(&self.node_id, seq)
    }

    /// Register a new query in Pending state and return a mutable reference.
    pub fn submit(&mut self, sql: String) -> QueryId {
        let id = self.allocate_id();
        let handle = QueryHandle::new(id.clone(), sql);
        self.handles.insert(id.clone(), handle);
        id
    }

    /// Look up a query handle by ID.
    pub fn get(&self, id: &QueryId) -> Option<&QueryHandle> {
        self.handles.get(id)
    }

    /// Look up a query handle mutably by ID.
    pub fn get_mut(&mut self, id: &QueryId) -> Option<&mut QueryHandle> {
        self.handles.get_mut(id)
    }

    /// Remove a completed query from the timeline.
    pub fn remove(&mut self, id: &QueryId) -> Option<QueryHandle> {
        self.handles.remove(id)
    }

    /// List all queries matching the given state filter.
    pub fn list_by_state(&self, state: QueryState) -> Vec<&QueryHandle> {
        self.handles
            .values()
            .filter(|h| h.state == state)
            .collect()
    }

    /// List all currently running queries.
    pub fn running_queries(&self) -> Vec<&QueryHandle> {
        self.list_by_state(QueryState::Running)
    }

    /// Cancel a query by ID.
    pub fn cancel(&mut self, id: &QueryId) -> Result<(), DrorisError> {
        if let Some(handle) = self.handles.get_mut(id) {
            handle.cancel()
        } else {
            Err(DrorisError::Query(format!(
                "query {} not found",
                id
            )))
        }
    }

    /// Evict all terminal-state queries older than the given duration (measured from
    /// query creation time via metrics.execution_end). This is a no-op placeholder
    /// since we use Instant which is not comparable across runs; real implementations
    /// would use timestamps.
    pub fn evict_finished(&mut self) {
        self.handles.retain(|_, h| !h.state.is_terminal());
    }
}

impl std::fmt::Debug for QueryTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryTimeline")
            .field("node_id", &self.node_id)
            .field("active_queries", &self.handles.len())
            .finish()
    }
}
