use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cluster::{NodeAddress, NodeId};
use crate::exchange::{ExchangeDestination, ExchangeKind};
use fe_sql_planner::plan_node::{
    ExchangeNode, ExchangeType as PlanExchangeType, PlanNode, PlanNodeType,
};

// ---------------------------------------------------------------------------
// Fragment: a sub-plan that executes on one or more BE nodes
// ---------------------------------------------------------------------------

/// Unique identifier for a fragment within a query.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FragmentId(pub u64);

impl std::fmt::Display for FragmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "frag-{}", self.0)
    }
}

/// A fragment represents a stage of distributed execution. It contains a plan
/// subtree and knows its parallelism and how its output is distributed.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// Unique ID within the query.
    pub id: FragmentId,
    /// The plan subtree this fragment executes.
    pub plan: PlanNode,
    /// How many instances of this fragment to create.
    pub parallelism: usize,
    /// How this fragment's output is distributed to consumers.
    pub exchange_kind: Option<ExchangeKind>,
    /// ID of the parent fragment (None for the root fragment).
    pub parent_fragment_id: Option<FragmentId>,
    /// Child fragment IDs (fragments that feed data into this one).
    pub child_fragment_ids: Vec<FragmentId>,
    /// The assigned instances (populated during scheduling).
    pub instances: Vec<FragmentInstance>,
}

impl Fragment {
    /// Create a new fragment with no instances assigned.
    pub fn new(id: FragmentId, plan: PlanNode) -> Self {
        Self {
            id,
            plan,
            parallelism: 1,
            exchange_kind: None,
            parent_fragment_id: None,
            child_fragment_ids: Vec::new(),
            instances: Vec::new(),
        }
    }

    /// Returns true if this fragment has no parent (i.e., it is the root / final
    /// stage of the query).
    pub fn is_root(&self) -> bool {
        self.parent_fragment_id.is_none()
    }

    /// Returns true if this fragment has no children (i.e., it is a leaf stage
    /// that reads from storage).
    pub fn is_leaf(&self) -> bool {
        self.child_fragment_ids.is_empty()
    }
}

// ---------------------------------------------------------------------------
// FragmentInstance: a specific parallel instance of a fragment on a BE node
// ---------------------------------------------------------------------------

/// Unique identifier for a fragment instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FragmentInstanceId {
    pub fragment_id: FragmentId,
    pub index: usize,
}

impl std::fmt::Display for FragmentInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-inst-{}", self.fragment_id, self.index)
    }
}

/// A concrete instance of a fragment assigned to a specific BE node.
#[derive(Debug, Clone)]
pub struct FragmentInstance {
    /// Unique instance identifier.
    pub id: FragmentInstanceId,
    /// Which fragment this is an instance of.
    pub fragment_id: FragmentId,
    /// The BE node this instance is assigned to.
    pub node_id: NodeId,
    /// Address of the assigned BE node.
    pub node_address: NodeAddress,
    /// Execution parameters.
    pub params: ExecutionParams,
    /// Where this instance sends its output (exchange destinations).
    pub output_destinations: Vec<ExchangeDestination>,
}

/// Execution parameters passed to each fragment instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    /// The query this instance belongs to.
    pub query_id: String,
    /// Maximum execution time in milliseconds.
    pub timeout_ms: u64,
    /// Memory limit per instance in bytes.
    pub mem_limit_bytes: u64,
    /// Batch size for internal operators.
    pub batch_size: usize,
}

impl Default for ExecutionParams {
    fn default() -> Self {
        Self {
            query_id: String::new(),
            timeout_ms: 300_000,         // 5 minutes
            mem_limit_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            batch_size: 4096,
        }
    }
}

// ---------------------------------------------------------------------------
// Fragment tree builder
// ---------------------------------------------------------------------------

/// Result of splitting a plan into fragments.
#[derive(Debug, Clone)]
pub struct FragmentTree {
    /// All fragments indexed by their ID.
    pub fragments: HashMap<FragmentId, Fragment>,
    /// The root fragment ID (the final stage).
    pub root_fragment_id: FragmentId,
}

impl FragmentTree {
    /// Look up a fragment by ID.
    pub fn get(&self, id: &FragmentId) -> Option<&Fragment> {
        self.fragments.get(id)
    }

