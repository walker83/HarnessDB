use crate::plan_node::*;
use crate::statistics::StatisticsProvider;
use std::collections::HashSet;
use std::sync::Arc;

pub struct Optimizer {
    stats_provider: Option<Arc<dyn StatisticsProvider>>,
}

impl Optimizer {
    pub fn new() -> Self { Self { stats_provider: None } }

    pub fn with_stats_provider(mut self, provider: Arc<dyn StatisticsProvider>) -> Self {
        self.stats_provider = Some(provider);
        self
    }

    pub fn optimize(&self, plan: PlanNode) -> PlanNode {
        const MAX_ITERATIONS: usize = 10;
        let mut plan = plan;
        for _ in 0..MAX_ITERATIONS {
            let before = format!("{}", plan);
            plan = self.apply_rules(plan);
            let after = format!("{}", plan);
            if before == after { break; }
        }
        plan
    }

    fn apply_rules(&self, plan: PlanNode) -> PlanNode {
        let plan = self.push_down_predicates(plan);
        let plan = self.prune_columns(plan);
        let plan = self.push_down_limit(plan);
        let plan = self.reorder_joins(plan);
        plan
    }

    fn push_down_predicates(&self, plan: PlanNode) -> PlanNode {
        let id = plan.id.clone();
        let stats = plan.stats.clone();
        match plan.node_type {
            PlanNodeType::Filter(filter) => {
                if plan.children.len() != 1 { return self.rebuild_children(PlanNode { id, node_type: PlanNodeType::Filter(filter), children: plan.children, stats }, |c| self.push_down_predicates(c)); }
                let child = plan.children.into_iter().next().unwrap();
                match child.node_type {
                    PlanNodeType::Project(_) => {
                        let grandchild = if child.children.len() == 1 {
                            child.children.into_iter().next().unwrap()
                        } else {
                            return PlanNode { id, node_type: PlanNodeType::Filter(filter), children: vec![self.push_down_predicates(child)], stats };
                        };
                        let pushed = PlanNode { id: child.id, node_type: PlanNodeType::Filter(filter), children: vec![self.push_down_predicates(grandchild)], stats: child.stats };
                        PlanNode { id, node_type: child.node_type, children: vec![pushed], stats }
                    }
                    PlanNodeType::Filter(inner) => {
                        let merged = format!("({}) AND ({})", filter.predicate, inner.predicate);
                        self.push_down_predicates(PlanNode { id, node_type: PlanNodeType::Filter(FilterNode { predicate: merged }), children: child.children, stats })
                    }
                    PlanNodeType::Scan(mut scan) => {
                        scan.predicates.push(filter.predicate);
                        PlanNode { id: child.id, node_type: PlanNodeType::Scan(scan), children: vec![], stats: child.stats }
                    }
                    _ => {
                        PlanNode { id, node_type: PlanNodeType::Filter(filter), children: vec![self.push_down_predicates(child)], stats }
                    }
                }
            }
            _ => self.rebuild_children(plan, |c| self.push_down_predicates(c)),
        }
    }

    fn prune_columns(&self, plan: PlanNode) -> PlanNode {
        let required = self.collect_required_columns(&plan);
        self.apply_column_pruning(plan, &required)
    }

    fn collect_required_columns(&self, plan: &PlanNode) -> HashSet<String> {
        let mut cols = HashSet::new();
        match &plan.node_type {
            PlanNodeType::Scan(scan) => { for c in &scan.columns { cols.insert(c.clone()); } for p in &scan.predicates { self.extract_column_names(p, &mut cols); } }
            PlanNodeType::Filter(f) => self.extract_column_names(&f.predicate, &mut cols),
            PlanNodeType::Project(p) => { for e in &p.exprs { self.extract_column_names(e, &mut cols); } }
            PlanNodeType::Aggregate(a) => { for g in &a.group_by { cols.insert(g.clone()); } for agg in &a.aggregates { if agg.arg != "*" { cols.insert(agg.arg.clone()); } } }
            PlanNodeType::Sort(s) => { for item in &s.order_by { self.extract_column_names(&item.expr, &mut cols); } }
            PlanNodeType::Join(j) => { if let Some(cond) = &j.condition { self.extract_column_names(cond, &mut cols); } }
            _ => {}
        }
        for child in &plan.children { cols.extend(self.collect_required_columns(child)); }
        cols
    }

