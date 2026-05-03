use std::fmt;

/// Unique identifier for a plan node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlanNodeId(pub usize);

/// The top-level plan node. Every node in the plan tree wraps a PlanNodeType
/// and carries child nodes plus estimated statistics.
#[derive(Debug, Clone)]
pub struct PlanNode {
    pub id: PlanNodeId,
    pub node_type: PlanNodeType,
    pub children: Vec<PlanNode>,
    pub stats: PlanStats,
}

/// All supported node types in the logical / physical plan.
#[derive(Debug, Clone)]
pub enum PlanNodeType {
    Scan(ScanNode),
    Filter(FilterNode),
    Project(ProjectNode),
    Aggregate(AggregateNode),
    Sort(SortNode),
    Limit(LimitNode),
    Join(JoinNode),
    HashJoin(HashJoinNode),
    MergeJoin(MergeJoinNode),
    Exchange(ExchangeNode),
    Union(UnionNode),
    Insert(InsertNode),
    CreateTable(CreateTableNode),
    CreateDatabase(CreateDatabaseNode),
    DropTable(DropTableNode),
    DropDatabase(DropDatabaseNode),
}

// ---- Leaf / scan nodes ----

#[derive(Debug, Clone)]
pub struct ScanNode {
    pub table_name: String,
    pub database: Option<String>,
    /// Columns to project out of the scan (column pruning).
    pub columns: Vec<String>,
    /// Push-down predicates that the scan can evaluate directly.
    pub predicates: Vec<String>,
    /// Limit pushed into the scan (early termination).
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct InsertNode {
    pub table_name: String,
    pub database: Option<String>,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateTableNode {
    pub database: Option<String>,
    pub table_name: String,
    pub if_not_exists: bool,
    pub columns: Vec<ColumnDefPlan>,
    pub keys_type: String,
    pub partition_info: Option<String>,
    pub distribution_info: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ColumnDefPlan {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub agg_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateDatabaseNode {
    pub name: String,
    pub if_not_exists: bool,
}

#[derive(Debug, Clone)]
pub struct DropTableNode {
    pub database: Option<String>,
    pub table_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct DropDatabaseNode {
    pub name: String,
    pub if_exists: bool,
}

// ---- Relational operators ----

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub predicate: String,
}

#[derive(Debug, Clone)]
pub struct ProjectNode {
    pub exprs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AggregateNode {
    pub group_by: Vec<String>,
    pub aggregates: Vec<AggregateExpr>,
}

#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub func: String,
    pub arg: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SortNode {
    pub order_by: Vec<SortItem>,
}

#[derive(Debug, Clone)]
pub struct SortItem {
    pub expr: String,
    pub ascending: bool,
}

#[derive(Debug, Clone)]
pub struct LimitNode {
    pub limit: usize,
    pub offset: usize,
}

// ---- Join operators ----

#[derive(Debug, Clone)]
pub struct JoinNode {
    pub join_type: JoinTypePlan,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinTypePlan {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
}

impl fmt::Display for JoinTypePlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinTypePlan::Inner => write!(f, "INNER"),
            JoinTypePlan::LeftOuter => write!(f, "LEFT OUTER"),
            JoinTypePlan::RightOuter => write!(f, "RIGHT OUTER"),
            JoinTypePlan::FullOuter => write!(f, "FULL OUTER"),
            JoinTypePlan::Cross => write!(f, "CROSS"),
        }
    }
}

/// Hash join: builds a hash table from build_keys on the build (right) side,
/// then probes with probe_keys on the probe (left) side.
#[derive(Debug, Clone)]
pub struct HashJoinNode {
    pub join_type: JoinTypePlan,
    pub build_keys: Vec<String>,
    pub probe_keys: Vec<String>,
    pub condition: Option<String>,
}

/// Merge join: expects both inputs sorted on the join keys.
#[derive(Debug, Clone)]
pub struct MergeJoinNode {
    pub join_type: JoinTypePlan,
    pub left_keys: Vec<String>,
    pub right_keys: Vec<String>,
    pub condition: Option<String>,
}

// ---- Distribution / exchange ----

#[derive(Debug, Clone)]
pub struct ExchangeNode {
    pub exchange_type: ExchangeType,
}

#[derive(Debug, Clone, Copy)]
pub enum ExchangeType {
    HashPartition { num_partitions: usize },
    Broadcast,
    Gather,
    RoundRobin { num_partitions: usize },
}

#[derive(Debug, Clone)]
pub struct UnionNode {
    /// Number of inputs (kept for readability; children vec holds actual nodes).
    pub input_count: usize,
}

// ---- Plan statistics ----

#[derive(Debug, Clone, Default)]
pub struct PlanStats {
    pub row_count: f64,
    pub byte_size: f64,
    pub cardinality: f64,
}

impl PlanStats {
    pub fn with_row_count(row_count: f64) -> Self {
        Self {
            row_count,
            ..Self::default()
        }
    }
}

// ---- Display ----

impl fmt::Display for PlanNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_indent(f, 0)
    }
}

impl PlanNode {
    fn format_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let prefix = "  ".repeat(indent);
        writeln!(f, "{}{}", prefix, self.format_node())?;
        for child in &self.children {
            child.format_indent(f, indent + 1)?;
        }
        Ok(())
    }

