use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use common::{DrorisError, CatalogError, PlanError};
use fe_catalog::{CatalogManager, Catalog};
use fe_sql_parser::ast::*;

use crate::expression;
use crate::plan_node::*;

/// The SQL planner converts parsed AST statements into logical plan trees.
pub struct Planner {
    catalog: Arc<CatalogManager>,
    external_catalogs: std::collections::HashMap<String, Arc<dyn Catalog>>,
    next_id: AtomicUsize,
    current_database: String,
}

impl Planner {
    pub fn new(catalog: Arc<CatalogManager>) -> Self {
        Self {
            catalog,
            external_catalogs: std::collections::HashMap::new(),
            next_id: AtomicUsize::new(0),
            current_database: "information_schema".to_string(),
        }
    }

    pub fn set_database(&mut self, db: &str) {
        self.current_database = db.to_string();
    }

    pub fn register_external_catalog(&mut self, name: &str, catalog: Arc<dyn Catalog>) {
        self.external_catalogs.insert(name.to_string(), catalog);
    }

    pub fn unregister_external_catalog(&mut self, name: &str) {
        self.external_catalogs.remove(name);
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
            Statement::Update(update) => self.plan_update(update),
            Statement::Delete(delete) => self.plan_delete(delete),
            Statement::CreateDatabase(create_db) => self.plan_create_database(create_db),
            Statement::CreateTable(create_tbl) => self.plan_create_table(create_tbl),
            Statement::DropDatabase(drop_db) => self.plan_drop_database(drop_db),
            Statement::DropTable(drop_tbl) => self.plan_drop_table(drop_tbl),
            Statement::UseDatabase(db) => self.plan_use(db),
            Statement::ShowDatabases => self.plan_show_databases(),
            Statement::ShowTables(db) => self.plan_show_tables(db),
            Statement::Explain(explain) => {
                self.plan(*explain.statement)
            }
            Statement::AlterTable(alter) => self.plan_alter_table(alter),
            Statement::SetVariable(_) => Err(DrorisError::plan(PlanError::UnsupportedOperation,
                "SET variable is not yet supported"
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
            Statement::CreateRepository(stmt) => self.plan_create_repository(stmt),
            Statement::DropRepository(stmt) => self.plan_drop_repository(stmt),
            Statement::ShowRepositories => self.plan_show_repositories(),
            Statement::BackupDatabase(stmt) => self.plan_backup_database(stmt),
            Statement::RestoreDatabase(stmt) => self.plan_restore_database(stmt),
            Statement::CreateMaterializedView(stmt) => self.plan_create_materialized_view(stmt),
            Statement::DropMaterializedView(stmt) => self.plan_drop_materialized_view(stmt),
            Statement::AlterMaterializedView(stmt) => self.plan_alter_materialized_view(stmt),
            Statement::RefreshMaterializedView(stmt) => self.plan_refresh_materialized_view(stmt),
            Statement::CreateUser(stmt) => self.plan_create_user(stmt),
            Statement::DropUser(stmt) => self.plan_drop_user(stmt),
            Statement::ShowUsers => self.plan_show_users(),
            Statement::CreateCatalog(stmt) => self.plan_create_catalog(stmt),
            Statement::DropCatalog(stmt) => self.plan_drop_catalog(stmt),
            Statement::ShowCatalogs => self.plan_show_catalogs(),
            Statement::RefreshCatalog(stmt) => self.plan_refresh_catalog(stmt),
            Statement::AlterDatabase(stmt) => self.plan_alter_database(stmt),
            Statement::ShowCreateDatabase(name) => self.plan_show_create_database(name),
            Statement::DropView(stmt) => self.plan_drop_view(stmt),
            Statement::AlterView(stmt) => self.plan_alter_view(stmt),
            Statement::ShowCreateView(db, name) => self.plan_show_create_view(db, name),
            
            // Batch 3 & 4: Management statements - handled directly in fe_main.rs
            Statement::ExportTable(_)
            | Statement::CancelExport(_)
            | Statement::ShowExport(_)
            | Statement::CreateFunction(_)
            | Statement::DropFunction(_)
            | Statement::ShowFunctions(_)
            | Statement::ShowCreateFunction(_, _)
            | Statement::DescFunction(_, _)
            | Statement::AnalyzeTable(_)
            | Statement::AlterStats(_)
            | Statement::DropStats(_)
            | Statement::DropAnalyzeJob(_)
            | Statement::KillAnalyzeJob(_)
            | Statement::ShowAnalyze(_)
            | Statement::ShowStats(_)
            | Statement::ShowTableStats(_)
            | Statement::CreateJob(_)
            | Statement::DropJob(_)
            | Statement::PauseJob(_)
            | Statement::ResumeJob(_)
            | Statement::CancelTask(_)
            | Statement::InstallPlugin(_)
            | Statement::UninstallPlugin(_)
            | Statement::ShowPlugins
            | Statement::RecoverDatabase(_)
            | Statement::RecoverTable(_)
            | Statement::RecoverPartition(_)
            | Statement::DropCatalogRecycleBin(_)
            | Statement::ShowCatalogRecycleBin
            | Statement::CreateSqlBlockRule(_)
            | Statement::AlterSqlBlockRule(_)
            | Statement::DropSqlBlockRule(_)
            | Statement::ShowSqlBlockRule(_)
            | Statement::CreateRowPolicy(_)
            | Statement::DropRowPolicy(_)
            | Statement::ShowRowPolicy(_) => {
                Err(DrorisError::plan(PlanError::UnsupportedOperation,
                    "This statement is handled directly by the execution layer, not the planner"
                ))
            }
        }
    }

    // ---- Catalog Management ----

    fn plan_create_catalog(&self, _stmt: CreateCatalogStmt) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("CREATE CATALOG not yet implemented in planner".to_string()))
    }

    fn plan_drop_catalog(&self, _stmt: DropCatalogStmt) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("DROP CATALOG not yet implemented in planner".to_string()))
    }

    fn plan_show_catalogs(&self) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("SHOW CATALOGS not yet implemented in planner".to_string()))
    }

    fn plan_refresh_catalog(&self, _stmt: RefreshCatalogStmt) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("REFRESH CATALOG not yet implemented in planner".to_string()))
    }

    // ---- User Management ----

    fn plan_create_user(&self, _stmt: fe_sql_parser::ast::CreateUserStmt) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("CREATE USER not yet implemented in planner".to_string()))
    }

    fn plan_drop_user(&self, _stmt: fe_sql_parser::ast::DropUserStmt) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("DROP USER not yet implemented in planner".to_string()))
    }

    fn plan_show_users(&self) -> Result<PlanNode, DrorisError> {
        Err(DrorisError::Internal("SHOW USERS not yet implemented in planner".to_string()))
    }

    // ---- Batch 1: New statement handlers ----

    fn plan_alter_database(&self, stmt: fe_sql_parser::ast::AlterDatabaseStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DdlCommand(DdlCommandNode { command: format!("ALTER DATABASE {}", stmt.name) }),
            vec![],
        ))
    }

    fn plan_show_create_database(&self, name: String) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DdlCommand(DdlCommandNode { command: format!("SHOW CREATE DATABASE {}", name) }),
            vec![],
        ))
    }

    fn plan_drop_view(&self, stmt: fe_sql_parser::ast::DropViewStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DdlCommand(DdlCommandNode { command: format!("DROP VIEW {}", stmt.name) }),
            vec![],
        ))
    }

    fn plan_alter_view(&self, stmt: fe_sql_parser::ast::AlterViewStmt) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DdlCommand(DdlCommandNode { command: format!("ALTER VIEW {}", stmt.name) }),
            vec![],
        ))
    }

    fn plan_show_create_view(&self, db: String, name: String) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DdlCommand(DdlCommandNode { command: format!("SHOW CREATE VIEW {}.{}", db, name) }),
            vec![],
        ))
    }

    // ---- DDL ----

    fn plan_describe(&self, db: String, table: String) -> Result<PlanNode, DrorisError> {
        let target_db = if db.is_empty() { &self.current_database } else { &db };
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                catalog: None,
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
                catalog: None,
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
            // VALUES case: create a Values plan node as child
            let values_node = self.make_node(
                PlanNodeType::Values(VirtualValuesNode {
                    rows: stmt.values.clone(),
                }),
                vec![],
            );
            vec![values_node]
        } else {
            return Err(DrorisError::plan(PlanError::InvalidExpression, "INSERT must have VALUES or SELECT"));
        };

        Ok(self.make_node(
            PlanNodeType::Insert(InsertNode {
                table_name,
                database,
                columns: stmt.columns,
                is_overwrite: stmt.is_overwrite,
            }),
            children,
        ))
    }

    fn plan_update(&self, stmt: UpdateStmt) -> Result<PlanNode, DrorisError> {
        let (database, table_name) = if stmt.table.contains('.') {
            let parts: Vec<&str> = stmt.table.splitn(2, '.').collect();
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            (None, stmt.table.clone())
        };

        let set_clauses: Vec<SetClausePlan> = stmt
            .set_clauses
            .iter()
            .map(|s| SetClausePlan {
                column: s.column.clone(),
                value: expression::expr_to_string(&s.value),
            })
            .collect();

        let selection_predicate = stmt
            .selection
            .as_ref()
            .map(expression::expr_to_string);

        Ok(self.make_node(
            PlanNodeType::Update(UpdateNode {
                table_name,
                database,
                set_clauses,
                selection_predicate,
            }),
            vec![],
        ))
    }

    fn plan_delete(&self, stmt: DeleteStmt) -> Result<PlanNode, DrorisError> {
        let (database, table_name) = if stmt.table.contains('.') {
            let parts: Vec<&str> = stmt.table.splitn(2, '.').collect();
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            (None, stmt.table.clone())
        };

        let selection_predicate = stmt
            .selection
            .as_ref()
            .map(expression::expr_to_string);

        Ok(self.make_node(
            PlanNodeType::Delete(DeleteNode {
                table_name,
                database,
                selection_predicate,
            }),
            vec![],
        ))
    }

    fn plan_alter_table(&self, stmt: AlterTableStmt) -> Result<PlanNode, DrorisError> {
        let (database, table_name) = if stmt.table.contains('.') {
            let parts: Vec<&str> = stmt.table.splitn(2, '.').collect();
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            (None, stmt.table.clone())
        };

        let operations: Vec<AlterOperationPlan> = stmt
            .operations
            .iter()
            .map(|op| match op {
                AlterOperation::AddColumn(col) => AlterOperationPlan::AddColumn {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                },
                AlterOperation::DropColumn(name) => AlterOperationPlan::DropColumn {
                    name: name.clone(),
                },
                AlterOperation::ModifyColumn(col) => AlterOperationPlan::ModifyColumn {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                },
                AlterOperation::RenameTable(new_name) => AlterOperationPlan::RenameTable {
                    new_name: new_name.clone(),
                },
                AlterOperation::RenameColumn { old_name, new_name } => AlterOperationPlan::RenameColumn {
                    old_name: old_name.clone(),
                    new_name: new_name.clone(),
                },
                AlterOperation::SetComment(comment) => AlterOperationPlan::SetComment {
                    comment: comment.clone(),
                },
                AlterOperation::SetProperty(props) => AlterOperationPlan::SetProperty {
                    properties: props.clone(),
                },
            })
            .collect();

        Ok(self.make_node(
            PlanNodeType::AlterTable(AlterTableNode {
                database,
                table_name,
                operations,
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
            return Err(DrorisError::catalog(CatalogError::DatabaseNotFound, format!(
                "database '{}' does not exist",
                db
            )));
        }
        // Return a trivial scan on dual as a marker.
        Ok(self.make_node(
            PlanNodeType::Scan(ScanNode {
                catalog: None,
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
                catalog: None,
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
                catalog: None,
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

    fn plan_query_body(&self, query: &QueryStmt, _cte_name: Option<&str>) -> Result<PlanNode, DrorisError> {
        // 1. FROM clause (table references, joins, subqueries).
        let mut plan = if let Some(table_ref) = &query.from {
            self.plan_table_ref(table_ref)?
        } else {
            self.make_node(
                PlanNodeType::Scan(ScanNode {
                    catalog: None,
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
    fn plan_where_clause(&self, where_expr: &fe_sql_parser::ast::Expr, plan: PlanNode) -> Result<PlanNode, DrorisError> {
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
                let (catalog, database, table_name) = self.resolve_table_name(name);

                let columns = if let Some(ref cat) = catalog {
                    self.resolve_external_table_columns(cat, &database, &table_name)
                } else {
                    self.resolve_table_columns(&database, &table_name)
                };

                Ok(self.make_node(
                    PlanNodeType::Scan(ScanNode {
                        catalog,
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

    /// Split "catalog.db.table", "db.table", or plain "table" into (catalog, database, table_name).
    fn resolve_table_name(&self, name: &str) -> (Option<String>, String, String) {
        let parts: Vec<&str> = name.split('.').collect();
        match parts.len() {
            1 => (None, self.current_database.clone(), parts[0].to_string()),
            2 => (None, parts[0].to_string(), parts[1].to_string()),
            3 => (Some(parts[0].to_string()), parts[1].to_string(), parts[2].to_string()),
            _ => (None, self.current_database.clone(), name.to_string()),
        }
    }

    /// Look up the column names for a table from the catalog.
    fn resolve_table_columns(&self, database: &str, table_name: &str) -> Vec<String> {
        self.catalog
            .get_table(database, table_name)
            .map(|t| t.column_names().into_iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Look up the column names for a table from an external catalog.
    fn resolve_external_table_columns(&self, _catalog_name: &str, _database: &str, _table_name: &str) -> Vec<String> {
        vec![]
    }

    // ---- Additional DDL/DML planning ----

    fn plan_union(&self, union: fe_sql_parser::ast::UnionStmt) -> Result<PlanNode, DrorisError> {
        let left_plan = self.plan_query(*union.left)?;
        let right_plan = self.plan_query(*union.right)?;

        let _union_type = match union.op {
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

    fn plan_create_repository(
        &self,
        stmt: fe_sql_parser::ast::CreateRepositoryStmt,
    ) -> Result<PlanNode, DrorisError> {
        let repo_type = match stmt.repo_type {
            fe_sql_parser::ast::RepositoryType::Local => "local".to_string(),
            fe_sql_parser::ast::RepositoryType::S3 => "s3".to_string(),
            fe_sql_parser::ast::RepositoryType::Hdfs => "hdfs".to_string(),
        };
        Ok(self.make_node(
            PlanNodeType::CreateRepository(CreateRepositoryNode {
                name: stmt.name,
                repo_type,
                properties: stmt.properties,
            }),
            vec![],
        ))
    }

    fn plan_drop_repository(
        &self,
        stmt: fe_sql_parser::ast::DropRepositoryStmt,
    ) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DropRepository(DropRepositoryNode {
                name: stmt.name,
                if_exists: stmt.if_exists,
            }),
            vec![],
        ))
    }

    fn plan_show_repositories(&self) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::ShowRepositories(ShowRepositoriesNode),
            vec![],
        ))
    }

    fn plan_backup_database(
        &self,
        stmt: fe_sql_parser::ast::BackupDatabaseStmt,
    ) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::BackupDatabase(BackupDatabaseNode {
                database: stmt.database,
                repository: stmt.repository,
                backup_name: stmt.backup_name,
                properties: stmt.properties,
            }),
            vec![],
        ))
    }

    fn plan_restore_database(
        &self,
        stmt: fe_sql_parser::ast::RestoreDatabaseStmt,
    ) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::RestoreDatabase(RestoreDatabaseNode {
                database: stmt.database,
                repository: stmt.repository,
                backup_name: stmt.backup_name,
                properties: stmt.properties,
            }),
            vec![],
        ))
    }

    fn plan_create_materialized_view(
        &self,
        stmt: fe_sql_parser::ast::CreateMaterializedViewStmt,
    ) -> Result<PlanNode, DrorisError> {
        let refresh_type = stmt.refresh.as_ref().map(|r| {
            match r.r#type {
                fe_sql_parser::ast::RefreshType::Complete => "COMPLETE".to_string(),
                fe_sql_parser::ast::RefreshType::Fast => "FAST".to_string(),
            }
        });

        Ok(self.make_node(
            PlanNodeType::CreateMaterializedView(CreateMaterializedViewNode {
                database: stmt.database,
                view_name: stmt.name,
                if_not_exists: stmt.if_not_exists,
                query: stmt.query,
                columns: stmt.columns,
                refresh_type,
            }),
            vec![],
        ))
    }

    fn plan_drop_materialized_view(
        &self,
        stmt: fe_sql_parser::ast::DropMaterializedViewStmt,
    ) -> Result<PlanNode, DrorisError> {
        Ok(self.make_node(
            PlanNodeType::DropMaterializedView(DropMaterializedViewNode {
                database: stmt.database,
                view_name: stmt.name,
                if_exists: stmt.if_exists,
            }),
            vec![],
        ))
    }

    fn plan_alter_materialized_view(
        &self,
        stmt: fe_sql_parser::ast::AlterMaterializedViewStmt,
    ) -> Result<PlanNode, DrorisError> {
        let operation = match stmt.operation {
            fe_sql_parser::ast::AlterMaterializedViewOperation::PauseRefresh => "PAUSE REFRESH".to_string(),
            fe_sql_parser::ast::AlterMaterializedViewOperation::ResumeRefresh => "RESUME REFRESH".to_string(),
            fe_sql_parser::ast::AlterMaterializedViewOperation::Rename(new_name) => format!("RENAME TO {}", new_name),
        };

        Ok(self.make_node(
            PlanNodeType::AlterMaterializedView(AlterMaterializedViewNode {
                database: stmt.database,
                view_name: stmt.name,
                operation,
            }),
            vec![],
        ))
    }

    fn plan_refresh_materialized_view(
        &self,
        stmt: fe_sql_parser::ast::RefreshMaterializedViewStmt,
    ) -> Result<PlanNode, DrorisError> {
        let refresh_type = match stmt.refresh_type {
            fe_sql_parser::ast::RefreshType::Complete => "COMPLETE",
            fe_sql_parser::ast::RefreshType::Fast => "FAST",
        };

        Ok(self.make_node(
            PlanNodeType::RefreshMaterializedView(RefreshMaterializedViewNode {
                database: stmt.database,
                view_name: stmt.name,
                refresh_type: refresh_type.to_string(),
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
