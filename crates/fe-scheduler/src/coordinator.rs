use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::DrorisError;
use dashmap::DashMap;
use fe_catalog::CatalogManager;
use fe_sql_parser::Statement;
use fe_sql_planner::optimizer::Optimizer;
use fe_sql_planner::planner::Planner;
use fe_sql_planner::PlanNode;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use types::Block;

use crate::cluster::ClusterManager;
use crate::fragment::FragmentTree;
use crate::scheduler::{QueryLimits, Scheduler};
use crate::timeline::{QueryId, QueryState, QueryTimeline};

// ---------------------------------------------------------------------------
// Coordinator config
// ---------------------------------------------------------------------------

/// Configuration for the query coordinator.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Unique identifier for this FE node (used in query ID generation).
    pub node_id: String,
    /// Default timeout for queries.
    pub default_query_timeout: Duration,
    /// Interval at which to check for timed-out queries.
    pub timeout_check_interval: Duration,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            node_id: "fe-0".into(),
            default_query_timeout: Duration::from_secs(300),
            timeout_check_interval: Duration::from_secs(5),
        }
    }
}

// ---------------------------------------------------------------------------
// Query result
// ---------------------------------------------------------------------------

/// The result of a completed query.
#[derive(Debug)]
pub struct QueryResult {
    pub query_id: QueryId,
    pub blocks: Vec<Block>,
    pub rows_produced: u64,
    pub elapsed: Duration,
}

// ---------------------------------------------------------------------------
// Running query tracker
// ---------------------------------------------------------------------------

/// Tracks the state of a query that has been scheduled and is executing.
struct RunningQuery {
    query_id: QueryId,
    sql: String,
    scheduled_at: std::time::Instant,
    timeout: Duration,
    fragment_tree: FragmentTree,
    /// Partial result blocks collected from BE nodes so far.
    result_blocks: Vec<Block>,
    /// Accumulated rows scanned across all fragments.
    rows_scanned: u64,
    /// Accumulated bytes processed.
    bytes_processed: u64,
}

// ---------------------------------------------------------------------------
// Coordinator
// ---------------------------------------------------------------------------

/// The query coordinator orchestrates the full lifecycle of query execution:
///
/// 1. Parse + Plan: Convert SQL into a logical plan.
/// 2. Optimize: Apply optimizer rules.
/// 3. Fragmentize: Split the plan into distributed fragments.
/// 4. Schedule: Assign fragment instances to BE nodes.
/// 5. Execute: Dispatch work to BE nodes (in a real system, via RPC).
/// 6. Collect: Gather results from BE nodes.
///
/// It also manages query cancellation, timeouts, and failure recovery.
pub struct Coordinator {
    config: CoordinatorConfig,
    planner: Planner,
    optimizer: Optimizer,
    scheduler: Scheduler,
    timeline: Arc<RwLock<QueryTimeline>>,
    /// Queries that are currently in Running state.
    running_queries: DashMap<QueryId, RunningQuery>,
}

