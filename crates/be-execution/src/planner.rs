//! Bridge from logical PlanNode to physical ExecutionPlan.
//!
//! This module converts a tree of PlanNodes (from fe-sql-planner) into
//! a tree of ExecNodes (from be-execution) that can be executed.
//!
//! # Limitations
//! The conversion between planner-level expressions (strings) and execution-level
//! column indices requires additional schema resolution that's not yet implemented.

use std::sync::Arc;
use be_storage::StorageEngine;
use fe_sql_planner::plan_node::{PlanNode, PlanNodeType, ScanNode, FilterNode, ProjectNode, AggregateNode, SortNode, LimitNode, JoinNode};
use fe_catalog::CatalogManager;
use types::Schema;

use crate::exec_node::{
    ExecutionPlan, ExecNode, ScanExecNode, FilterExecNode, ProjectExecNode,
    AggregateExecNode, SortExecNode, LimitExecNode, HashJoinExecNode,
};

/// Error type for plan execution conversion.
#[derive(Debug, thiserror::Error)]
pub enum PlanExecutionError {
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Database not found: {0}")]
    DatabaseNotFound(String),
    #[error("Unsupported node type: {0}")]
    UnsupportedNode(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Not yet implemented: {0}")]
    NotYetImplemented(String),
}

/// Context needed to convert PlanNodes to ExecNodes.
pub struct ExecutionContext {
    /// The storage engine for reading data.
    pub storage: Arc<StorageEngine>,
    /// The catalog for resolving table names to tablet IDs.
    pub catalog: Arc<CatalogManager>,
}

impl ExecutionContext {
    pub fn new(storage: Arc<StorageEngine>, catalog: Arc<CatalogManager>) -> Self {
        Self { storage, catalog }
    }

    /// Resolve a table name to a tablet ID.
    /// Returns the table's ID which is used as the tablet_id in storage.
    pub fn resolve_tablet_id(&self, database: &str, table_name: &str) -> Result<u64, PlanExecutionError> {
        let table = self.catalog
            .get_table(database, table_name)
            .ok_or_else(|| PlanExecutionError::TableNotFound(format!("{}.{}", database, table_name)))?;
        Ok(table.id)
    }

    /// Convert a PlanNode tree into an ExecutionPlan tree.
    pub fn create_exec_plan(&self, plan: &PlanNode) -> Result<ExecutionPlan, PlanExecutionError> {
        self.create_exec_plan_impl(plan)
    }

