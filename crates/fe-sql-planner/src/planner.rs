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
            Statement::Describe(db, table) => self.plan_describe(db, table),
            Statement::ShowColumns(db, table) => self.plan_show_columns(db, table),
            Statement::Union(union) => self.plan_union(union),
            Statement::TruncateTable { database, table, if_exists } => {
                self.plan_truncate_table(database, table, if_exists)
            }
            Statement::ShowCreateTable(db, table) => self.plan_show_create_table(db, table),
            Statement::CreateView { database, name, if_not_exists, query, columns } => {
                self.plan_create_view(database, name, if_not_exists, query, columns)
            }
        }
    }

    // ---- DDL ----

    fn plan_describe(&self, db: String, table: String) -> Result<PlanNode, DrorisError> {
        let target_db = if db.is_empty() { &self.current_database } else { &db };
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                table_name: "information_schema.columns".into(),
                database: Some("information_schema".into()),
                columns: vec!["column_name".into(), "data_type".into(), "is_nullable".into(), "column_default".into(), "column_comment".into()],
                predicates: vec![
                    format!("table_schema = '{}'", target_db),
                    format!("table_name = '{}'", table),
                ],
                limit: None,
            }),
            vec![],
        ))
    }

    fn plan_show_columns(&self, db: Option<String>, table: Option<String>) -> Result<PlanNode, DrorisError> {
        let target_db = db.as_deref().unwrap_or(&self.current_database);
        let table_pred = if let Some(tbl) = table {
            format!("table_name = '{}'", tbl)
        } else {
            "1=1".to_string()
        };
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                table_name: "information_schema.columns".into(),
                database: Some("information_schema".into()),
                columns: vec!["column_name".into(), "data_type".into(), "is_nullable".into(), "column_default".into(), "column_comment".into()],
                predicates: vec![
                    format!("table_schema = '{}'", target_db),
                    table_pred,
                ],
                limit: None,
            }),
            vec![],
        ))
    }

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
        // Parse qualified table name (db.table) into (database, table_name).
        let (database, table_name) = if stmt.table.contains('.') {
            let parts: Vec<&str> = stmt.table.splitn(2, '.').collect();
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            (None, stmt.table.clone())
        };

        let children = if let Some(query) = stmt.query {
            vec![self.plan_query(query)?]
        } else if !stmt.values.is_empty() {
            return Err(DrorisError::Plan("INSERT VALUES not yet implemented".into()));
        } else {
            return Err(DrorisError::Plan("INSERT must have VALUES or SELECT".into()));
        };

        Ok(self.make_node(
            PlanNodeType::Insert(InsertNode {
                table_name,
                database,
                columns: stmt.columns,
            }),
            children,
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
        // 0. CTE (WITH clause) - plan the CTE first if present
        if let Some(cte) = &query.with {
            let cte_plan = self.plan_query((*cte.query).clone())?;
            let cte_name = cte.name.clone();
            let cte_columns = if cte.columns.is_empty() {
                cte_plan.output_columns()
            } else {
                cte.columns.clone()
            };

            // Wrap the CTE in a CteNode that registers it as a temporary result
            let cte_wrapper = self.make_node(
                PlanNodeType::Cte(CteNode {
                    name: cte_name.clone(),
                    columns: cte_columns.clone(),
                }),
                vec![cte_plan],
            );

            // Plan the main query
            let main_plan = self.plan_query_body(&query, Some(&cte_name))?;

            // Combine CTE wrapper with main query
            Ok(self.make_node(
                PlanNodeType::Project(ProjectNode {
                    exprs: main_plan.output_columns(),
                }),
                vec![cte_wrapper, main_plan],
            ))
        } else {
            self.plan_query_body(&query, None)
        }
    }

    fn plan_query_body(&self, query: &QueryStmt, cte_name: Option<&str>) -> Result<PlanNode, DrorisError> {
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

        // 2. WHERE clause - handle subqueries (IN/EXISTS)
        if let Some(where_expr) = &query.r#where {
            plan = self.plan_where_clause(where_expr, plan)?;
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

    /// Plan WHERE clause, handling subqueries (IN/EXISTS) by converting to semi-joins
    fn plan_where_clause(&self, where_expr: &fe_sql_parser::ast::Expr, mut plan: PlanNode) -> Result<PlanNode, DrorisError> {
        match where_expr {
            fe_sql_parser::ast::Expr::Exists(subquery) => {
                // EXISTS subquery -> SemiJoin
                let subquery_plan = self.plan_query(*subquery.clone())?;

                // For EXISTS, we check if any row exists from the subquery
                // Use a simple column comparison (can be enhanced later)
                let left_col = plan.output_columns().first().cloned().unwrap_or_else(|| "*".to_string());
                let right_col = subquery_plan.output_columns().first().cloned().unwrap_or_else(|| "*".to_string());

                Ok(self.make_node(
                    PlanNodeType::SemiJoin(SemiJoinNode {
                        left_key: left_col,
                        right_key: right_col,
                        condition: None,
                    }),
                    vec![plan, subquery_plan],
                ))
            }
            fe_sql_parser::ast::Expr::InSubquery { expr: _, query, negated } => {
                // IN subquery -> SemiJoin or AntiSemiJoin
                let subquery_plan = self.plan_query(*query.clone())?;

                let left_col = plan.output_columns().first().cloned().unwrap_or_else(|| "*".to_string());
                let right_col = subquery_plan.output_columns().first().cloned().unwrap_or_else(|| "*".to_string());

                if *negated {
                    // NOT IN -> AntiSemiJoin
                    Ok(self.make_node(
                        PlanNodeType::AntiSemiJoin(AntiSemiJoinNode {
                            left_key: left_col,
                            right_key: right_col,
                            condition: None,
                        }),
                        vec![plan, subquery_plan],
                    ))
                } else {
                    // IN -> SemiJoin
                    Ok(self.make_node(
                        PlanNodeType::SemiJoin(SemiJoinNode {
                            left_key: left_col,
                            right_key: right_col,
                            condition: None,
                        }),
                        vec![plan, subquery_plan],
                    ))
                }
            }
            _ => {
                // Regular WHERE clause - use Filter node
                Ok(self.make_node(
                    PlanNodeType::Filter(FilterNode {
                        predicate: expression::expr_to_string(where_expr),
                    }),
                    vec![plan],
                ))
            }
        }
    }

    fn plan_table_ref(&self, table_ref: &TableRef) -> Result<PlanNode, DrorisError> {
        self.plan_table_ref_with_cte(table_ref, None)
    }

    fn plan_table_ref_with_cte(&self, table_ref: &TableRef, _cte_name: Option<&str>) -> Result<PlanNode, DrorisError> {
        match table_ref {
            TableRef::Table { name, alias: _ } => {
                // Check if this is a reference to a CTE
                // For now, we'll resolve it normally and handle CTE references in the execution layer

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

    // ---- Additional DDL/DML planning ----

    fn plan_union(&self, union: fe_sql_parser::ast::UnionStmt) -> Result<PlanNode, DrorisError> {
        let left_plan = self.plan_query(*union.left)?;
        let right_plan = self.plan_query(*union.right)?;

        let union_type = match union.op {
            fe_sql_parser::ast::UnionOperator::Union => {
                if union.all { "UNION ALL" } else { "UNION" }
            }
            fe_sql_parser::ast::UnionOperator::Except => {
                if union.all { "EXCEPT ALL" } else { "EXCEPT" }
            }
            fe_sql_parser::ast::UnionOperator::Intersect => {
                if union.all { "INTERSECT ALL" } else { "INTERSECT" }
            }
        };

        Ok(self.make_node(
            PlanNodeType::Union(UnionNode {
                input_count: 2,
            }),
            vec![left_plan, right_plan],
        ))
    }

    fn plan_truncate_table(
        &self,
        database: Option<String>,
        table_name: String,
        if_exists: bool,
    ) -> Result<PlanNode, DrorisError> {
        let target_db = database.unwrap_or_else(|| self.current_database.clone());
        Ok(self.make_node(
            PlanNodeType::TruncateTable(TruncateTableNode {
                database: Some(target_db),
                table_name,
                if_exists,
            }),
            vec![],
        ))
    }

    fn plan_show_create_table(&self, db: String, table: String) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::ShowCreateTable(ShowCreateTableNode {
                database: db,
                table_name: table,
            }),
            vec![],
        ))
    }

    fn plan_create_view(
        &self,
        database: Option<String>,
        name: String,
        if_not_exists: bool,
        query: String,
        columns: Vec<String>,
    ) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::CreateView(CreateViewNode {
                database,
                view_name: name,
                if_not_exists,
                query,
                columns,
            }),
            vec![],
        ))
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
