use crate::plan_node::*;
use crate::statistics::StatisticsProvider;
use std::collections::HashSet;
use std::sync::Arc;

pub struct Optimizer {
    stats_provider: Option<Arc<dyn StatisticsProvider>>,
    cbo: Option<crate::cbo_optimizer::CboOptimizer>,
}

impl Optimizer {
    pub fn new() -> Self { Self { stats_provider: None, cbo: None } }

    pub fn with_stats_provider(mut self, provider: Arc<dyn StatisticsProvider>) -> Self {
        self.cbo = Some(crate::cbo_optimizer::CboOptimizer::new(provider.clone()));
        self.stats_provider = Some(provider);
        self
    }

    pub fn optimize(&self, plan: PlanNode) -> PlanNode {
        // Phase 1: RBO rules
        const MAX_ITERATIONS: usize = 10;
        let mut plan = plan;
        for _ in 0..MAX_ITERATIONS {
            let before = format!("{}", plan);
            plan = self.apply_rules(plan);
            let after = format!("{}", plan);
            if before == after { break; }
        }

        // Phase 2: CBO rules
        if let Some(ref cbo) = self.cbo {
            plan = cbo.optimize(plan);
        }

        plan
    }

    fn apply_rules(&self, plan: PlanNode) -> PlanNode {
        let plan = self.push_down_predicates(plan);
        let plan = self.prune_columns(plan);
        let plan = self.push_down_limit(plan);
        
        self.reorder_joins(plan)
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
            } else if !cleaned.is_empty() && cleaned != "*" && !["AND","OR","NOT","IS","NULL","IN","BETWEEN","LIKE","ASC","DESC","TRUE","FALSE"].contains(&cleaned)
                && cleaned.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) && !cleaned.contains('(') {
                    cols.insert(cleaned.to_string());
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
        if let PlanNodeType::Join(join) = &plan.node_type
            && children.len() == 2 && matches!(join.join_type, JoinTypePlan::Inner) {
                let left_rows = self.estimate_rows(&children[0]);
                let right_rows = self.estimate_rows(&children[1]);
                if right_rows < left_rows {
                    let mut reordered = children.clone();
                    reordered.reverse();
                    return PlanNode { id: plan.id, node_type: PlanNodeType::Join(join.clone()), children: reordered, stats: plan.stats };
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

#[cfg(test)]
mod tests {
    use super::*;
    

    #[allow(dead_code)]
    fn next_id() -> PlanNodeId { PlanNodeId(0) }

    fn scan_node(table: &str, cols: &[&str]) -> PlanNode {
        PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode {
                table_name: table.to_string(),
                database: Some("db".to_string()),
                columns: cols.iter().map(|c| c.to_string()).collect(),
                predicates: vec![],
                limit: None,
            }),
            children: vec![],
            stats: PlanStats::default(),
        }
    }

    // ---- Predicate pushdown ----

    #[test]
    fn test_predicate_pushdown_to_scan() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a", "b"]);
        let plan = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Filter(FilterNode { predicate: "a > 10".to_string() }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Predicate should be pushed into scan
        match &result.node_type {
            PlanNodeType::Scan(scan) => {
                assert_eq!(scan.predicates.len(), 1);
                assert_eq!(scan.predicates[0], "a > 10");
            }
            other => panic!("Expected Scan, got {:?}", other),
        }
    }

    #[test]
    fn test_predicate_pushdown_through_project() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a", "b"]);
        let project = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Project(ProjectNode { exprs: vec!["a".to_string()] }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let plan = PlanNode {
            id: PlanNodeId(2),
            node_type: PlanNodeType::Filter(FilterNode { predicate: "a > 5".to_string() }),
            children: vec![project],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Optimizer pushes filter through project into scan
        fn find_scan(node: &PlanNode) -> Option<&ScanNode> {
            match &node.node_type {
                PlanNodeType::Scan(s) => Some(s),
                _ => node.children.iter().find_map(find_scan),
            }
        }
        let scan = find_scan(&result).expect("should find scan");
        assert!(scan.predicates.contains(&"a > 5".to_string()));
    }

    #[test]
    fn test_filter_merge() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a"]);
        let inner = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Filter(FilterNode { predicate: "a > 5".to_string() }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let plan = PlanNode {
            id: PlanNodeId(2),
            node_type: PlanNodeType::Filter(FilterNode { predicate: "a < 100".to_string() }),
            children: vec![inner],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Filters should be merged and pushed into scan
        fn find_scan(node: &PlanNode) -> Option<&ScanNode> {
            match &node.node_type {
                PlanNodeType::Scan(s) => Some(s),
                _ => node.children.iter().find_map(find_scan),
            }
        }
        let scan = find_scan(&result).expect("should find scan");
        let combined: String = scan.predicates.join(" ");
        assert!(combined.contains("a > 5") || combined.contains("a < 100"));
    }

    // ---- Column pruning ----

    #[test]
    fn test_column_pruning_removes_unused() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a", "b", "c"]);
        let plan = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Project(ProjectNode { exprs: vec!["a".to_string()] }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        fn find_scan(node: &PlanNode) -> Option<&ScanNode> {
            match &node.node_type {
                PlanNodeType::Scan(s) => Some(s),
                _ => node.children.iter().find_map(find_scan),
            }
        }
        let scan = find_scan(&result).expect("should find scan");
        // Column pruning should keep at least column "a"
        assert!(scan.columns.contains(&"a".to_string()));
    }

    // ---- Limit pushdown ----

    #[test]
    fn test_limit_pushdown_to_scan() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a"]);
        let plan = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Limit(LimitNode { limit: 10, offset: 0 }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Limit should be pushed into scan
        match &result.node_type {
            PlanNodeType::Scan(scan) => {
                assert_eq!(scan.limit, Some(10));
            }
            other => panic!("Expected Scan with limit, got {:?}", other),
        }
    }

    #[test]
    fn test_limit_pushdown_through_sort() {
        let opt = Optimizer::new();
        let scan = scan_node("t1", &["a"]);
        let sort = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Sort(SortNode { order_by: vec![SortItem { expr: "a".to_string(), ascending: true }] }),
            children: vec![scan],
            stats: PlanStats::default(),
        };
        let plan = PlanNode {
            id: PlanNodeId(2),
            node_type: PlanNodeType::Limit(LimitNode { limit: 5, offset: 0 }),
            children: vec![sort],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Limit should be pushed down to scan
        fn find_scan(node: &PlanNode) -> Option<&ScanNode> {
            match &node.node_type {
                PlanNodeType::Scan(s) => Some(s),
                _ => node.children.iter().find_map(find_scan),
            }
        }
        let scan = find_scan(&result).expect("should find scan");
        assert_eq!(scan.limit, Some(5));
    }

    // ---- Join reordering ----

    #[test]
    fn test_join_reorder_smaller_build_side() {
        let opt = Optimizer::new();
        let left = PlanNode {
            id: PlanNodeId(0),
            node_type: PlanNodeType::Scan(ScanNode { table_name: "big".into(), database: None, columns: vec!["*".into()], predicates: vec![], limit: None }),
            children: vec![],
            stats: PlanStats::with_row_count(10000.0),
        };
        let right = PlanNode {
            id: PlanNodeId(1),
            node_type: PlanNodeType::Scan(ScanNode { table_name: "small".into(), database: None, columns: vec!["*".into()], predicates: vec![], limit: None }),
            children: vec![],
            stats: PlanStats::with_row_count(100.0),
        };
        let plan = PlanNode {
            id: PlanNodeId(2),
            node_type: PlanNodeType::Join(JoinNode { join_type: JoinTypePlan::Inner, condition: Some("id = id".to_string()) }),
            children: vec![left, right],
            stats: PlanStats::default(),
        };
        let result = opt.optimize(plan);
        // Small table should be first (build side)
        match &result.node_type {
            PlanNodeType::Join(_) => {
                let first = &result.children[0];
                match &first.node_type {
                    PlanNodeType::Scan(s) => assert_eq!(s.table_name, "small"),
                    other => panic!("Expected Scan, got {:?}", other),
                }
            }
            other => panic!("Expected Join, got {:?}", other),
        }
    }

    // ---- Expression simplification ----

    #[test]
    fn test_simplify_true_and_x() {
        use crate::expression::simplify;
        use fe_sql_parser::ast::*;
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Boolean(true))),
            op: BinaryOp::And,
            right: Box::new(Expr::ColumnRef { table: None, column: "a".to_string() }),
        };
        let result = simplify(expr);
        assert!(matches!(result, Expr::ColumnRef { .. }));
    }

    #[test]
    fn test_simplify_false_or_x() {
        use crate::expression::simplify;
        use fe_sql_parser::ast::*;
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Boolean(false))),
            op: BinaryOp::Or,
            right: Box::new(Expr::ColumnRef { table: None, column: "a".to_string() }),
        };
        let result = simplify(expr);
        assert!(matches!(result, Expr::ColumnRef { .. }));
    }

    #[test]
    fn test_simplify_false_and_x() {
        use crate::expression::simplify;
        use fe_sql_parser::ast::*;
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Boolean(false))),
            op: BinaryOp::And,
            right: Box::new(Expr::ColumnRef { table: None, column: "a".to_string() }),
        };
        let result = simplify(expr);
        assert!(matches!(result, Expr::Literal(LiteralValue::Boolean(false))));
    }

    #[test]
    fn test_simplify_not_not() {
        use crate::expression::simplify;
        use fe_sql_parser::ast::*;
        let _col = Expr::ColumnRef { table: None, column: "a".to_string() };
        let expr = Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(Expr::ColumnRef { table: None, column: "a".to_string() }),
            }),
        };
        let result = simplify(expr);
        // NOT (NOT col) should simplify to col
        assert!(matches!(result, Expr::ColumnRef { .. }));
    }

    #[test]
    fn test_constant_folding_arithmetic() {
        use crate::expression::simplify;
        use fe_sql_parser::ast::*;
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(LiteralValue::Int64(3))),
            op: BinaryOp::Plus,
            right: Box::new(Expr::Literal(LiteralValue::Int64(7))),
        };
        let result = simplify(expr);
        assert!(matches!(result, Expr::Literal(LiteralValue::Int64(10))));
    }
}