    /// Look up a fragment mutably by ID.
    pub fn get_mut(&mut self, id: &FragmentId) -> Option<&mut Fragment> {
        self.fragments.get_mut(id)
    }

    /// Return fragments in topological order: leaves first, root last.
    pub fn topological_order(&self) -> Vec<&Fragment> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.visit_children(&self.root_fragment_id, &mut order, &mut visited);
        order
    }

    fn visit_children<'a>(
        &'a self,
        id: &FragmentId,
        order: &mut Vec<&'a Fragment>,
        visited: &mut std::collections::HashSet<FragmentId>,
    ) {
        if visited.contains(id) {
            return;
        }
        visited.insert(id.clone());
        if let Some(frag) = self.fragments.get(id) {
            for child_id in &frag.child_fragment_ids {
                self.visit_children(child_id, order, visited);
            }
            order.push(frag);
        }
    }
}

/// Splits a logical plan into distributed fragments at exchange boundaries.
///
/// Walks the plan tree. Every `ExchangeNode` in the plan acts as a boundary:
/// the subtree below the exchange becomes one fragment, and the subtree above
/// continues to be split recursively.
pub struct Fragmentizer {
    next_fragment_id: u64,
}

impl Fragmentizer {
    pub fn new() -> Self {
        Self {
            next_fragment_id: 0,
        }
    }

    fn allocate_id(&mut self) -> FragmentId {
        let id = FragmentId(self.next_fragment_id);
        self.next_fragment_id += 1;
        id
    }

    /// Split the plan into a fragment tree.
    pub fn fragmentize(&mut self, plan: PlanNode) -> FragmentTree {
        let mut fragments = HashMap::new();
        let root_id = self.split_recursive(plan, &mut fragments, None);

        // Build child lists from parent references.
        let child_map = self.build_child_map(&fragments);
        for (parent_id, child_ids) in child_map {
            if let Some(parent) = fragments.get_mut(&parent_id) {
                parent.child_fragment_ids = child_ids;
            }
        }

        FragmentTree {
            fragments,
            root_fragment_id: root_id,
        }
    }

    /// Recursively walk the plan, splitting at exchange nodes.
    /// Returns the FragmentId of the fragment that contains the current plan node.
    fn split_recursive(
        &mut self,
        plan: PlanNode,
        fragments: &mut HashMap<FragmentId, Fragment>,
        parent_id: Option<FragmentId>,
    ) -> FragmentId {
        // Check if this node is an exchange boundary.
        match &plan.node_type {
            PlanNodeType::Exchange(exchange_node) => {
                // The exchange node itself becomes the boundary.
                // Children become their own fragments; this exchange starts a new fragment above.
                let exchange_kind = self.convert_exchange_type(exchange_node);

                // Process each child as a separate sub-plan.
                let mut child_fragment_ids = Vec::new();
                for child in plan.children.clone() {
                    let child_frag_id =
                        self.split_recursive(child, fragments, None /* set later */);
                    child_fragment_ids.push(child_frag_id);
                }

                // Create the fragment that contains this exchange (upper fragment).
                let frag_id = self.allocate_id();
                let mut fragment = Fragment::new(frag_id.clone(), plan);
                fragment.exchange_kind = exchange_kind;
                fragment.parent_fragment_id = parent_id;
                fragment.child_fragment_ids = child_fragment_ids.clone();
                // Parallelism for the consuming side.
                fragment.parallelism = self.infer_parallelism(&fragment.exchange_kind);

                // Set parent references on child fragments.
                for child_id in &child_fragment_ids {
                    if let Some(child_frag) = fragments.get_mut(child_id) {
                        child_frag.parent_fragment_id = Some(frag_id.clone());
                    }
                }

                fragments.insert(frag_id.clone(), fragment);
                frag_id
            }
            _ => {
                // Not an exchange boundary. Check if any children contain exchanges.
                let has_exchange_child = plan
                    .children
                    .iter()
                    .any(|c| matches!(c.node_type, PlanNodeType::Exchange(_)));

                if has_exchange_child {
                    // This node is above an exchange boundary.
                    // Process children first (they will create their own fragments).
                    let mut child_fragment_ids = Vec::new();
                    for child in plan.children.clone() {
                        let child_id = self.split_recursive(child, fragments, None);
                        child_fragment_ids.push(child_id);
                    }

                    // This node continues in the current (upper) fragment.
                    let frag_id = self.allocate_id();
                    let mut fragment = Fragment::new(frag_id.clone(), plan);
                    fragment.child_fragment_ids = child_fragment_ids.clone();
                    // Gather at the top if no explicit exchange.
                    fragment.exchange_kind = if parent_id.is_none() {
                        Some(ExchangeKind::Gather)
                    } else {
                        None
                    };
                    fragment.parallelism = 1;
                    fragment.parent_fragment_id = parent_id;

                    for child_id in &child_fragment_ids {
                        if let Some(child_frag) = fragments.get_mut(child_id) {
                            child_frag.parent_fragment_id = Some(frag_id.clone());
                        }
                    }

                    fragments.insert(frag_id.clone(), fragment);
                    frag_id
                } else {
                    // No exchange in the subtree: the entire subtree is one fragment.
                    let frag_id = self.allocate_id();
                    let mut fragment = Fragment::new(frag_id.clone(), plan);
                    fragment.exchange_kind = if parent_id.is_none() {
                        Some(ExchangeKind::Gather)
                    } else {
                        None
                    };
                    fragment.parallelism = 1;
                    fragment.parent_fragment_id = parent_id;
                    fragments.insert(frag_id.clone(), fragment);
                    frag_id
                }
            }
        }
    }

