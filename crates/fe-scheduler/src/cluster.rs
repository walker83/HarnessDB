use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use tracing::{info, warn};

/// Helper to get the current time as milliseconds since Unix epoch.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Unique identifier for a BE (Backend) node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "be-{}", self.0)
    }
}

/// Network address of a BE node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    pub host: String,
    pub rpc_port: u16,
    pub http_port: u16,
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.rpc_port)
    }
}

/// Current health status of a BE node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeHealth {
    /// Node is healthy and accepting work.
    Healthy,
    /// Node has missed heartbeats but may recover.
    Suspect,
    /// Node is confirmed down or administratively removed.
    Dead,
}

/// Load statistics reported by a BE node via heartbeat.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeLoad {
    /// CPU utilization as a fraction [0.0, 1.0].
    pub cpu_usage: f64,
    /// Memory usage as a fraction [0.0, 1.0].
    pub memory_usage: f64,
    /// Disk usage as a fraction [0.0, 1.0].
    pub disk_usage: f64,
    /// Number of fragment instances currently executing.
    pub running_fragments: u32,
    /// Available memory in bytes.
    pub available_memory_bytes: u64,
    /// Total memory in bytes.
    pub total_memory_bytes: u64,
}

impl NodeLoad {
    /// Compute a composite load score (lower is better for placement).
    pub fn score(&self) -> f64 {
        // Weighted combination favoring memory pressure and running fragment count.
        let memory_weight = 0.4;
        let cpu_weight = 0.3;
        let fragment_weight = 0.2;
        let disk_weight = 0.1;

        let fragment_score = (self.running_fragments as f64) / 100.0;

        (memory_weight * self.memory_usage)
            + (cpu_weight * self.cpu_usage)
            + (fragment_weight * fragment_score)
            + (disk_weight * self.disk_usage)
    }
}

/// Full descriptor for a registered BE node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeNode {
    pub id: NodeId,
    pub address: NodeAddress,
    pub health: NodeHealth,
    pub load: NodeLoad,
    /// Maximum number of concurrent fragment instances this node supports.
    pub capacity: u32,
    /// When the last heartbeat was received (unix timestamp millis).
    pub last_heartbeat: Option<u64>,
    /// When the node was first registered (unix timestamp millis).
    pub registered_at: u64,
}

impl BeNode {
    /// Whether the node can accept additional fragment instances.
    pub fn is_available(&self) -> bool {
        self.health == NodeHealth::Healthy && (self.load.running_fragments < self.capacity)
    }

    /// Number of remaining slots for fragment instances.
    pub fn available_slots(&self) -> u32 {
        self.capacity.saturating_sub(self.load.running_fragments)
    }
}

/// Configuration for cluster management.
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// How often to check for missed heartbeats.
    pub heartbeat_interval: Duration,
    /// How many heartbeats can be missed before marking a node Suspect.
    pub suspect_threshold: u32,
    /// How many heartbeats can be missed before marking a node Dead.
    pub dead_threshold: u32,
    /// Default fragment capacity for newly registered nodes.
    pub default_capacity: u32,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            suspect_threshold: 3,
            dead_threshold: 6,
            default_capacity: 64,
        }
    }
}

/// Manages BE node registration, health, and load information.
pub struct ClusterManager {
    config: ClusterConfig,
    next_node_id: AtomicU64,
    /// Interior-mutable node registry using DashMap for better concurrency.
    nodes: DashMap<NodeId, BeNode>,
}