impl Coordinator {
    /// Create a new coordinator backed by the given cluster manager and catalog.
    pub fn new(cluster: Arc<ClusterManager>, catalog: Arc<CatalogManager>, config: CoordinatorConfig) -> Self {
        let scheduler = Scheduler::new(cluster);
        let timeline = QueryTimeline::new(config.node_id.clone());

        Self {
            config,
            planner: Planner::new(catalog),
            optimizer: Optimizer::new(),
            scheduler,
            timeline: Arc::new(RwLock::new(timeline)),
            running_queries: DashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Full query lifecycle
    // -----------------------------------------------------------------------

    /// Execute a SQL statement through the full pipeline.
    ///
    /// This is the primary entry point. It performs:
    /// plan -> optimize -> fragment -> schedule -> execute -> collect.
    pub async fn execute(&self, stmt: Statement, sql: String) -> Result<QueryResult, DrorisError> {
        // Step 1: Allocate a query ID and register it in the timeline.
        let query_id = {
            let mut timeline = self.timeline.write().await;
            let id = timeline.submit(sql.clone());
            let handle = timeline.get_mut(&id).unwrap();
            handle.start()?;
            id
        };

        info!("Coordinator executing query {}: {}", query_id, sql);

        // Step 2: Check cluster limits.
        let running_count = self.running_queries.len();
        self.scheduler.check_cluster_limits(running_count).await?;

        // Step 3: Plan.
        let plan = self.plan(&stmt).map_err(|e| {
            self.record_failure(&query_id, e.to_string());
            e
        })?;

        // Step 4: Optimize.
        let optimized = self.optimize(plan);

        // Step 5: Schedule (fragmentize + assign instances).
        let limits = QueryLimits {
            timeout_ms: self.config.default_query_timeout.as_millis() as u64,
            ..QueryLimits::default()
        };

        let fragment_tree = self
            .scheduler
            .schedule(optimized, &query_id.to_string(), &limits)
            .await
            .map_err(|e| {
                self.record_failure(&query_id, e.to_string());
                e
            })?;

        // Step 6: Track the running query.
        self.running_queries.insert(
            query_id.clone(),
            RunningQuery {
                query_id: query_id.clone(),
                sql: sql.clone(),
                scheduled_at: std::time::Instant::now(),
                timeout: self.config.default_query_timeout,
                fragment_tree,
                result_blocks: Vec::new(),
                rows_scanned: 0,
                bytes_processed: 0,
                },
            );
        }

        // Step 7: Execute (dispatch to BE nodes).
        // In a real implementation this would send RPCs and await responses.
        // For now we simulate execution and produce an empty result.
        let result = self.dispatch_and_collect(&query_id).await?;

        Ok(result)
    }

    /// Parse and plan a SQL statement into a logical plan.
    pub fn plan(&self, stmt: &Statement) -> Result<PlanNode, DrorisError> {
        self.planner.plan(stmt.clone())
    }

    /// Apply optimizer rules to a logical plan.
    pub fn optimize(&self, plan: PlanNode) -> PlanNode {
        self.optimizer.optimize(plan)
    }

    // -----------------------------------------------------------------------
    // Execution dispatch and result collection
    // -----------------------------------------------------------------------

    /// Dispatch fragment instances to BE nodes and collect results.
    ///
    /// In a production system this would:
    /// - Serialize each FragmentInstance and send it to the assigned BE via RPC.
    /// - Stream result blocks back from each BE.
    /// - Handle partial failures and retries.
    ///
    /// Here we simulate the process and return an empty result set.
    async fn dispatch_and_collect(&self, query_id: &QueryId) -> Result<QueryResult, DrorisError> {
        let running_query_ref = self.running_queries
            .get_mut(query_id)
            .ok_or_else(|| DrorisError::Internal(format!("query {} not in running tracker", query_id)))?;

        // Simulate: in a real system we would dispatch RPCs here.
        // For each fragment instance in topological order (leaves first),
        // send an execution request to the assigned BE node.
        let topological = running_query_ref.fragment_tree.topological_order();
        debug!(
            "Query {}: dispatching {} fragments",
            query_id,
            topological.len()
        );

        for fragment in &topological {
            for instance in &fragment.instances {
                debug!(
                    "Query {}: dispatching instance {} to {}",
                    query_id, instance.id, instance.node_address
                );
                // In a real implementation:
                // self.rpc_client.execute_fragment(instance).await?;
            }
        }

        // Simulate collecting results (empty for now).
        let blocks: Vec<Block> = vec![];
        let elapsed = running_query_ref.scheduled_at.elapsed();
        let rows_produced: u64 = blocks.iter().map(|b| b.num_rows() as u64).sum();
        let rows_scanned = running_query_ref.rows_scanned;
        let bytes_processed = running_query_ref.bytes_processed;

        // Remove from running tracker.
        self.running_queries.remove(query_id);

        // Record metrics in the timeline.
        {
            let mut timeline = self.timeline.write().await;
            if let Some(handle) = timeline.get_mut(query_id) {
                handle.accumulate_metrics(
                    rows_scanned,
                    bytes_processed,
                    0,
                );
                let _ = handle.finish(blocks.clone());
            }
        }

        Ok(QueryResult {
            query_id: query_id.clone(),
            blocks,
            rows_produced,
            elapsed,
        })
    }

    // -----------------------------------------------------------------------
    // Cancellation
    // -----------------------------------------------------------------------

    /// Cancel a running query.
    pub async fn cancel(&self, query_id: &QueryId) -> Result<(), DrorisError> {
        info!("Cancelling query {}", query_id);

        // Remove from running queries tracker.
        if let Some((_, rq)) = self.running_queries.remove(query_id) {
            // In a real system, send cancellation RPCs to all BE nodes with
            // instances for this query.
            for fragment in rq.fragment_tree.topological_order() {
                for instance in &fragment.instances {
                    debug!(
                        "Sending cancel for instance {} to {}",
                        instance.id, instance.node_address
                    );
                    // self.rpc_client.cancel_fragment(&instance).await?;
                }
            }
        }

        // Update timeline.
        let mut timeline = self.timeline.write().await;
        timeline.cancel(query_id)
    }

    // -----------------------------------------------------------------------
    // Timeout management
    // -----------------------------------------------------------------------

    /// Check all running queries for timeout and cancel those that have exceeded
    /// their deadline.
    pub async fn check_timeouts(&self) -> Vec<QueryId> {
        let mut timed_out = Vec::new();
        let now = std::time::Instant::now();

        for entry in self.running_queries.iter() {
            let query_id = entry.key();
            let rq = entry.value();
            if now.duration_since(rq.scheduled_at) > rq.timeout {
                warn!("Query {} timed out after {:?}", query_id, rq.timeout);
                timed_out.push(query_id.clone());
            }
        }

        for query_id in &timed_out {
            let _ = self.cancel(query_id).await;
            // Record failure reason.
            let mut timeline = self.timeline.write().await;
            if let Some(handle) = timeline.get_mut(query_id) {
                let _ = handle.fail("query timed out".into());
            }
        }

        timed_out
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Look up a query handle from the timeline.
    pub async fn get_query(&self, query_id: &QueryId) -> Option<crate::timeline::QueryHandle> {
        let timeline = self.timeline.read().await;
        // We cannot return &QueryHandle because the RwLock guard is dropped.
        // Clone the handle data into a lightweight snapshot.
        timeline.get(query_id).map(|h| {
            // Return a reconstructed handle (the QueryHandle does not impl Clone
            // because it contains Instant; we create a new one for the snapshot).
            // For simplicity, we return a copy by reconstructing.
            let mut snapshot = crate::timeline::QueryHandle::new(
                h.query_id.clone(),
                h.sql.clone(),
            );
            snapshot.state = h.state;
            snapshot.error_message = h.error_message.clone();
            snapshot.result_blocks = h.result_blocks.clone();
            snapshot.metrics.rows_scanned = h.metrics.rows_scanned;
            snapshot.metrics.bytes_processed = h.metrics.bytes_processed;
            snapshot.metrics.rows_produced = h.metrics.rows_produced;
            snapshot.metrics.peak_memory_bytes = h.metrics.peak_memory_bytes;
            snapshot.metrics.cpu_time_ns = h.metrics.cpu_time_ns;
            snapshot
        })
    }

    /// List all queries in a given state.
    pub async fn list_queries(&self, state: QueryState) -> Vec<QueryId> {
        let timeline = self.timeline.read().await;
        timeline
            .list_by_state(state)
            .into_iter()
            .map(|h| h.query_id.clone())
            .collect()
    }

    /// Number of currently running queries.
    pub async fn running_query_count(&self) -> usize {
        self.running_queries.len()
    }

    // -----------------------------------------------------------------------
    // Failure recovery
    // -----------------------------------------------------------------------

    /// Handle a BE node failure: re-schedule all affected fragment instances.
    pub async fn handle_node_failure(
        &self,
        failed_node_id: &crate::cluster::NodeId,
    ) -> Result<(), DrorisError> {
        warn!("Handling BE node failure: {}", failed_node_id);

        for mut entry in self.running_queries.iter_mut() {
            let query_id_str = entry.query_id.to_string();
            let limits = QueryLimits {
                timeout_ms: entry.timeout.as_millis() as u64,
                ..QueryLimits::default()
            };

            let reassigned = self
                .scheduler
                .reschedule_on_failure(
                    &mut entry.fragment_tree,
                    failed_node_id,
                    &query_id_str,
                    &limits,
                )
                .await?;

            if !reassigned.is_empty() {
                info!(
                    "Re-scheduled {} fragment instances from failed node {}",
                    reassigned.len(),
                    failed_node_id
                );
                // In a real system: re-dispatch the reassigned instances.
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn record_failure(&self, query_id: &QueryId, message: String) {
        // Best-effort: spawn a task to update the timeline.
        let timeline = self.timeline.clone();
        let qid = query_id.clone();
        tokio::spawn(async move {
            let mut tl = timeline.write().await;
            if let Some(handle) = tl.get_mut(&qid) {
                let _ = handle.fail(message);
            }
        });
    }
}

impl std::fmt::Debug for Coordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Coordinator")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let cluster = Arc::new(ClusterManager::new(Default::default()));
        let catalog = Arc::new(CatalogManager::new());
        let config = CoordinatorConfig::default();
        let _coordinator = Coordinator::new(cluster, catalog, config);
    }
}
