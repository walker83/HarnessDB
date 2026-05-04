use crate::plan_node::*;
use crate::statistics::StatisticsProvider;
use std::sync::Arc;

/// Cost in abstract cost units. Weights are configurable via CostModelConfig.
#[derive(Debug, Clone, Default)]
pub struct Cost {
    pub cpu: f64,
    pub io: f64,
    pub network: f64,
    pub memory: f64,
}

impl Cost {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn cpu(cost: f64) -> Self {
        Self {
            cpu: cost,
            ..Self::default()
        }
    }

    pub fn io(cost: f64) -> Self {
        Self {
            io: cost,
            ..Self::default()
        }
    }

    pub fn network(cost: f64) -> Self {
        Self {
            network: cost,
            ..Self::default()
        }
    }
}

impl std::ops::Add for Cost {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self.cpu += rhs.cpu;
        self.io += rhs.io;
        self.network += rhs.network;
        self.memory += rhs.memory;
        self
    }
}

impl std::ops::AddAssign for Cost {
    fn add_assign(&mut self, rhs: Self) {
        self.cpu += rhs.cpu;
        self.io += rhs.io;
        self.network += rhs.network;
        self.memory += rhs.memory;
    }
}

#[derive(Debug, Clone)]
pub struct CostModelConfig {
    pub cpu_weight: f64,
    pub io_weight: f64,
    pub network_weight: f64,
    pub memory_weight: f64,
    pub scan_cost_per_row: f64,
    pub filter_cost_per_row: f64,
    pub join_probe_cost_per_row: f64,
    pub join_build_cost_per_row: f64,
    pub aggregate_cost_per_row: f64,
    pub sort_cost_per_row: f64,
    pub network_cost_per_row: f64,
    pub broadcast_threshold_bytes: f64,
    pub default_row_width: f64,
}

impl Default for CostModelConfig {
    fn default() -> Self {
        Self {
            cpu_weight: 1.0,
            io_weight: 10.0,
            network_weight: 5.0,
            memory_weight: 2.0,
            scan_cost_per_row: 1.0,
            filter_cost_per_row: 0.5,
            join_probe_cost_per_row: 1.5,
            join_build_cost_per_row: 2.0,
            aggregate_cost_per_row: 1.0,
            sort_cost_per_row: 2.0,
            network_cost_per_row: 1.0,
            broadcast_threshold_bytes: 64.0 * 1024.0 * 1024.0, // 64 MB
            default_row_width: 100.0,
        }
    }
}

pub struct CostModel {
    config: CostModelConfig,
    stats_provider: Option<Arc<dyn StatisticsProvider>>,
}

impl CostModel {
    pub fn new(config: CostModelConfig) -> Self {
        Self {
            config,
            stats_provider: None,
        }
    }

    pub fn with_stats_provider(mut self, provider: Arc<dyn StatisticsProvider>) -> Self {
        self.stats_provider = Some(provider);
        self
    }

    pub fn config(&self) -> &CostModelConfig {
        &self.config
    }

    /// Compute the weighted total cost.
    pub fn total_cost(&self, cost: &Cost) -> f64 {
        cost.cpu * self.config.cpu_weight
            + cost.io * self.config.io_weight
            + cost.network * self.config.network_weight
            + cost.memory * self.config.memory_weight
    }

    /// Estimate cost for a full table scan.
    pub fn cost_scan(&self, row_count: f64, byte_size: f64) -> Cost {
        Cost {
            io: byte_size / 1024.0,
            cpu: row_count * self.config.scan_cost_per_row,
            ..Cost::zero()
        }
    }

    /// Estimate cost for a filter operation.
    pub fn cost_filter(&self, input_rows: f64) -> Cost {
        Cost::cpu(input_rows * self.config.filter_cost_per_row)
    }

    /// Estimate cost for a hash join.
    pub fn cost_hash_join(
        &self,
        build_rows: f64,
        build_byte_size: f64,
        probe_rows: f64,
    ) -> Cost {
        Cost {
            cpu: build_rows * self.config.join_build_cost_per_row
                + probe_rows * self.config.join_probe_cost_per_row,
            memory: build_byte_size,
            ..Cost::zero()
        }
    }

