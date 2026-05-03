use std::sync::Arc;

use common::DrorisError;
use tokio::sync::RwLock;
use tracing::info;

use crate::cluster::{ClusterManager, NodeId};
use crate::exchange::{ExchangeDestination, ExchangeKind};
use crate::fragment::{
    ExecutionParams, Fragment, FragmentId, FragmentInstance, FragmentInstanceId, FragmentTree,
    Fragmentizer,
};
use fe_sql_planner::PlanNode;

// ---------------------------------------------------------------------------
// Resource limits
// ---------------------------------------------------------------------------

/// Per-query resource constraints.
#[derive(Debug, Clone)]
pub struct QueryLimits {
    pub timeout_ms: u64,
    pub mem_limit_bytes: u64,
    pub max_parallelism: usize,
    pub batch_size: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 300_000,
            mem_limit_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            max_parallelism: 16,
            batch_size: 4096,
        }
    }
}

/// Cluster-wide resource quotas.
#[derive(Debug, Clone)]
pub struct ClusterLimits {
    /// Maximum number of concurrently running queries across the cluster.
    pub max_concurrent_queries: usize,
    /// Maximum total fragment instances across all queries.
    pub max_total_fragments: usize,
    /// Maximum memory usable by all running queries combined.
    pub max_total_memory_bytes: u64,
}

