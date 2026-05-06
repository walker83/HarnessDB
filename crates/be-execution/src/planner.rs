//! Bridge from logical PlanNode to physical ExecutionPlan.
//!
//! This module converts a tree of PlanNodes (from fe-sql-planner) into
//! a tree of ExecNodes (from be-execution) that can be executed.
//!
//! # Limitations
//! The conversion between planner-level expressions (strings) and execution-level
//! column indices requires additional schema resolution that's not yet implemented.

use std::sync::{Arc, RwLock as StdRwLock};
use be_storage::StorageEngine;
use fe_sql_planner::plan_node::{PlanNode, PlanNodeType, ScanNode, UpdateNode, DeleteNode, AlterTableNode, InsertNode, VirtualValuesNode};
use fe_catalog::CatalogManager;
use types::Schema;

use crate::exec_node::{
    ExecutionPlan, ExecNode, ScanExecNode, FilterExecNode, ProjectExecNode,
    AggregateExecNode, SortExecNode, LimitExecNode, HashJoinExecNode,
    UpdateExecNode, DeleteExecNode, AlterTableExecNode, InsertExecNode,
    ValuesExecNode, TransactionContext, PendingWrite, WriteOp, PendingDelete,
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
    /// Transaction context for staging DML operations in transactions.
    /// When None, DML executes immediately. When set, DML should be staged
    /// to the transaction context instead of executed immediately.
    pub transaction_ctx: Option<Arc<StdRwLock<TransactionContext>>>,
}

impl ExecutionContext {
    pub fn new(storage: Arc<StorageEngine>, catalog: Arc<CatalogManager>) -> Self {
        Self {
            storage,
            catalog,
            transaction_ctx: None,
        }
    }