    /// Estimate cost for broadcasting byte_size to num_receivers.
    pub fn cost_broadcast(&self, byte_size: f64, num_receivers: usize) -> Cost {
        Cost::network(byte_size * num_receivers as f64 / 1024.0)
    }

    /// Estimate cost for hash-partitioning rows across num_partitions.
    pub fn cost_shuffle(&self, rows: f64, byte_size: f64, num_partitions: usize) -> Cost {
        Cost {
            cpu: rows * 0.5,
            network: byte_size / 1024.0 * num_partitions as f64,
            ..Cost::zero()
        }
    }

    /// Estimate cost for aggregation.
    pub fn cost_aggregate(&self, input_rows: f64, group_ndv: f64) -> Cost {
        Cost {
            cpu: input_rows * self.config.aggregate_cost_per_row,
            memory: group_ndv * self.config.default_row_width,
            ..Cost::zero()
        }
    }

    /// Estimate cost for sorting (n * log(n)).
    pub fn cost_sort(&self, input_rows: f64) -> Cost {
        let n = input_rows.max(1.0);
        Cost::cpu(n * n.log2() * self.config.sort_cost_per_row)
    }

    /// Get table row count from stats provider.
    pub fn get_table_rows(&self, database: &Option<String>, table: &str) -> f64 {
        if let Some(ref provider) = self.stats_provider {
            let db = database.as_deref().unwrap_or("");
            if let Some(stats) = provider.get_table_stats(db, table) {
                return stats.row_count as f64;
            }
        }
        1000.0
    }

