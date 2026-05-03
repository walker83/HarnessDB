use fe_catalog::CatalogManager;
use fe_sql_parser::ast::QueryStmt;

use crate::plan_node::PlanNode;

/// Materialized view definition
#[derive(Debug, Clone)]
pub struct MaterializedView {
    pub id: u64,
    pub name: String,
    pub database: String,
    /// The SQL query that defines the MV
    pub definition: String,
    /// The logical plan of the definition
    pub plan: PlanNode,
    /// Refresh strategy
    pub refresh: RefreshStrategy,
    /// Base tables this MV depends on
    pub base_tables: Vec<(String, String)>, // (database, table)
}

#[derive(Debug, Clone)]
pub enum RefreshStrategy {
    /// Refresh after each INSERT (synchronous)
    Immediate,
    /// Refresh on a schedule (cron-like)
    Scheduled(String),
    /// Manual refresh only
    Manual,
}

/// Rewrite a query to use a materialized view if applicable.
/// Returns the rewritten plan node if a matching MV is found.
pub fn rewrite_query(
    _query: &QueryStmt,
    _available_mvs: &[MaterializedView],
    _catalog: &CatalogManager,
) -> Option<PlanNode> {
    // TODO: Implement MV rewriting:
    // 1. Match query predicates against MV base table predicates
    // 2. Check if SELECT columns are covered by MV
    // 3. If GROUP BY is compatible, use MV directly
    // 4. Return rewritten plan or None
    None
}