use crate::cost_model::{CostModel, CostModelConfig};
use crate::plan_node::*;
use crate::statistics::StatisticsProvider;
use std::sync::Arc;

/// Cost-based optimizer that applies physical optimization rules
/// after the rule-based optimizer has converged.
pub struct CboOptimizer {
    cost_model: CostModel,
}

impl CboOptimizer {
    pub fn new(stats_provider: Arc<dyn StatisticsProvider>) -> Self {
        let cost_model = CostModel::new(CostModelConfig::default())
            .with_stats_provider(stats_provider);
        Self { cost_model }
    }

    pub fn with_config(
        stats_provider: Arc<dyn StatisticsProvider>,
        config: CostModelConfig,
    ) -> Self {
        let cost_model = CostModel::new(config).with_stats_provider(stats_provider);
        Self { cost_model }
    }

    /// Apply CBO rules to the plan.
    pub fn optimize(&self, plan: PlanNode) -> PlanNode {
        self.choose_join_distribution(plan)
    }

    /// For each join node, decide whether to use Broadcast or Shuffle distribution
    /// based on estimated costs. When broadcast is cheaper, wrap the smaller side
    /// with an Exchange::Broadcast; otherwise use HashPartition on both sides.
    fn choose_join_distribution(&self, plan: PlanNode) -> PlanNode {
        match plan.node_type {
            PlanNodeType::Join(ref join) if plan.children.len() == 2 => {
                let left = self.choose_join_distribution(plan.children[0].clone());
                let right = self.choose_join_distribution(plan.children[1].clone());

                let left_rows = self.cost_model.estimate_rows(&left);
                let right_rows = self.cost_model.estimate_rows(&right);
                let left_bytes = self.cost_model.estimate_byte_size(&left);
                let right_bytes = self.cost_model.estimate_byte_size(&right);

                let threshold = self.cost_model.config().broadcast_threshold_bytes;

                let (build_side, build_rows, build_bytes, probe_rows) =
                    if right_rows <= left_rows {
                        (Side::Right, right_rows, right_bytes, left_rows)
                    } else {
                        (Side::Left, left_rows, left_bytes, right_rows)
                    };

                let should_broadcast = build_bytes < threshold;

                if should_broadcast {
                    // Insert Broadcast exchange on the build side
                    let broadcast_node = PlanNode {
                        id: left.id.clone(),
                        node_type: PlanNodeType::Exchange(ExchangeNode {
                            exchange_type: ExchangeType::Broadcast,
                        }),
                        children: match build_side {
                            Side::Left => vec![left.clone()],
                            Side::Right => vec![right.clone()],
                        },
                        stats: PlanStats::with_row_count(build_rows),
                    };

                    let (new_left, new_right) = match build_side {
                        Side::Left => (broadcast_node, right),
                        Side::Right => (left, broadcast_node),
                    };

                    PlanNode {
                        id: plan.id,
                        node_type: PlanNodeType::Join(join.clone()),
                        children: vec![new_left, new_right],
                        stats: PlanStats::with_row_count(
                            self.estimate_join_output_rows(probe_rows, build_rows),
                        ),
                    }
                } else {
                    // Use HashPartition on both sides
                    let num_partitions = 8;
                    let left_exchange = PlanNode {
                        id: left.id.clone(),
                        node_type: PlanNodeType::Exchange(ExchangeNode {
                            exchange_type: ExchangeType::HashPartition { num_partitions },
                        }),
                        children: vec![left],
                        stats: PlanStats::default(),
                    };
                    let right_exchange = PlanNode {
                        id: right.id.clone(),
                        node_type: PlanNodeType::Exchange(ExchangeNode {
                            exchange_type: ExchangeType::HashPartition { num_partitions },
                        }),
                        children: vec![right],
                        stats: PlanStats::default(),
                    };

                    PlanNode {
                        id: plan.id,
                        node_type: PlanNodeType::Join(join.clone()),
                        children: vec![left_exchange, right_exchange],
                        stats: PlanStats::with_row_count(
                            self.estimate_join_output_rows(probe_rows, build_rows),
                        ),
                    }
                }
            }
            _ => {
                let children: Vec<PlanNode> = plan
                    .children
                    .into_iter()
                    .map(|c| self.choose_join_distribution(c))
                    .collect();
                PlanNode {
                    id: plan.id,
                    node_type: plan.node_type,
                    children,
                    stats: plan.stats,
                }
            }
        }
    }