    /// Convert a plan-level exchange node into a scheduler-level exchange kind.
    fn convert_exchange_type(&self, node: &ExchangeNode) -> Option<ExchangeKind> {
        match node.exchange_type {
            PlanExchangeType::HashPartition { num_partitions } => Some(ExchangeKind::HashPartition {
                key_columns: vec![], // Will be filled in during full optimization.
                num_partitions,
            }),
            PlanExchangeType::Broadcast => Some(ExchangeKind::Broadcast),
            PlanExchangeType::Gather => Some(ExchangeKind::Gather),
            PlanExchangeType::RoundRobin { num_partitions } => Some(ExchangeKind::HashPartition {
                key_columns: vec![],
                num_partitions,
            }),
        }
    }

    /// Infer the parallelism for a fragment based on its exchange kind.
    fn infer_parallelism(&self, exchange_kind: &Option<ExchangeKind>) -> usize {
        match exchange_kind {
            Some(ExchangeKind::HashPartition { num_partitions, .. }) => *num_partitions,
            Some(ExchangeKind::Broadcast) => 1, // Broadcast consumers are typically 1-to-N
            Some(ExchangeKind::Gather) => 1,    // Gather has one consumer
            None => 1,
        }
    }

    /// Build a mapping from parent fragment ID to its child fragment IDs.
    fn build_child_map(
        &self,
        fragments: &HashMap<FragmentId, Fragment>,
    ) -> HashMap<FragmentId, Vec<FragmentId>> {
        let mut map: HashMap<FragmentId, Vec<FragmentId>> = HashMap::new();
        for (id, frag) in fragments {
            if let Some(parent_id) = &frag.parent_fragment_id {
                map.entry(parent_id.clone())
                    .or_default()
                    .push(id.clone());
            }
        }
        map
    }
}

impl Default for Fragmentizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe_sql_planner::plan_node::{PlanNodeId, PlanStats, ScanNode};

    #[test]
    fn test_single_fragment_no_exchange() {
        let plan = PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode {
                table_name: "t1".into(),
                database: None,
                columns: vec!["a".into()],
                predicates: vec![],
                limit: None,
            }),
            children: vec![],
            stats: PlanStats::default(),
        };

        let mut fragmentizer = Fragmentizer::new();
        let tree = fragmentizer.fragmentize(plan);

        assert_eq!(tree.fragments.len(), 1);
        assert!(tree.root_fragment_id == FragmentId(0));
    }

    #[test]
    fn test_fragment_tree_topological_order() {
        let plan = PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode {
                table_name: "t1".into(),
                database: None,
                columns: vec![],
                predicates: vec![],
                limit: None,
            }),
            children: vec![],
            stats: PlanStats::default(),
        };

        let mut fragmentizer = Fragmentizer::new();
        let tree = fragmentizer.fragmentize(plan);
        let order = tree.topological_order();

        assert_eq!(order.len(), 1);
    }
}