    /// Estimate output rows for a plan node.
    pub fn estimate_rows(&self, plan: &PlanNode) -> f64 {
        if plan.stats.row_count > 0.0 {
            return plan.stats.row_count;
        }
        match &plan.node_type {
            PlanNodeType::Scan(scan) => {
                let base = self.get_table_rows(&scan.database, &scan.table_name);
                let selectivity = self.estimate_predicates_selectivity(
                    &scan.predicates,
                    &scan.database,
                    &scan.table_name,
                );
                if let Some(limit) = scan.limit {
                    (base * selectivity).min(limit as f64)
                } else {
                    base * selectivity
                }
            }
            PlanNodeType::Filter(filter) => {
                let input = plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                let sel = self.estimate_expression_selectivity(&filter.predicate);
                input * sel
            }
            PlanNodeType::Aggregate(agg) => {
                let input = plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                if agg.group_by.is_empty() {
                    1.0
                } else {
                    input * 0.1
                }
            }
            PlanNodeType::Limit(lim) => {
                let input = plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                input.min(lim.limit as f64)
            }
            PlanNodeType::Join(_) | PlanNodeType::HashJoin(_) | PlanNodeType::MergeJoin(_) => {
                let left = plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                let right = plan.children.get(1).map(|c| self.estimate_rows(c)).unwrap_or(1000.0);
                // Default join selectivity: 1/max(ndv_left, ndv_right)
                (left * right) * 0.01
            }
            PlanNodeType::SemiJoin(_) | PlanNodeType::AntiSemiJoin(_) => {
                plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0) * 0.5
            }
            _ => plan.children.first().map(|c| self.estimate_rows(c)).unwrap_or(1000.0),
        }
    }

    /// Estimate byte size for a plan's output.
    pub fn estimate_byte_size(&self, plan: &PlanNode) -> f64 {
        self.estimate_rows(plan) * self.config.default_row_width
    }

    /// Estimate selectivity for pushed-down predicates.
    fn estimate_predicates_selectivity(
        &self,
        predicates: &[String],
        database: &Option<String>,
        table: &str,
    ) -> f64 {
        if predicates.is_empty() {
            return 1.0;
        }
        let mut sel = 1.0;
        for pred in predicates {
            sel *= self.estimate_expression_selectivity_with_stats(pred, database, table);
        }
        sel
    }

    /// Estimate selectivity for a single expression using stats when available.
    fn estimate_expression_selectivity_with_stats(
        &self,
        expr: &str,
        database: &Option<String>,
        table: &str,
    ) -> f64 {
        if let Some(ref provider) = self.stats_provider {
            let db = database.as_deref().unwrap_or("");
            if let Some(stats) = provider.get_table_stats(db, table) {
                return self.selectivity_from_stats(expr, &stats);
            }
        }
        self.estimate_expression_selectivity(expr)
    }

    /// Try to estimate selectivity using table statistics.
    fn selectivity_from_stats(&self, expr: &str, stats: &crate::statistics::TableStats) -> f64 {
        let expr = expr.trim();
        // Equality: col = value
        if let Some(eq_pos) = expr.find('=') {
            let left = expr[..eq_pos].trim();
            let col = Self::extract_column_name(left);
            if !col.is_empty() {
                return stats.estimate_selectivity(&col);
            }
        }
        // Range: col > value, col < value, col >= value, col <= value
        for op in [">=", "<=", ">", "<"] {
            if let Some(pos) = expr.find(op) {
                let col = Self::extract_column_name(expr[..pos].trim());
                if !col.is_empty() {
                    // For range predicates, estimate 1/3 selectivity if no histogram
                    return 0.33;
                }
            }
        }
        // BETWEEN
        if expr.to_uppercase().contains("BETWEEN") {
            return 0.33;
        }
        // IN list
        if expr.to_uppercase().contains("IN") {
            return 0.1;
        }
        // IS NULL
        if expr.to_uppercase().contains("IS NULL") {
            return 0.01;
        }
        // AND/OR handled via predicate splitting
        if expr.to_uppercase().contains(" AND ") {
            return 0.3 * 0.3; // independent assumption
        }
        0.3
    }

    /// Fallback selectivity estimation without statistics.
    fn estimate_expression_selectivity(&self, expr: &str) -> f64 {
        let expr = expr.trim().to_uppercase();
        if expr.contains('=') && !expr.contains("!=") && !expr.contains("<>") {
            0.1 // equality
        } else if expr.contains(" BETWEEN ") {
            0.33
        } else if expr.contains(" IN ") {
            0.1
        } else if expr.contains(" IS NULL") {
            0.01
        } else if expr.contains(" AND ") {
            0.3 * 0.3
        } else if expr.contains(" OR ") {
            1.0 - (1.0 - 0.3) * (1.0 - 0.3)
        } else {
            0.3 // default comparison selectivity
        }
    }

    fn extract_column_name(token: &str) -> String {
        let cleaned = token
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .trim();
        if cleaned.contains('.') {
            if let Some(pos) = cleaned.rfind('.') {
                let col = cleaned[pos + 1..].trim();
                if !col.is_empty() && col.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    return col.to_string();
                }
            }
        }
        if !cleaned.is_empty()
            && cleaned.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
        {
            cleaned.to_string()
        } else {
            String::new()
        }
    }

    /// Compute total estimated cost for an entire plan tree.
    pub fn estimate_plan_cost(&self, plan: &PlanNode) -> Cost {
        let children_cost: Cost = plan
            .children
            .iter()
            .map(|c| self.estimate_plan_cost(c))
            .fold(Cost::zero(), |acc, c| acc + c);

        let node_cost = match &plan.node_type {
            PlanNodeType::Scan(scan) => {
                let rows = self.get_table_rows(&scan.database, &scan.table_name);
                let bytes = rows * self.config.default_row_width;
                self.cost_scan(rows, bytes)
            }
            PlanNodeType::Filter(_) => {
                let input_rows = plan
                    .children
                    .first()
                    .map(|c| self.estimate_rows(c))
                    .unwrap_or(1000.0);
                self.cost_filter(input_rows)
            }
            PlanNodeType::Aggregate(agg) => {
                let input_rows = plan
                    .children
                    .first()
                    .map(|c| self.estimate_rows(c))
                    .unwrap_or(1000.0);
                let group_ndv = if agg.group_by.is_empty() {
                    1.0
                } else {
                    input_rows * 0.1
                };
                self.cost_aggregate(input_rows, group_ndv)
            }
            PlanNodeType::Sort(_) => {
                let input_rows = plan
                    .children
                    .first()
                    .map(|c| self.estimate_rows(c))
                    .unwrap_or(1000.0);
                self.cost_sort(input_rows)
            }
            PlanNodeType::Join(_) | PlanNodeType::HashJoin(_) => {
                let left_rows = plan
                    .children
                    .first()
                    .map(|c| self.estimate_rows(c))
                    .unwrap_or(1000.0);
                let right_rows = plan
                    .children
                    .get(1)
                    .map(|c| self.estimate_rows(c))
                    .unwrap_or(1000.0);
                let right_bytes = right_rows * self.config.default_row_width;
                self.cost_hash_join(right_rows.min(left_rows), right_bytes, right_rows.max(left_rows))
            }
            PlanNodeType::Exchange(ex) => match ex.exchange_type {
                ExchangeType::Broadcast => {
                    let bytes = plan
                        .children
                        .first()
                        .map(|c| self.estimate_byte_size(c))
                        .unwrap_or(0.0);
                    self.cost_broadcast(bytes, 3)
                }
                ExchangeType::HashPartition { num_partitions } => {
                    let rows = plan
                        .children
                        .first()
                        .map(|c| self.estimate_rows(c))
                        .unwrap_or(0.0);
                    let bytes = plan
                        .children
                        .first()
                        .map(|c| self.estimate_byte_size(c))
                        .unwrap_or(0.0);
                    self.cost_shuffle(rows, bytes, num_partitions)
                }
                _ => Cost::zero(),
            },
            _ => Cost::zero(),
        };

        children_cost + node_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_addition() {
        let a = Cost {
            cpu: 1.0,
            io: 2.0,
            network: 3.0,
            memory: 4.0,
        };
        let b = Cost {
            cpu: 10.0,
            io: 20.0,
            network: 30.0,
            memory: 40.0,
        };
        let c = a + b;
        assert_eq!(c.cpu, 11.0);
        assert_eq!(c.io, 22.0);
        assert_eq!(c.network, 33.0);
        assert_eq!(c.memory, 44.0);
    }

    #[test]
    fn test_cost_model_scan() {
        let model = CostModel::new(CostModelConfig::default());
        let cost = model.cost_scan(1000.0, 100_000.0);
        assert!(cost.cpu > 0.0);
        assert!(cost.io > 0.0);
        assert!(model.total_cost(&cost) > 0.0);
    }

    #[test]
    fn test_cost_model_join() {
        let model = CostModel::new(CostModelConfig::default());
        let cost = model.cost_hash_join(100.0, 10_000.0, 10000.0);
        assert!(cost.cpu > 0.0);
        assert!(cost.memory > 0.0);
        assert!(model.total_cost(&cost) > 0.0);
    }

    #[test]
    fn test_cost_model_sort() {
        let model = CostModel::new(CostModelConfig::default());
        let cost = model.cost_sort(1000.0);
        assert!(cost.cpu > 0.0);
        // 1000 * log2(1000) * 2.0 ≈ 19931
        assert!(cost.cpu > 10000.0);
    }

    #[test]
    fn test_broadcast_vs_shuffle() {
        let model = CostModel::new(CostModelConfig::default());
        let small_bytes = 1024.0 * 1024.0; // 1 MB
        let large_bytes = 512.0 * 1024.0 * 1024.0; // 512 MB
        let num_nodes = 3;

        let broadcast_small = model.cost_broadcast(small_bytes, num_nodes);
        let broadcast_large = model.cost_broadcast(large_bytes, num_nodes);

        // Small broadcast should be cheaper than large broadcast
        assert!(model.total_cost(&broadcast_small) < model.total_cost(&broadcast_large));
    }

    #[test]
    fn test_estimate_expression_selectivity() {
        let model = CostModel::new(CostModelConfig::default());
        assert!((model.estimate_expression_selectivity("a = 5") - 0.1).abs() < 0.001);
        assert!((model.estimate_expression_selectivity("a BETWEEN 1 AND 10") - 0.33).abs() < 0.001);
        assert!((model.estimate_expression_selectivity("a IN (1,2,3)") - 0.1).abs() < 0.001);
        assert!((model.estimate_expression_selectivity("a IS NULL") - 0.01).abs() < 0.001);
        assert!((model.estimate_expression_selectivity("a > 10") - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_estimate_rows_scan() {
        let model = CostModel::new(CostModelConfig::default());
        let scan = PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode {
                table_name: "t1".into(),
                database: Some("db".into()),
                columns: vec!["a".into()],
                predicates: vec![],
                limit: None,
            }),
            children: vec![],
            stats: PlanStats::default(),
        };
        // Without stats provider, returns 1000
        let rows = model.estimate_rows(&scan);
        assert_eq!(rows, 1000.0);
    }
}