    fn format_node(&self) -> String {
        match &self.node_type {
            PlanNodeType::Scan(scan) => {
                let db_prefix = scan
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                let cols = if scan.columns.is_empty() {
                    "*".to_string()
                } else {
                    scan.columns.join(", ")
                };
                let mut s = format!("ScanNode: {}{} [{}]", db_prefix, scan.table_name, cols);
                if !scan.predicates.is_empty() {
                    s.push_str(&format!(
                        ", pushdown=[{}]",
                        scan.predicates.join(" AND ")
                    ));
                }
                if let Some(lim) = scan.limit {
                    s.push_str(&format!(", limit={}", lim));
                }
                s
            }
            PlanNodeType::Filter(filter) => {
                format!("FilterNode: {}", filter.predicate)
            }
            PlanNodeType::Project(proj) => {
                format!("ProjectNode: [{}]", proj.exprs.join(", "))
            }
            PlanNodeType::Aggregate(agg) => {
                let gb = if agg.group_by.is_empty() {
                    "NONE".to_string()
                } else {
                    agg.group_by.join(", ")
                };
                let aggs: Vec<String> = agg
                    .aggregates
                    .iter()
                    .map(|a| {
                        let alias = a
                            .alias
                            .as_ref()
                            .map(|al| format!(" AS {}", al))
                            .unwrap_or_default();
                        format!("{}({}){}", a.func, a.arg, alias)
                    })
                    .collect();
                format!("AggregateNode: group_by=[{}], agg=[{}]", gb, aggs.join(", "))
            }
            PlanNodeType::Sort(sort) => {
                let items: Vec<String> = sort
                    .order_by
                    .iter()
                    .map(|s| {
                        format!(
                            "{} {}",
                            s.expr,
                            if s.ascending { "ASC" } else { "DESC" }
                        )
                    })
                    .collect();
                format!("SortNode: [{}]", items.join(", "))
            }
            PlanNodeType::Limit(limit) => {
                if limit.offset > 0 {
                    format!("LimitNode: {} OFFSET {}", limit.limit, limit.offset)
                } else {
                    format!("LimitNode: {}", limit.limit)
                }
            }
            PlanNodeType::Join(join) => {
                let cond = join
                    .condition
                    .as_deref()
                    .map(|c| format!(" ON {}", c))
                    .unwrap_or_default();
                format!("JoinNode: {}{}", join.join_type, cond)
            }
            PlanNodeType::HashJoin(hj) => {
                let cond = hj
                    .condition
                    .as_deref()
                    .map(|c| format!(" filter={}", c))
                    .unwrap_or_default();
                format!(
                    "HashJoinNode: {} build=[{}] probe=[{}]{}",
                    hj.join_type,
                    hj.build_keys.join(", "),
                    hj.probe_keys.join(", "),
                    cond
                )
            }
            PlanNodeType::MergeJoin(mj) => {
                let cond = mj
                    .condition
                    .as_deref()
                    .map(|c| format!(" filter={}", c))
                    .unwrap_or_default();
                format!(
                    "MergeJoinNode: {} left=[{}] right=[{}]{}",
                    mj.join_type,
                    mj.left_keys.join(", "),
                    mj.right_keys.join(", "),
                    cond
                )
            }
            PlanNodeType::Exchange(ex) => match ex.exchange_type {
                ExchangeType::HashPartition { num_partitions } => {
                    format!("ExchangeNode: HASH(partitions={})", num_partitions)
                }
                ExchangeType::Broadcast => "ExchangeNode: BROADCAST".to_string(),
                ExchangeType::Gather => "ExchangeNode: GATHER".to_string(),
                ExchangeType::RoundRobin { num_partitions } => {
                    format!("ExchangeNode: ROUND_ROBIN(partitions={})", num_partitions)
                }
            },
            PlanNodeType::Insert(ins) => {
                let db_prefix = ins
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                format!("InsertNode: {}{}({})", db_prefix, ins.table_name, ins.columns.join(", "))
            }
            PlanNodeType::CreateTable(ct) => {
                let db_prefix = ct
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                let ifne = if ct.if_not_exists { " IF NOT EXISTS" } else { "" };
                format!(
                    "CreateTableNode:{} {}{} [{}]",
                    ifne,
                    db_prefix,
                    ct.table_name,
                    ct.columns
                        .iter()
                        .map(|c| format!("{} {}", c.name, c.data_type))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            PlanNodeType::CreateDatabase(cd) => {
                let ifne = if cd.if_not_exists { " IF NOT EXISTS" } else { "" };
                format!("CreateDatabaseNode:{} {}", ifne, cd.name)
            }
            PlanNodeType::DropTable(dt) => {
                let db_prefix = dt
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                let ife = if dt.if_exists { " IF EXISTS" } else { "" };
                format!("DropTableNode:{} {}{}", ife, db_prefix, dt.table_name)
            }
            PlanNodeType::DropDatabase(dd) => {
                let ife = if dd.if_exists { " IF EXISTS" } else { "" };
                format!("DropDatabaseNode:{} {}", ife, dd.name)
            }
            PlanNodeType::Union(union) => {
                format!("UnionNode: inputs={}", union.input_count)
            }
        }
    }

    /// Returns the output schema (column names) this node produces.
    /// This is a lightweight approximation used by the optimizer.
    pub fn output_columns(&self) -> Vec<String> {
        match &self.node_type {
            PlanNodeType::Scan(scan) => {
                if scan.columns.is_empty() {
                    vec!["*".to_string()]
                } else {
                    scan.columns.clone()
                }
            }
            PlanNodeType::Project(proj) => proj.exprs.clone(),
            PlanNodeType::Aggregate(agg) => {
                let mut cols: Vec<String> = agg.group_by.clone();
                for a in &agg.aggregates {
                    cols.push(
                        a.alias
                            .clone()
                            .unwrap_or_else(|| format!("{}({})", a.func, a.arg)),
                    );
                }
                cols
            }
            PlanNodeType::Filter(_)
            | PlanNodeType::Sort(_)
            | PlanNodeType::Limit(_)
            | PlanNodeType::Exchange(_) => {
                // Pass-through: return first child's output columns.
                self.children
                    .first()
                    .map(|c| c.output_columns())
                    .unwrap_or_default()
            }
            PlanNodeType::Join(_)
            | PlanNodeType::HashJoin(_)
            | PlanNodeType::MergeJoin(_) => {
                let mut cols = Vec::new();
                for child in &self.children {
                    cols.extend(child.output_columns());
                }
                cols
            }
            PlanNodeType::Union(_) => self
                .children
                .first()
                .map(|c| c.output_columns())
                .unwrap_or_default(),
            // DDL / DML nodes don't produce query columns.
            PlanNodeType::Insert(_)
            | PlanNodeType::CreateTable(_)
            | PlanNodeType::CreateDatabase(_)
            | PlanNodeType::DropTable(_)
            | PlanNodeType::DropDatabase(_) => vec![],
        }
    }
}