impl Default for ClusterLimits {
    fn default() -> Self {
        Self {
            max_concurrent_queries: 100,
            max_total_fragments: 1000,
            max_total_memory_bytes: 64 * 1024 * 1024 * 1024, // 64 GiB
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduling strategy
// ---------------------------------------------------------------------------

/// Strategy for assigning fragment instances to BE nodes.
#[derive(Debug, Clone, Copy)]
pub enum SchedulingStrategy {
    /// Round-robin across available nodes.
    RoundRobin,
    /// Choose least-loaded node based on reported load stats.
    LoadAware,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        Self::LoadAware
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// The distributed scheduler is responsible for:
/// 1. Splitting a logical plan into fragments.
/// 2. Assigning fragment instances to BE nodes.
/// 3. Wiring up exchange destinations between instances.
/// 4. Tracking resource usage and re-scheduling on failure.
pub struct Scheduler {
    cluster: Arc<ClusterManager>,
    limits: ClusterLimits,
    strategy: SchedulingStrategy,
    /// Round-robin counter for RoundRobin strategy.
    rr_counter: Arc<RwLock<usize>>,
}

impl Scheduler {
    pub fn new(cluster: Arc<ClusterManager>) -> Self {
        Self {
            cluster,
            limits: ClusterLimits::default(),
            strategy: SchedulingStrategy::default(),
            rr_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub fn with_limits(mut self, limits: ClusterLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_strategy(mut self, strategy: SchedulingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Full scheduling pipeline: plan -> fragment tree -> assigned instances.
    pub async fn schedule(
        &self,
        plan: PlanNode,
        query_id: &str,
        query_limits: &QueryLimits,
    ) -> Result<FragmentTree, DrorisError> {
        // Step 1: Split the plan into fragments.
        let mut fragmentizer = Fragmentizer::new();
        let mut tree = fragmentizer.fragmentize(plan);

        // Step 2: Assign instances to each fragment in topological order (leaves first).
        let ordered_ids: Vec<FragmentId> = tree.topological_order().iter().map(|f| f.id.clone()).collect();

        for frag_id in &ordered_ids {
            let fragment = tree.get(frag_id).unwrap();
            let parallelism = fragment
                .parallelism
                .min(query_limits.max_parallelism)
                .max(1);

            let instances = self
                .assign_instances(fragment, parallelism, query_id, query_limits)
                .await?;

            if let Some(frag) = tree.get_mut(frag_id) {
                frag.instances = instances;
            }
        }

        // Step 3: Wire exchange destinations between parent and child instances.
        self.wire_exchanges(&mut tree)?;

        info!(
            "Scheduled query {}: {} fragments, {} total instances",
            query_id,
            tree.fragments.len(),
            tree.fragments
                .values()
                .map(|f| f.instances.len())
                .sum::<usize>()
        );

        Ok(tree)
    }

    /// Assign fragment instances to BE nodes using the configured strategy.
    async fn assign_instances(
        &self,
        fragment: &Fragment,
        parallelism: usize,
        query_id: &str,
        query_limits: &QueryLimits,
    ) -> Result<Vec<FragmentInstance>, DrorisError> {
        let nodes = match self.strategy {
            SchedulingStrategy::RoundRobin => {
                self.select_round_robin(parallelism).await
            }
            SchedulingStrategy::LoadAware => {
                self.cluster.select_nodes(parallelism).await
            }
        };

        let selected_nodes = if nodes.is_empty() {
            // Fallback: try any available node.
            let fallback = self.cluster.available_nodes().await;
            if fallback.is_empty() {
                return Err(DrorisError::Internal(
                    "no available BE nodes for scheduling".into(),
                ));
            }
            // Replicate the first available node to fill parallelism.
            let mut result = Vec::with_capacity(parallelism);
            for i in 0..parallelism {
                let node = &fallback[i % fallback.len()];
                result.push(node.clone());
            }
            result
        } else {
            nodes
        };

        // Build FragmentInstance objects from the selected BE nodes.
        let instances: Vec<FragmentInstance> = selected_nodes
            .into_iter()
            .enumerate()
            .map(|(idx, node)| FragmentInstance {
                id: FragmentInstanceId {
                    fragment_id: fragment.id.clone(),
                    index: idx,
                },
                fragment_id: fragment.id.clone(),
                node_id: node.id,
                node_address: node.address,
                params: ExecutionParams {
                    query_id: query_id.to_string(),
                    timeout_ms: query_limits.timeout_ms,
                    mem_limit_bytes: query_limits.mem_limit_bytes,
                    batch_size: query_limits.batch_size,
                },
                output_destinations: Vec::new(),
            })
            .collect();

        Ok(instances)
    }

    /// Select nodes using round-robin.
    async fn select_round_robin(&self, count: usize) -> Vec<crate::cluster::BeNode> {
        let available = self.cluster.available_nodes().await;
        if available.is_empty() {
            return vec![];
        }

        let mut counter = self.rr_counter.write().await;
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let idx = *counter % available.len();
            selected.push(available[idx].clone());
            *counter += 1;
        }
        selected
    }

    /// After instances are assigned, wire the exchange destinations so that
    /// each producer instance knows where to send its output.
    fn wire_exchanges(&self, tree: &mut FragmentTree) -> Result<(), DrorisError> {
        // For each fragment, if it has a parent, set up destinations from
        // this fragment's instances to the parent's instances.
        let fragment_ids: Vec<FragmentId> = tree.fragments.keys().cloned().collect();

        // Collect (child_frag_id, parent_frag_id, exchange_kind) triples.
        let mut wiring_info: Vec<(FragmentId, FragmentId, ExchangeKind)> = Vec::new();

        for frag_id in &fragment_ids {
            let fragment = tree.get(frag_id).unwrap();
            if let Some(parent_id) = &fragment.parent_fragment_id {
                let exchange_kind = fragment
                    .exchange_kind
                    .clone()
                    .unwrap_or(ExchangeKind::Gather);
                wiring_info.push((frag_id.clone(), parent_id.clone(), exchange_kind));
            }
        }

        // Apply wiring: for each child fragment, set output_destinations on its instances.
        for (child_id, parent_id, exchange_kind) in &wiring_info {
            let parent_instances: Vec<FragmentInstance> = tree
                .get(parent_id)
                .map(|f| f.instances.clone())
                .unwrap_or_default();

            let child_instances: Vec<FragmentInstance> = tree
                .get(child_id)
                .map(|f| f.instances.clone())
                .unwrap_or_default();

            // Build destinations based on exchange kind.
            let destinations = self.build_destinations(
                &child_instances,
                &parent_instances,
                exchange_kind,
            );

            // Assign destinations to child instances.
            if let Some(child_frag) = tree.get_mut(child_id) {
                for (idx, dests) in destinations.into_iter().enumerate() {
                    if idx < child_frag.instances.len() {
                        child_frag.instances[idx].output_destinations = dests;
                    }
                }
            }
        }

        Ok(())
    }

    /// Build the destination lists for each child instance based on exchange kind.
    fn build_destinations(
        &self,
        child_instances: &[FragmentInstance],
        parent_instances: &[FragmentInstance],
        exchange_kind: &ExchangeKind,
    ) -> Vec<Vec<ExchangeDestination>> {
        match exchange_kind {
            ExchangeKind::HashPartition {
                num_partitions,
                key_columns: _,
            } => {
                // Each child instance sends to specific parent partitions.
                let mut result = Vec::with_capacity(child_instances.len());
                for child in child_instances {
                    let dests: Vec<ExchangeDestination> = parent_instances
                        .iter()
                        .enumerate()
                        .map(|(part, parent)| ExchangeDestination {
                            node: parent.node_address.clone(),
                            target_instance_id: parent.id.to_string(),
                            channel: crate::exchange::ChannelId {
                                fragment_instance_id: child.id.to_string(),
                                partition: part,
                            },
                        })
                        .collect();
                    result.push(dests);
                }
                result
            }
            ExchangeKind::Broadcast => {
                // Each child sends to ALL parent instances.
                let mut result = Vec::with_capacity(child_instances.len());
                for child in child_instances {
                    let dests: Vec<ExchangeDestination> = parent_instances
                        .iter()
                        .map(|parent| ExchangeDestination {
                            node: parent.node_address.clone(),
                            target_instance_id: parent.id.to_string(),
                            channel: crate::exchange::ChannelId {
                                fragment_instance_id: child.id.to_string(),
                                partition: 0,
                            },
                        })
                        .collect();
                    result.push(dests);
                }
                result
            }
            ExchangeKind::Gather => {
                // All children send to the single parent instance (or first if multiple).
                let mut result = Vec::with_capacity(child_instances.len());
                for child in child_instances {
                    let dests: Vec<ExchangeDestination> = parent_instances
                        .iter()
                        .map(|parent| ExchangeDestination {
                            node: parent.node_address.clone(),
                            target_instance_id: parent.id.to_string(),
                            channel: crate::exchange::ChannelId {
                                fragment_instance_id: child.id.to_string(),
                                partition: 0,
                            },
                        })
                        .collect();
                    result.push(dests);
                }
                result
            }
        }
    }

    /// Re-schedule fragments that were on a failed node.
    /// Returns the set of fragment instance IDs that were re-assigned.
    pub async fn reschedule_on_failure(
        &self,
        tree: &mut FragmentTree,
        failed_node: &NodeId,
        query_id: &str,
        query_limits: &QueryLimits,
    ) -> Result<Vec<FragmentInstanceId>, DrorisError> {
        let replacement = self.cluster.select_node().await;

        match replacement {
            Some(new_node) => {
                let mut reassigned = Vec::new();
                for (_, fragment) in tree.fragments.iter_mut() {
                    for instance in &mut fragment.instances {
                        if instance.node_id == *failed_node {
                            info!(
                                "Re-assigning instance {} from failed node {} to {}",
                                instance.id, failed_node, new_node.id
                            );
                            instance.node_id = new_node.id.clone();
                            instance.node_address = new_node.address.clone();
                            instance.params.query_id = query_id.to_string();
                            reassigned.push(instance.id.clone());
                        }
                    }
                }
                // Re-wire exchanges after reassignment.
                self.wire_exchanges(tree)?;
                Ok(reassigned)
            }
            None => {
                Err(DrorisError::Internal(format!(
                    "cannot re-schedule fragments: no available nodes to replace {}",
                    failed_node
                )))
            }
        }
    }

    /// Check cluster-level resource limits before admitting a new query.
    pub async fn check_cluster_limits(&self, running_queries: usize) -> Result<(), DrorisError> {
        if running_queries >= self.limits.max_concurrent_queries {
            return Err(DrorisError::Query(format!(
                "cluster concurrent query limit reached ({}/{})",
                running_queries, self.limits.max_concurrent_queries
            )));
        }
        Ok(())
    }
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("strategy", &self.strategy)
            .field("limits", &self.limits)
            .finish()
    }
}
