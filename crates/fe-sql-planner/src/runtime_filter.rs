use crate::plan_node::*;

static NEXT_FILTER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_filter_id() -> u64 {
    NEXT_FILTER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

pub struct RuntimeFilterRule {
    pub enabled: bool,
    pub max_filters_per_join: usize,
    pub bloom_filter_threshold_rows: usize,
    pub in_filter_threshold_rows: usize,
}

impl RuntimeFilterRule {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_filters_per_join: 4,
            bloom_filter_threshold_rows: 10000,
            in_filter_threshold_rows: 100,
        }
    }

    pub fn apply(&self, plan: PlanNode) -> PlanNode {
        if !self.enabled {
            return plan;
        }
        self.rewrite(plan)
    }

    fn rewrite(&self, plan: PlanNode) -> PlanNode {
        let id = plan.id.clone();
        let stats = plan.stats.clone();
        let children: Vec<PlanNode> = plan.children.into_iter().map(|c| self.rewrite(c)).collect();

        match plan.node_type {
            PlanNodeType::HashJoin(mut hj) => {
                if self.is_runnable_join(&hj) && children.len() == 2 {
                    let build_rows = self.estimate_rows(&children[1]);
                    let probe_rows = self.estimate_rows(&children[0]);

                    if build_rows > 0.0 && probe_rows > 0.0 && probe_rows > build_rows {
                        let filters = self.generate_runtime_filters(&hj, build_rows as usize);
                        if !filters.is_empty() {
                            hj.build_filters = filters;
                        }
                    }
                }
                PlanNode { id, node_type: PlanNodeType::HashJoin(hj), children, stats }
            }
            _ => PlanNode { id, node_type: plan.node_type, children, stats },
        }
    }

    fn is_runnable_join(&self, hj: &HashJoinNode) -> bool {
        matches!(
            hj.join_type,
            JoinTypePlan::Inner
        ) && !hj.build_keys.is_empty() && hj.build_keys.len() == hj.probe_keys.len()
    }

    fn generate_runtime_filters(
        &self,
        hj: &HashJoinNode,
        build_rows: usize,
    ) -> Vec<RuntimeFilterPlan> {
        let mut filters = Vec::new();

        for (build_col, probe_col) in hj.build_keys.iter().zip(hj.probe_keys.iter()) {
            if filters.len() >= self.max_filters_per_join {
                break;
            }

            let filter_type = self.select_filter_type(build_rows);

            filters.push(RuntimeFilterPlan {
                id: next_filter_id(),
                filter_type,
                build_column: build_col.clone(),
                probe_column: probe_col.clone(),
            });
        }

        filters
    }

    fn select_filter_type(&self, build_rows: usize) -> RuntimeFilterTypePlan {
        if build_rows <= self.in_filter_threshold_rows {
            RuntimeFilterTypePlan::In
        } else if build_rows <= self.bloom_filter_threshold_rows {
            RuntimeFilterTypePlan::MinMax
        } else {
            RuntimeFilterTypePlan::Bloom
        }
    }

    fn estimate_rows(&self, plan: &PlanNode) -> f64 {
        if plan.stats.row_count > 0.0 {
            return plan.stats.row_count;
        }
        match &plan.node_type {
            PlanNodeType::Scan(_) => 1000.0,
            PlanNodeType::Filter(_) => plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0) * 0.3,
            PlanNodeType::Aggregate(_) => plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0) * 0.1,
            PlanNodeType::Limit(lim) => plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0).min(lim.limit as f64),
            PlanNodeType::Join(_) | PlanNodeType::HashJoin(_) | PlanNodeType::MergeJoin(_) => {
                let l = plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                let r = plan.children.get(1).map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                (l * r) * 0.01
            }
            _ => plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0),
        }
    }
}

impl Default for RuntimeFilterRule {
    fn default() -> Self {
        Self::new()
    }
}