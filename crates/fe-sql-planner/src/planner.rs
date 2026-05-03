use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use common::DrorisError;
use fe_catalog::CatalogManager;
use fe_sql_parser::ast::*;

use crate::expression;
use crate::plan_node::*;

/// The SQL planner converts parsed AST statements into logical plan trees.
pub struct Planner {
    catalog: Arc<CatalogManager>,
    next_id: AtomicUsize,
    current_database: String,
}

impl Planner {
    pub fn new(catalog: Arc<CatalogManager>) -> Self {
        Self {
            catalog,
            next_id: AtomicUsize::new(0),
            current_database: "information_schema".to_string(),
        }
    }

    pub fn set_database(&mut self, db: &str) {
        self.current_database = db.to_string();
    }

    pub fn database(&self) -> &str {
        &self.current_database
    }

    fn next_node_id(&self) -> PlanNodeId {
        PlanNodeId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    fn default_stats(&self) -> PlanStats {
        PlanStats::default()
    }

    fn make_node(&self, node_type: PlanNodeType, children: Vec<PlanNode>) -> PlanNode {
        PlanNode {
            id: self.next_node_id(),
            node_type,
            children,
            stats: self.default_stats(),
        }
    }

    // ---- Top-level dispatch ----

    pub fn plan(&self, stmt: Statement) -> Result<PlanNode, DrorisError> {
        match stmt {
            Statement::Query(query) => self.plan_query(query),
            Statement::Insert(insert) => self.plan_insert(insert),
            Statement::CreateDatabase(create_db) => self.plan_create_database(create_db),
            Statement::CreateTable(create_tbl) => self.plan_create_table(create_tbl),
            Statement::DropDatabase(drop_db) => self.plan_drop_database(drop_db),
            Statement::DropTable(drop_tbl) => self.plan_drop_table(drop_tbl),
            Statement::UseDatabase(db) => self.plan_use(db),
            Statement::ShowDatabases => self.plan_show_databases(),
            Statement::ShowTables(db) => self.plan_show_tables(db),
            Statement::Explain(explain) => {
                // Plan the inner statement; the explain wrapper is transparent
                // at the logical plan level.
                self.plan(*explain.statement)
            }
            Statement::AlterTable(_) => Err(DrorisError::Plan(
                "ALTER TABLE is not yet supported".into(),
            )),
            Statement::SetVariable(_) => Err(DrorisError::Plan(
                "SET variable is not yet supported".into(),
            )),
            Statement::ShowCreateTable(..) => Err(DrorisError::Plan(
                "SHOW CREATE TABLE is not yet supported".into(),
            )),
        }
    }

    // ---- DDL ----

    fn plan_create_database(&self, stmt: CreateDatabaseStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::CreateDatabase(CreateDatabaseNode {
                name: stmt.name,
                if_not_exists: stmt.if_not_exists,
            }),
            vec![],
        ))
    }

    fn plan_create_table(&self, stmt: CreateTableStmt) -> Result<PlanNode, DrorisError> {
        let columns: Vec<ColumnDefPlan> = stmt
            .columns
            .iter()
            .map(|c| ColumnDefPlan {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullable: c.nullable,
                agg_type: c.agg_type.clone(),
            })
            .collect();

        let partition_info = stmt.partition.map(|p| {
            format!(
                "{}({})",
                p.partition_type,
                p.columns.join(", ")
            )
        });

        let distribution_info = stmt.distribution.map(|d| {
            format!(
                "{}({}) BUCKETS {}",
                d.dist_type,
                d.columns.join(", "),
                d.buckets
            )
        });

        Ok(self.make_node(
            PlanNodeType::CreateTable(CreateTableNode {
                database: stmt.database,
                table_name: stmt.name,
                if_not_exists: stmt.if_not_exists,
                columns,
                keys_type: format!("{:?}", stmt.keys_type),
                partition_info,
                distribution_info,
            }),
            vec![],
        ))
    }

    fn plan_drop_database(&self, stmt: DropDatabaseStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DropDatabase(DropDatabaseNode {
                name: stmt.name,
                if_exists: stmt.if_exists,
            }),
            vec![],
        ))
    }

    fn plan_drop_table(&self, stmt: DropTableStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DropTable(DropTableNode {
                database: stmt.database,
                table_name: stmt.name,
                if_exists: stmt.if_exists,
            }),
            vec![],
        ))
    }

    // ---- DML ----

    fn plan_insert(&self, stmt: InsertStmt) -> Result<PlanNode, DrorisError> {
        let database = if stmt.table.contains('.') {
            let parts: Vec<&str> = stmt.table.splitn(2, '.').collect();
            // If qualified name is used, split it.
            // But InsertStmt only has a single `table` field, so we keep it simple.
            None
        } else {
            None
        };

        Ok(self.make_node(
            PlanNodeType::Insert(InsertNode {
                table_name: stmt.table,
                database,
                columns: stmt.columns,
            }),
            vec![],
        ))
    }

    // ---- Utility ----

    fn plan_use(&self, db: String) -> Result<PlanNode, DrorisError> {
        // USE is a session-level command; we represent it as a trivial plan
        // that the executor uses to switch the session database.
        // For now we validate the database exists.
        if self.catalog.get_database(&db).is_none() {
            return Err(DrorisError::Plan(format!(
                "database '{}' does not exist",
                db
            )));
        }
        // Return a trivial scan on dual as a marker.
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                table_name: format!("__use_{}", db),
                database: None,
                columns: vec![],
                predicates: vec![],
                limit: None,
            }),
            vec![],
        ))
    }

    fn plan_show_databases(&self) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                table_name: "information_schema.schemata".into(),
                database: Some("information_schema".into()),
                columns: vec!["schema_name".into()],
                predicates: vec![],
                limit: None,
            }),
            vec![],
        ))
    }

    fn plan_show_tables(&self, db: Option<String>) -> Result<PlanNode, DrorisError> {
        let target_db = db.as_deref().unwrap_or(&self.current_database);
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                table_name: "information_schema.tables".into(),
                database: Some("information_schema".into()),
                columns: vec!["table_name".into()],
                predicates: vec![format!("table_schema = '{}'", target_db)],
                limit: None,
            }),
            vec![],
        ))
    }

    // ---- Query planning ----

    fn plan_query(&self, query: QueryStmt) -> Result<PlanNode, DrorisError> {
        // 1. FROM clause (table references, joins, subqueries).
        let mut plan = if let Some(table_ref) = &query.from {
            self.plan_table_ref(table_ref)?
        } else {
            self.make_node(
                PlanNodeType::Scan(ScanNode {
                    table_name: "dual".into(),
                    database: None,
                    columns: vec![],
                    predicates: vec![],
                    limit: None,
                }),
                vec![],
            )
        };

        // 2. WHERE clause.
        if let Some(where_expr) = &query.r#where {
            plan = self.make_node(
                PlanNodeType::Filter(FilterNode {
                    predicate: expression::expr_to_string(where_expr),
                }),
                vec![plan],
            );
        }

        // 3. GROUP BY + aggregate functions.
        if !query.group_by.is_empty() || self.has_aggregates(&query.select_list) {
            let group_by: Vec<String> = query
                .group_by
                .iter()
                .map(expression::expr_to_string)
                .collect();

            let aggregates: Vec<AggregateExpr> = query
                .select_list
                .iter()
                .filter_map(|item| {
                    if let Expr::FunctionCall {
                        name,
                        args,
                        distinct: _,
                    } = &item.expr
                    {
                        if is_aggregate_function(name) {
                            Some(AggregateExpr {
                                func: name.clone(),
                                arg: args
                                    .first()
                                    .map(expression::expr_to_string)
                                    .unwrap_or_else(|| "*".into()),
                                alias: item.alias.clone(),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            plan = self.make_node(
                PlanNodeType::Aggregate(AggregateNode {
                    group_by,
                    aggregates,
                }),
                vec![plan],
            );
        }

        // 4. HAVING clause (filter after aggregation).
        if let Some(having) = &query.having {
            plan = self.make_node(
                PlanNodeType::Filter(FilterNode {
                    predicate: expression::expr_to_string(having),
                }),
                vec![plan],
            );
        }

        // 5. SELECT list projection.
        let proj_exprs: Vec<String> = query
            .select_list
            .iter()
            .map(|item| match &item.expr {
                Expr::Wildcard => "*".into(),
                _ => expression::expr_to_string(&item.expr),
            })
            .collect();

        plan = self.make_node(
            PlanNodeType::Project(ProjectNode { exprs: proj_exprs }),
            vec![plan],
        );

        // 6. ORDER BY.
        if !query.order_by.is_empty() {
            let order_items: Vec<SortItem> = query
                .order_by
                .iter()
                .map(|o| SortItem {
                    expr: expression::expr_to_string(&o.expr),
                    ascending: o.ascending,
                })
                .collect();
            plan = self.make_node(
                PlanNodeType::Sort(SortNode {
                    order_by: order_items,
                }),
                vec![plan],
            );
        }

        // 7. LIMIT / OFFSET.
        if let Some(limit) = query.limit {
            plan = self.make_node(
                PlanNodeType::Limit(LimitNode {
                    limit,
                    offset: query.offset.unwrap_or(0),
                }),
                vec![plan],
            );
        }

        Ok(plan)
    }

    /// Check if the select list contains aggregate function calls at the top level.
    fn has_aggregates(&self, select_list: &[SelectItem]) -> bool {
        select_list.iter().any(|item| {
            if let Expr::FunctionCall { name, .. } = &item.expr {
                is_aggregate_function(name)
            } else {
                false
            }
        })
    }

    fn plan_table_ref(&self, table_ref: &TableRef) -> Result<PlanNode, DrorisError> {
        match table_ref {
            TableRef::Table { name, alias: _ } => {
                let (database, table_name) = self.resolve_table_name(name);
                let columns = self.resolve_table_columns(&database, &table_name);

                Ok(self.make_node(
                    PlanNodeType::Scan(ScanNode {
                        table_name,
                        database: Some(database),
                        columns,
                        predicates: vec![],
                        limit: None,
                    }),
                    vec![],
                ))
            }
            TableRef::Join {
                left,
                right,
                r#type,
                condition,
            } => {
                let left_plan = self.plan_table_ref(left)?;
                let right_plan = self.plan_table_ref(right)?;

                let join_type = match r#type {
                    JoinType::Inner => JoinTypePlan::Inner,
                    JoinType::LeftOuter => JoinTypePlan::LeftOuter,
                    JoinType::RightOuter => JoinTypePlan::RightOuter,
                    JoinType::FullOuter => JoinTypePlan::FullOuter,
                    JoinType::Cross => JoinTypePlan::Cross,
                };

                let cond_str = condition
                    .as_ref()
                    .map(expression::expr_to_string);

                Ok(self.make_node(
                    PlanNodeType::Join(JoinNode {
                        join_type,
                        condition: cond_str,
                    }),
                    vec![left_plan, right_plan],
                ))
            }
            TableRef::Subquery { query, alias: _ } => self.plan_query(*query.clone()),
        }
    }

    /// Split "db.table" or plain "table" into (database, table_name).
    fn resolve_table_name(&self, name: &str) -> (String, String) {
        if let Some(pos) = name.find('.') {
            let db = &name[..pos];
            let tbl = &name[pos + 1..];
            (db.to_string(), tbl.to_string())
        } else {
            (self.current_database.clone(), name.to_string())
        }
    }

    /// Look up the column names for a table from the catalog.
    fn resolve_table_columns(&self, database: &str, table_name: &str) -> Vec<String> {
        self.catalog
            .get_table(database, table_name)
            .map(|t| t.column_names().into_iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new(Arc::new(CatalogManager::new()))
    }
}

/// Recognised aggregate function names.
fn is_aggregate_function(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "COUNT"
            | "SUM"
            | "AVG"
            | "MIN"
            | "MAX"
            | "BITMAP_UNION"
            | "HLL_UNION"
            | "PERCENTILE_UNION"
            | "GROUP_CONCAT"
    )
}