impl ClusterManager {
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            config,
            next_node_id: AtomicU64::new(1),
            nodes: DashMap::new(),
        }
    }

    /// Register a new BE node. Returns its assigned NodeId.
    pub async fn register(&self, address: NodeAddress) -> NodeId {
        let id = NodeId(self.next_node_id.fetch_add(1, Ordering::Relaxed));
        let node = BeNode {
            id: id.clone(),
            address,
            health: NodeHealth::Healthy,
            load: NodeLoad::default(),
            capacity: self.config.default_capacity,
            last_heartbeat: Some(now_millis()),
            registered_at: now_millis(),
        };
        info!("BE node registered: {} at {}", id, node.address);
        self.nodes.insert(id.clone(), node);
        id
    }

    /// Deregister a BE node. Returns true if the node was found and removed.
    pub async fn deregister(&self, id: &NodeId) -> bool {
        let removed = self.nodes.remove(id).is_some();
        if removed {
            info!("BE node deregistered: {}", id);
        } else {
            warn!("Attempted to deregister unknown BE node: {}", id);
        }
        removed
    }

    /// Process an incoming heartbeat from a BE node, updating its load stats.
    pub async fn heartbeat(&self, id: &NodeId, load: NodeLoad) -> Result<(), String> {
        if let Some(mut node_ref) = self.nodes.get_mut(id) {
            node_ref.last_heartbeat = Some(now_millis());
            node_ref.load = load;
            // Promote suspect nodes back to healthy on heartbeat.
            if node_ref.health == NodeHealth::Suspect {
                node_ref.health = NodeHealth::Healthy;
            }
            Ok(())
        } else {
            Err(format!("unknown node {}", id))
        }
    }

    /// Check for nodes that have missed heartbeats and update their health status.
    /// Returns the list of node IDs that transitioned to Dead.
    pub async fn check_health(&self) -> Vec<NodeId> {
        let mut newly_dead = Vec::new();
        let now = now_millis();
        let suspect_cutoff_ms = self.config.suspect_threshold as u64 * self.config.heartbeat_interval.as_millis() as u64;
        let dead_cutoff_ms = self.config.dead_threshold as u64 * self.config.heartbeat_interval.as_millis() as u64;

        for mut node_ref in self.nodes.iter_mut() {
            let last = match node_ref.last_heartbeat {
                Some(t) => t,
                None => continue,
            };
            let elapsed_ms = now.saturating_sub(last);
            let elapsed_secs = elapsed_ms / 1000;

            if elapsed_ms > dead_cutoff_ms && node_ref.health != NodeHealth::Dead {
                warn!("BE node {} marked Dead (no heartbeat for {}s)", node_ref.id, elapsed_secs);
                node_ref.health = NodeHealth::Dead;
                newly_dead.push(node_ref.id.clone());
            } else if elapsed_ms > suspect_cutoff_ms && node_ref.health == NodeHealth::Healthy {
                warn!("BE node {} marked Suspect (no heartbeat for {}s)", node_ref.id, elapsed_secs);
                node_ref.health = NodeHealth::Suspect;
            }
        }
        newly_dead
    }

    /// Get a snapshot of all registered nodes.
    pub async fn all_nodes(&self) -> Vec<BeNode> {
        self.nodes.iter().map(|r| r.value().clone()).collect()
    }

    /// Get a snapshot of all healthy, available nodes.
    pub async fn available_nodes(&self) -> Vec<BeNode> {
        self.nodes
            .iter()
            .filter(|r| r.value().is_available())
            .map(|r| r.value().clone())
            .collect()
    }

    /// Select the best node for a new fragment instance based on load.
    /// Returns None if no nodes are available.
    pub async fn select_node(&self) -> Option<BeNode> {
        let nodes = self.available_nodes().await;
        nodes
            .into_iter()
            .min_by(|a, b| {
                a.load
                    .score()
                    .partial_cmp(&b.load.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Select N nodes for a parallel fragment, choosing the least-loaded ones.
    /// Returns as many nodes as are available up to `count`.
    pub async fn select_nodes(&self, count: usize) -> Vec<BeNode> {
        let mut nodes = self.available_nodes().await;
        nodes.sort_by(|a, b| {
            a.load
                .score()
                .partial_cmp(&b.load.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes.truncate(count);
        nodes
    }

    /// Look up a single node by ID.
    pub async fn get_node(&self, id: &NodeId) -> Option<BeNode> {
        self.nodes.get(id).map(|r| r.value().clone())
    }

    /// Mark a BE as dead
    pub async fn mark_dead(&mut self, node_id: NodeId) {
        if let Some(mut node_ref) = self.nodes.get_mut(&node_id) {
            node_ref.health = NodeHealth::Dead;
            warn!("BE node {} marked dead manually", node_id);
        }
    }

    /// Check if a BE is alive (Healthy status)
    pub async fn is_alive(&self, node_id: NodeId) -> bool {
        self.nodes
            .get(&node_id)
            .map(|r| r.value().health == NodeHealth::Healthy)
            .unwrap_or(false)
    }

    /// Get only alive (Healthy) nodes
    pub async fn get_alive_nodes(&self) -> Vec<BeNode> {
        self.nodes
            .iter()
            .filter(|r| r.value().health == NodeHealth::Healthy)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get composite load score for a node (lower is better)
    pub async fn node_load_score(&self, node_id: NodeId) -> f64 {
        self.nodes
            .get(&node_id)
            .map(|r| r.value().load.score())
            .unwrap_or(f64::MAX)
    }

    /// Number of registered nodes (regardless of health).
    pub async fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl std::fmt::Debug for ClusterManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterManager")
            .field("config", &self.config)
            .finish()
    }
}