    /// Configure transaction context for DML staging.
    pub fn with_transaction_ctx(mut self, tx_ctx: Arc<StdRwLock<TransactionContext>>) -> Self {
        self.transaction_ctx = Some(tx_ctx);
        self
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
            PlanNodeType::Aggregate(_agg) => {
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
            PlanNodeType::Sort(_sort) => {
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
            PlanNodeType::Update(update) => {
                self.create_update_node(update)
            }
            PlanNodeType::Delete(delete) => {
                self.create_delete_node(delete)
            }
            PlanNodeType::AlterTable(alter) => {
                self.create_alter_table_node(alter)
            }
            PlanNodeType::Insert(insert) => {
                self.create_insert_node(insert, &plan.children)
            }
            // DDL and other node types - return a no-op scan that returns empty
            _ => {
                tracing::debug!("Unsupported node type for execution: {:?}", plan.node_type);
                Err(PlanExecutionError::UnsupportedNode(format!("{:?}", plan.node_type)))
            }
        }
    }

    fn create_update_node(&self, update: &UpdateNode) -> Result<ExecutionPlan, PlanExecutionError> {
        let database = update.database.as_deref().unwrap_or("default");
        let tablet_id = self.resolve_tablet_id(database, &update.table_name).ok();
        let set_clauses: Vec<(String, String)> = update.set_clauses.iter()
            .map(|s| (s.column.clone(), s.value.clone()))
            .collect();

        let mut node = UpdateExecNode::new(
            update.table_name.clone(),
            database.to_string(),
            set_clauses,
            update.selection_predicate.clone(),
        );

        if let (Some(tid), Some(storage)) = (tablet_id, Some(self.storage.clone())) {
            node = node.with_storage(tid, storage);
        }

        // Pass transaction context if set
        if let Some(ref tx_ctx) = self.transaction_ctx {
            node = node.with_transaction_ctx(tx_ctx.clone());
        }

        Ok(ExecutionPlan::Update(node))
    }

    fn create_delete_node(&self, delete: &DeleteNode) -> Result<ExecutionPlan, PlanExecutionError> {
        let database = delete.database.as_deref().unwrap_or("default");
        let tablet_id = self.resolve_tablet_id(database, &delete.table_name).ok();

        let mut node = DeleteExecNode::new(
            delete.table_name.clone(),
            database.to_string(),
            delete.selection_predicate.clone(),
        );

        if let (Some(tid), Some(storage)) = (tablet_id, Some(self.storage.clone())) {
            node = node.with_storage(tid, storage);
        }

        // Pass transaction context if set
        if let Some(ref tx_ctx) = self.transaction_ctx {
            node = node.with_transaction_ctx(tx_ctx.clone());
        }

        Ok(ExecutionPlan::Delete(node))
    }

    fn create_alter_table_node(&self, alter: &AlterTableNode) -> Result<ExecutionPlan, PlanExecutionError> {
        let database = alter.database.as_deref().unwrap_or("default");
        let operations: Vec<String> = alter.operations.iter().map(|op| format!("{:?}", op)).collect();

        let node = AlterTableExecNode::new(
            database.to_string(),
            alter.table_name.clone(),
            operations,
        );

        Ok(ExecutionPlan::AlterTable(node))
    }

    fn create_insert_node(&self, insert: &InsertNode, children: &[PlanNode]) -> Result<ExecutionPlan, PlanExecutionError> {
        let database = insert.database.as_deref().unwrap_or("default");
        let tablet_id = self.resolve_tablet_id(database, &insert.table_name).ok();

        // Get table schema for Values node
        let values_schema = self.get_table_schema(database, &insert.table_name)?;

        // Create child execution plan from first child (either Values or Select)
        let child_plan = if !children.is_empty() {
            let child = &children[0];
            match &child.node_type {
                PlanNodeType::Values(vals) => {
                    // If INSERT specifies column list, pass raw rows to InsertExecNode for expansion
                    // instead of creating ValuesExecNode with mismatched schema
                    if !insert.columns.is_empty() {
                        None  // Will use with_raw_rows() below
                    } else {
                        // Create ValuesExecNode directly with schema (full column INSERT)
                        Some(ExecutionPlan::Values(ValuesExecNode::new(
                            vals.rows.clone(),
                            values_schema.clone(),
                        )))
                    }
                }
                _ => {
                    // For SELECT or other, use the normal conversion
                    Some(self.create_exec_plan_impl(child)?)
                }
            }
        } else {
            None
        };

        let mut node = InsertExecNode::new(
            insert.table_name.clone(),
            database.to_string(),
        )
        .with_columns(insert.columns.clone())
        .with_on_duplicate_key_update(
            insert.on_duplicate_key_update.iter()
                .map(|s| (s.column.clone(), s.value.clone()))
                .collect()
        );

        // If partial column INSERT with VALUES, pass raw rows for expansion
        if !insert.columns.is_empty() && !children.is_empty() {
            if let PlanNodeType::Values(vals) = &children[0].node_type {
                node = node.with_raw_rows(vals.rows.clone(), values_schema.clone());
            }
        }

        // Always set child plan if we have one
        if let Some(child) = child_plan {
            node = node.with_child(Box::new(child));
        }

        if let (Some(tid), Some(storage)) = (tablet_id, Some(self.storage.clone())) {
            node = node.with_storage(tid, storage);
        }

        // Pass transaction context if set
        if let Some(ref tx_ctx) = self.transaction_ctx {
            node = node.with_transaction_ctx(tx_ctx.clone());
        }

        Ok(ExecutionPlan::Insert(node))
    }

    /// Get the schema for a table.
    fn get_table_schema(&self, database: &str, table_name: &str) -> Result<Schema, PlanExecutionError> {
        let table = self.catalog
            .get_table(database, table_name)
            .ok_or_else(|| PlanExecutionError::TableNotFound(format!("{}.{}", database, table_name)))?;

        // Convert table columns to schema fields
        let fields: Vec<types::Field> = table.columns.iter().map(|col| {
            types::Field::new(&col.name, col.data_type.clone(), col.nullable)
        }).collect();

        Ok(Schema::new(fields))
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
                tracing::error!("Error during execution: {}", e);
                return Err(PlanExecutionError::StorageError(e.to_string()));
            }
        }
    }

    // Close the plan
    let _ = exec_plan.close().await;

    Ok(results)
}
