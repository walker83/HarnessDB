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
    SemiJoin(SemiJoinNode),
    AntiSemiJoin(AntiSemiJoinNode),
    HashJoin(HashJoinNode),
    MergeJoin(MergeJoinNode),
    Exchange(ExchangeNode),
    Union(UnionNode),
    Cte(CteNode),
    Insert(InsertNode),
    Update(UpdateNode),
    Delete(DeleteNode),
    CreateTable(CreateTableNode),
    CreateDatabase(CreateDatabaseNode),
    CreateView(CreateViewNode),
    DropTable(DropTableNode),
    DropDatabase(DropDatabaseNode),
    TruncateTable(TruncateTableNode),
    ShowCreateTable(ShowCreateTableNode),
    AlterTable(AlterTableNode),
    Values(VirtualValuesNode),
    CreateRepository(CreateRepositoryNode),
    DropRepository(DropRepositoryNode),
    ShowRepositories(ShowRepositoriesNode),
    BackupDatabase(BackupDatabaseNode),
    RestoreDatabase(RestoreDatabaseNode),
    CreateMaterializedView(CreateMaterializedViewNode),
    DropMaterializedView(DropMaterializedViewNode),
    AlterMaterializedView(AlterMaterializedViewNode),
    RefreshMaterializedView(RefreshMaterializedViewNode),
    DdlCommand(DdlCommandNode),
    AnalyzeStats(AnalyzeStatsNode),
}

// ---- Leaf / scan nodes ----