    fn extract_column_names(&self, expr_str: &str, cols: &mut HashSet<String>) {
        for token in expr_str.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.');
            if cleaned.contains('.') {
                if let Some(pos) = cleaned.rfind('.') {
                    let col = cleaned[pos + 1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !col.is_empty() && col != "*" { cols.insert(col.to_string()); }
                }
            } else if !cleaned.is_empty() && cleaned != "*" && !["AND","OR","NOT","IS","NULL","IN","BETWEEN","LIKE","ASC","DESC","TRUE","FALSE"].contains(&cleaned) {
                if cleaned.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) && !cleaned.contains('(') {
                    cols.insert(cleaned.to_string());
                }
            }
        }
    }

    fn apply_column_pruning(&self, plan: PlanNode, required: &HashSet<String>) -> PlanNode {
        let children: Vec<PlanNode> = plan.children.into_iter().map(|c| self.apply_column_pruning(c, required)).collect();
        let node_type = match plan.node_type {
            PlanNodeType::Scan(mut scan) => {
                if !scan.columns.is_empty() {
                    let pruned: Vec<String> = scan.columns.into_iter().filter(|c| required.contains(c)).collect();
                    scan.columns = if pruned.is_empty() { vec!["*".to_string()] } else { pruned };
                }
                PlanNodeType::Scan(scan)
            }
            other => other,
        };
        PlanNode { id: plan.id, node_type, children, stats: plan.stats }
    }

    fn push_down_limit(&self, plan: PlanNode) -> PlanNode {
        let id = plan.id.clone();
        let stats = plan.stats.clone();
        match plan.node_type {
            PlanNodeType::Limit(limit_node) => {
                if plan.children.len() != 1 { return self.rebuild_children(PlanNode { id, node_type: PlanNodeType::Limit(limit_node), children: plan.children, stats }, |c| self.push_down_limit(c)); }
                let child = plan.children.into_iter().next().unwrap();
                match child.node_type.clone() {
                    PlanNodeType::Sort(sort) => {
                        if child.children.len() != 1 {
                            return PlanNode { id, node_type: PlanNodeType::Limit(limit_node), children: vec![self.push_down_limit(child)], stats };
                        }
                        let sort_child = child.children.into_iter().next().unwrap();
                        let pushed = PlanNode { id: child.id, node_type: PlanNodeType::Limit(LimitNode { limit: limit_node.limit + limit_node.offset, offset: 0 }), children: vec![self.push_down_limit(sort_child)], stats: child.stats };
                        PlanNode { id, node_type: PlanNodeType::Sort(sort), children: vec![pushed], stats }
                    }
                    PlanNodeType::Project(proj) => {
                        if child.children.len() != 1 {
                            return PlanNode { id, node_type: PlanNodeType::Limit(limit_node), children: vec![self.push_down_limit(child)], stats };
                        }
                        let proj_child = child.children.into_iter().next().unwrap();
                        let pushed = PlanNode { id: child.id, node_type: PlanNodeType::Limit(limit_node), children: vec![self.push_down_limit(proj_child)], stats: child.stats };
                        PlanNode { id, node_type: PlanNodeType::Project(proj), children: vec![pushed], stats }
                    }
                    PlanNodeType::Scan(mut scan) => {
                        scan.limit = Some(limit_node.limit + limit_node.offset);
                        PlanNode { id: child.id, node_type: PlanNodeType::Scan(scan), children: vec![], stats: child.stats }
                    }
                    _ => PlanNode { id, node_type: PlanNodeType::Limit(limit_node), children: vec![self.push_down_limit(child)], stats }
                }
            }
            _ => self.rebuild_children(plan, |c| self.push_down_limit(c)),
        }
    }

    fn reorder_joins(&self, plan: PlanNode) -> PlanNode {
        let children: Vec<PlanNode> = plan.children.into_iter().map(|c| self.reorder_joins(c)).collect();
        if let PlanNodeType::Join(join) = &plan.node_type {
            if children.len() == 2 && matches!(join.join_type, JoinTypePlan::Inner) {
                let left_rows = self.estimate_rows(&children[0]);
                let right_rows = self.estimate_rows(&children[1]);
                if right_rows < left_rows {
                    let mut reordered = children.clone();
                    reordered.reverse();
                    return PlanNode { id: plan.id, node_type: PlanNodeType::Join(join.clone()), children: reordered, stats: plan.stats };
                }
            }
        }
        PlanNode { id: plan.id, node_type: plan.node_type, children, stats: plan.stats }
    }

    fn rebuild_children(&self, plan: PlanNode, f: impl Fn(PlanNode) -> PlanNode) -> PlanNode {
        let children: Vec<PlanNode> = plan.children.into_iter().map(f).collect();
        PlanNode { id: plan.id, node_type: plan.node_type, children, stats: plan.stats }
    }

    fn estimate_rows(&self, plan: &PlanNode) -> f64 {
        if plan.stats.row_count > 0.0 { return plan.stats.row_count; }
        match &plan.node_type {
            PlanNodeType::Scan(scan) => {
                if let Some(ref provider) = self.stats_provider {
                    let db = scan.database.as_deref().unwrap_or("");
                    if let Some(stats) = provider.get_table_stats(db, &scan.table_name) { return stats.row_count as f64; }
                }
                1000.0
            }
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

impl Default for Optimizer { fn default() -> Self { Self::new() } }