    fn create_exec_plan_impl(&self, plan: &PlanNode) -> Result<ExecutionPlan, PlanExecutionError> {
        match &plan.node_type {
            PlanNodeType::Scan(scan) => {
                self.create_scan_node(scan)
            }
            PlanNodeType::Filter(filter) => {
                if plan.children.is_empty() {
                    return Err(PlanExecutionError::UnsupportedNode("Filter without child".into()));
                }
                let child = self.create_exec_plan_impl(&plan.children[0])?;
                Ok(ExecutionPlan::Filter(FilterExecNode {
                    predicate: filter.predicate.clone(),
                    child: Box::new(child),
                    opened: false,
                }))
            }
            PlanNodeType::Project(project) => {
                if plan.children.is_empty() {
                    return Err(PlanExecutionError::UnsupportedNode("Project without child".into()));
                }
                let child = self.create_exec_plan_impl(&plan.children[0])?;
                Ok(ExecutionPlan::Project(ProjectExecNode {
                    exprs: project.exprs.clone(),
                    child: Box::new(child),
                    opened: false,
                }))
            }
            PlanNodeType::Aggregate(agg) => {
                if plan.children.is_empty() {
                    return Err(PlanExecutionError::UnsupportedNode("Aggregate without child".into()));
                }
                let child = self.create_exec_plan_impl(&plan.children[0])?;
                // Note: AggregateExecNode uses column indices, but AggregateNode uses string expressions.
                // For now, use empty group_by/aggregates which means just pass through all columns.
                Ok(ExecutionPlan::Aggregate(AggregateExecNode {
                    group_by: vec![], // TODO: resolve from agg.group_by strings
                    aggregates: vec![], // TODO: resolve from agg.aggregates
                    child: Box::new(child),
                    opened: false,
                    returned: false,
                }))
            }
            PlanNodeType::Sort(sort) => {
                if plan.children.is_empty() {
                    return Err(PlanExecutionError::UnsupportedNode("Sort without child".into()));
                }
                let child = self.create_exec_plan_impl(&plan.children[0])?;
                // Note: SortExecNode uses column indices, but SortNode uses SortItem expressions.
                // For now, use empty order_by which means no sorting.
                Ok(ExecutionPlan::Sort(SortExecNode {
                    order_by: vec![], // TODO: resolve from sort.order_by expressions
                    child: Box::new(child),
                    opened: false,
                    buffered: Vec::new(),
                    returned: false,
                }))
            }
            PlanNodeType::Limit(limit) => {
                if plan.children.is_empty() {
                    return Err(PlanExecutionError::UnsupportedNode("Limit without child".into()));
                }
                let child = self.create_exec_plan_impl(&plan.children[0])?;
                Ok(ExecutionPlan::Limit(LimitExecNode::new(
                    limit.limit,
                    Box::new(child),
                )))
            }
            PlanNodeType::Join(join) => {
                // For hash join, we need two children
                if plan.children.len() >= 2 {
                    let probe_child = self.create_exec_plan_impl(&plan.children[0])?;
                    let build_child = self.create_exec_plan_impl(&plan.children[1])?;
                    // Note: HashJoinExecNode requires column indices for keys.
                    // For now, use empty keys which means cross join behavior.
                    Ok(ExecutionPlan::HashJoin(HashJoinExecNode::new(
                        format!("{:?}", join.join_type),
                        vec![], // TODO: resolve from join.condition
                        vec![], // TODO: resolve from join.condition
                        Box::new(build_child),
                        Box::new(probe_child),
                        Schema::new(vec![]), // TODO: proper schema
                        Schema::new(vec![]),
                    )))
                } else {
                    Err(PlanExecutionError::UnsupportedNode("Join with < 2 children".into()))
                }
            }
            // DDL and other node types - return a no-op scan that returns empty
            _ => {
                tracing::debug!("Unsupported node type for execution: {:?}", plan.node_type);
                Err(PlanExecutionError::UnsupportedNode(format!("{:?}", plan.node_type)))
            }
        }
    }

    fn create_scan_node(&self, scan: &ScanNode) -> Result<ExecutionPlan, PlanExecutionError> {
        let database = scan.database.as_deref().unwrap_or("default");
        let table_name = &scan.table_name;

        // Try to resolve tablet_id from catalog
        let tablet_id = match self.resolve_tablet_id(database, table_name) {
            Ok(id) => {
                tracing::debug!("Resolved {}.{} to tablet_id={}", database, table_name, id);
                Some(id)
            }
            Err(e) => {
                tracing::debug!("Could not resolve tablet_id for {}.{}: {}", database, table_name, e);
                None
            }
        };

        let mut exec_node = ScanExecNode::new(table_name.clone(), scan.columns.clone());

        if let Some(tid) = tablet_id {
            exec_node = exec_node.with_storage(tid, self.storage.clone());
        }

        exec_node = exec_node.with_predicates(scan.predicates.clone());

        if let Some(limit) = scan.limit {
            exec_node = exec_node.with_limit(limit);
        }

        Ok(ExecutionPlan::Scan(exec_node))
    }
}

/// Execute a plan and return the resulting blocks.
pub async fn execute_plan(plan: &PlanNode, context: &ExecutionContext) -> Result<Vec<types::Block>, PlanExecutionError> {
    // First try to create the execution plan
    let exec_plan = match context.create_exec_plan(plan) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Failed to create exec plan: {}", e);
            return Err(e);
        }
    };

    let mut exec_plan = exec_plan;

    // Open the plan
    if let Err(e) = exec_plan.open().await {
        return Err(PlanExecutionError::StorageError(e.to_string()));
    }

    // Collect all output blocks
    let mut results = Vec::new();
    loop {
        match exec_plan.get_next().await {
            Ok(Some(block)) => {
                if !block.is_empty() {
                    results.push(block);
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("Error during execution: {}", e);
                break;
            }
        }
    }

    // Close the plan
    let _ = exec_plan.close().await;

    Ok(results)
}