    fn estimate_join_output_rows(&self, left_rows: f64, right_rows: f64) -> f64 {
        // Default: cross product * 0.01 selectivity
        (left_rows * right_rows * 0.01).max(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Side {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::InMemoryStatsProvider;
    use crate::statistics::TableStats;
    use std::sync::Arc;

    fn make_scan_node(table: &str, db: &str, rows: f64) -> PlanNode {
        PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode {
                catalog: None,
                table_name: table.to_string(),
                database: Some(db.to_string()),
                columns: vec!["*".to_string()],
                predicates: vec![],
                limit: None,
            }),
            children: vec![],
            stats: PlanStats::with_row_count(rows),
        }
    }

    fn make_join_node(left: PlanNode, right: PlanNode) -> PlanNode {
        PlanNode {
            id: PlanNodeId(2),
            node_type: PlanNodeType::Join(JoinNode {
                join_type: JoinTypePlan::Inner,
                condition: Some("id = id".to_string()),
            }),
            children: vec![left, right],
            stats: PlanStats::default(),
        }
    }

    #[test]
    fn test_cbo_broadcast_for_small_table() {
        let mut provider = InMemoryStatsProvider::new();
        provider.add_table_stats("db", "small", TableStats::with_row_count(100));
        provider.add_table_stats("db", "large", TableStats::with_row_count(100000));

        let cbo = CboOptimizer::new(Arc::new(provider));
        let small = make_scan_node("small", "db", 100.0);
        let large = make_scan_node("large", "db", 100000.0);
        let join = make_join_node(large, small);

        let result = cbo.optimize(join);

        // Should have a Broadcast exchange for the small table
        fn has_broadcast(node: &PlanNode) -> bool {
            match &node.node_type {
                PlanNodeType::Exchange(ex) => matches!(ex.exchange_type, ExchangeType::Broadcast),
                _ => node.children.iter().any(has_broadcast),
            }
        }
        assert!(has_broadcast(&result), "Expected Broadcast exchange for small table");
    }

    #[test]
    fn test_cbo_shuffle_for_large_tables() {
        let mut provider = InMemoryStatsProvider::new();
        // Two large tables (rows = data_size/100 bytes per row)
        provider.add_table_stats("db", "t1", TableStats::with_row_count(10_000_000));
        provider.add_table_stats("db", "t2", TableStats::with_row_count(10_000_000));

        let cbo = CboOptimizer::new(Arc::new(provider));
        let t1 = make_scan_node("t1", "db", 10_000_000.0);
        let t2 = make_scan_node("t2", "db", 10_000_000.0);
        let join = make_join_node(t1, t2);

        let result = cbo.optimize(join);

        // Should NOT have Broadcast for large tables
        fn has_broadcast(node: &PlanNode) -> bool {
            match &node.node_type {
                PlanNodeType::Exchange(ex) => matches!(ex.exchange_type, ExchangeType::Broadcast),
                _ => node.children.iter().any(has_broadcast),
            }
        }
        fn has_hash_partition(node: &PlanNode) -> bool {
            match &node.node_type {
                PlanNodeType::Exchange(ex) => {
                    matches!(ex.exchange_type, ExchangeType::HashPartition { .. })
                }
                _ => node.children.iter().any(has_hash_partition),
            }
        }
        assert!(!has_broadcast(&result), "Should not broadcast large tables");
        assert!(has_hash_partition(&result), "Should use HashPartition for large tables");
    }
}