#[derive(Debug, Clone)]
pub struct ScanNode {
    pub catalog: Option<String>,
    pub table_name: String,
    pub database: Option<String>,
    pub columns: Vec<String>,
    pub predicates: Vec<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct InsertNode {
    pub table_name: String,
    pub database: Option<String>,
    pub columns: Vec<String>,
    pub is_overwrite: bool,
}

/// A node that produces rows from VALUES clause.
#[derive(Debug, Clone)]
pub struct VirtualValuesNode {
    pub rows: Vec<Vec<fe_sql_parser::ast::Expr>>,
}

#[derive(Debug, Clone)]
pub struct UpdateNode {
    pub table_name: String,
    pub database: Option<String>,
    pub set_clauses: Vec<SetClausePlan>,
    pub selection_predicate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetClausePlan {
    pub column: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct DeleteNode {
    pub table_name: String,
    pub database: Option<String>,
    pub selection_predicate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AlterTableNode {
    pub database: Option<String>,
    pub table_name: String,
    pub operations: Vec<AlterOperationPlan>,
}

#[derive(Debug, Clone)]
pub enum AlterOperationPlan {
    AddColumn { name: String, data_type: String, nullable: bool },
    DropColumn { name: String },
    ModifyColumn { name: String, data_type: String },
    RenameTable { new_name: String },
    RenameColumn { old_name: String, new_name: String },
    SetComment { comment: String },
    SetProperty { properties: Vec<(String, String)> },
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

#[derive(Debug, Clone)]
pub struct TruncateTableNode {
    pub database: Option<String>,
    pub table_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CteNode {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShowCreateTableNode {
    pub database: String,
    pub table_name: String,
}

#[derive(Debug, Clone)]
pub struct CreateViewNode {
    pub database: Option<String>,
    pub view_name: String,
    pub if_not_exists: bool,
    pub query: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateRepositoryNode {
    pub name: String,
    pub repo_type: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct DropRepositoryNode {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct ShowRepositoriesNode;

#[derive(Debug, Clone)]
pub struct BackupDatabaseNode {
    pub database: String,
    pub repository: String,
    pub backup_name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct RestoreDatabaseNode {
    pub database: String,
    pub repository: String,
    pub backup_name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CreateMaterializedViewNode {
    pub database: Option<String>,
    pub view_name: String,
    pub if_not_exists: bool,
    pub query: String,
    pub columns: Vec<String>,
    pub refresh_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DropMaterializedViewNode {
    pub database: Option<String>,
    pub view_name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterMaterializedViewNode {
    pub database: Option<String>,
    pub view_name: String,
    pub operation: String,
}

#[derive(Debug, Clone)]
pub struct RefreshMaterializedViewNode {
    pub database: Option<String>,
    pub view_name: String,
    pub refresh_type: String,
}

#[derive(Debug, Clone)]
pub struct DdlCommandNode {
    pub command: String,
}

// ---- Statistics / ANALYZE ----

/// A node that triggers statistics collection (ANALYZE TABLE).
/// The executor reads table data and computes column statistics.
#[derive(Debug, Clone)]
pub struct AnalyzeStatsNode {
    pub database: Option<String>,
    pub table_name: String,
    pub columns: Vec<String>,
    pub sample_rate: Option<f64>,
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
    pub build_filters: Vec<RuntimeFilterPlan>,
    pub probe_filters: Vec<RuntimeFilterPlan>,
}

#[derive(Debug, Clone)]
pub struct RuntimeFilterPlan {
    pub id: u64,
    pub filter_type: RuntimeFilterTypePlan,
    pub build_column: String,
    pub probe_column: String,
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeFilterTypePlan {
    Bloom,
    MinMax,
    In,
}

/// Merge join: expects both inputs sorted on the join keys.
#[derive(Debug, Clone)]
pub struct MergeJoinNode {
    pub join_type: JoinTypePlan,
    pub left_keys: Vec<String>,
    pub right_keys: Vec<String>,
    pub condition: Option<String>,
}

/// Semi join: returns rows from the left side that have a match in the right side.
/// Used for EXISTS and IN subqueries.
#[derive(Debug, Clone)]
pub struct SemiJoinNode {
    pub left_key: String,
    pub right_key: String,
    pub condition: Option<String>,
}

/// Anti semi join: returns rows from the left side that do NOT have a match in the right side.
/// Used for NOT EXISTS and NOT IN subqueries.
#[derive(Debug, Clone)]
pub struct AntiSemiJoinNode {
    pub left_key: String,
    pub right_key: String,
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
            PlanNodeType::SemiJoin(sj) => {
                let cond = sj
                    .condition
                    .as_deref()
                    .map(|c| format!(" filter={}", c))
                    .unwrap_or_default();
                format!(
                    "SemiJoinNode: left=[{}] right=[{}]{}",
                    sj.left_key, sj.right_key, cond
                )
            }
            PlanNodeType::AntiSemiJoin(asj) => {
                let cond = asj
                    .condition
                    .as_deref()
                    .map(|c| format!(" filter={}", c))
                    .unwrap_or_default();
                format!(
                    "AntiSemiJoinNode: left=[{}] right=[{}]{}",
                    asj.left_key, asj.right_key, cond
                )
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
            PlanNodeType::Cte(cte) => {
                format!("CteNode: {}({})", cte.name, cte.columns.join(", "))
            }
            PlanNodeType::TruncateTable(tt) => {
                let db_prefix = tt
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                let ife = if tt.if_exists { " IF EXISTS" } else { "" };
                format!("TruncateTableNode:{} {}{}", ife, db_prefix, tt.table_name)
            }
            PlanNodeType::ShowCreateTable(_) => "ShowCreateTableNode".to_string(),
            PlanNodeType::CreateView(_) => "CreateViewNode".to_string(),
            PlanNodeType::Update(_) => "UpdateNode".to_string(),
            PlanNodeType::Delete(_) => "DeleteNode".to_string(),
            PlanNodeType::AlterTable(_) => "AlterTableNode".to_string(),
            PlanNodeType::Values(vals) => {
                format!("VirtualValuesNode: {} rows", vals.rows.len())
            }
            PlanNodeType::CreateRepository(repo) => {
                format!("CreateRepositoryNode: {} (type={})", repo.name, repo.repo_type)
            }
            PlanNodeType::DropRepository(repo) => {
                format!("DropRepositoryNode: {} (if_exists={})", repo.name, repo.if_exists)
            }
            PlanNodeType::ShowRepositories(_) => "ShowRepositoriesNode".to_string(),
            PlanNodeType::BackupDatabase(backup) => {
                format!("BackupDatabaseNode: {} TO {} (name={})", backup.database, backup.repository, backup.backup_name)
            }
            PlanNodeType::RestoreDatabase(restore) => {
                format!("RestoreDatabaseNode: {} FROM {} (name={})", restore.database, restore.repository, restore.backup_name)
            }
            PlanNodeType::CreateMaterializedView(mv) => {
                format!("CreateMaterializedViewNode: {}{}", mv.database.as_ref().map(|d| format!("{}.", d)).unwrap_or_default(), mv.view_name)
            }
            PlanNodeType::DropMaterializedView(mv) => {
                format!("DropMaterializedViewNode: {}{}", mv.database.as_ref().map(|d| format!("{}.", d)).unwrap_or_default(), mv.view_name)
            }
            PlanNodeType::AlterMaterializedView(mv) => {
                format!("AlterMaterializedViewNode: {}{}", mv.database.as_ref().map(|d| format!("{}.", d)).unwrap_or_default(), mv.view_name)
            }
            PlanNodeType::RefreshMaterializedView(mv) => {
                format!("RefreshMaterializedViewNode: {}{}", mv.database.as_ref().map(|d| format!("{}.", d)).unwrap_or_default(), mv.view_name)
            }
            PlanNodeType::DdlCommand(cmd) => format!("DdlCommand({})", cmd.command),
            PlanNodeType::AnalyzeStats(analyze) => {
                let db_prefix = analyze
                    .database
                    .as_ref()
                    .map(|d| format!("{}.", d))
                    .unwrap_or_default();
                let cols = if analyze.columns.is_empty() {
                    "ALL".to_string()
                } else {
                    analyze.columns.join(", ")
                };
                let sample = analyze
                    .sample_rate
                    .map(|r| format!(" (sample={})", r))
                    .unwrap_or_default();
                format!("AnalyzeStatsNode: {}{} [{}]{}", db_prefix, analyze.table_name, cols, sample)
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
            | PlanNodeType::SemiJoin(_)
            | PlanNodeType::AntiSemiJoin(_)
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
            PlanNodeType::Cte(cte) => cte.columns.clone(),
            // DDL / DML nodes don't produce query columns.
            PlanNodeType::Insert(_)
            | PlanNodeType::Update(_)
            | PlanNodeType::Delete(_)
            | PlanNodeType::AlterTable(_)
            | PlanNodeType::CreateTable(_)
            | PlanNodeType::CreateDatabase(_)
            | PlanNodeType::CreateView(_)
            | PlanNodeType::DropTable(_)
            | PlanNodeType::DropDatabase(_)
            | PlanNodeType::TruncateTable(_)
            | PlanNodeType::ShowCreateTable(_)
            | PlanNodeType::Values(_)
            | PlanNodeType::CreateRepository(_)
            | PlanNodeType::DropRepository(_)
            | PlanNodeType::ShowRepositories(_)
            | PlanNodeType::BackupDatabase(_)
            | PlanNodeType::RestoreDatabase(_)
            | PlanNodeType::CreateMaterializedView(_)
            | PlanNodeType::DropMaterializedView(_)
            | PlanNodeType::AlterMaterializedView(_)
            | PlanNodeType::RefreshMaterializedView(_)
            | PlanNodeType::DdlCommand(_)
            | PlanNodeType::AnalyzeStats(_) => vec![],
        }
    }
}
