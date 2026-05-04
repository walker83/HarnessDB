use fe_catalog::{CatalogManager, MaterializedView, RefreshStrategy};
use fe_sql_parser::ast::QueryStmt;
use crate::plan_node::{PlanNode, PlanNodeId, PlanNodeType, ScanNode, PlanStats};

pub fn rewrite_query(
    query: &QueryStmt,
    catalog: &CatalogManager,
) -> Option<PlanNode> {
    if let Some(ref from) = query.from {
        let query_tables = extract_tables_from_table_ref(from);
        if query_tables.is_empty() {
            return None;
        }

        let query_table = query_tables.first()?;
        let (db, tbl) = if let Some(pos) = query_table.find('.') {
            (&query_table[..pos], Some(&query_table[pos + 1..]))
        } else {
            ("", None)
        };

        let db_name = if db.is_empty() { "test_db" } else { db };
        let table_name = tbl.unwrap_or(query_table);

        if let Some(mv) = catalog.get_materialized_view(db_name, table_name) {
            return Some(create_mv_scan_plan(&mv, query));
        }
    }
    None
}

fn extract_tables_from_table_ref(table_ref: &fe_sql_parser::ast::TableRef) -> Vec<String> {
    match table_ref {
        fe_sql_parser::ast::TableRef::Table { name, .. } => vec![name.clone()],
        fe_sql_parser::ast::TableRef::Join { left, right, .. } => {
            let mut tables = extract_tables_from_table_ref(left);
            tables.extend(extract_tables_from_table_ref(right));
            tables
        }
        fe_sql_parser::ast::TableRef::Subquery { query, .. } => {
            extract_tables_from_query(query)
        }
    }
}

fn extract_tables_from_query(query: &QueryStmt) -> Vec<String> {
    if let Some(ref from) = query.from {
        extract_tables_from_table_ref(from)
    } else {
        vec![]
    }
}

fn is_aggregate_column(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("count_")
        || lower.starts_with("sum_")
        || lower.starts_with("avg_")
        || lower.starts_with("max_")
        || lower.starts_with("min_")
}

fn get_select_item_name(item: &fe_sql_parser::ast::SelectItem) -> Option<String> {
    match &item.expr {
        fe_sql_parser::ast::Expr::ColumnRef { column, .. } => Some(column.clone()),
        fe_sql_parser::ast::Expr::Wildcard => Some("*".to_string()),
        _ => item.alias.clone(),
    }
}

fn can_use_mv_without_groupby(query: &QueryStmt, mv: &MaterializedView) -> bool {
    for item in &query.select_list {
        if let Some(name) = get_select_item_name(item) {
            if name == "*" {
                return true;
            }
            if !mv.schema.iter().any(|c| c.name == name) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn can_use_mv_with_groupby(query: &QueryStmt, mv: &MaterializedView) -> bool {
    if query.group_by.is_empty() {
        return false;
    }

    for item in &query.select_list {
        if let Some(name) = get_select_item_name(item) {
            if !mv.schema.iter().any(|c| c.name == name) {
                if !is_aggregate_expr(&item.expr) {
                    return false;
                }
            }
        }
    }

    for gb_expr in &query.group_by {
        let col_name = expr_column_name(gb_expr);
        if let Some(name) = col_name {
            if !mv.schema.iter().any(|c| c.name == name) {
                return false;
            }
        }
    }

    true
}

fn expr_column_name(expr: &fe_sql_parser::ast::Expr) -> Option<String> {
    match expr {
        fe_sql_parser::ast::Expr::ColumnRef { column, .. } => Some(column.clone()),
        _ => None,
    }
}

fn is_aggregate_expr(expr: &fe_sql_parser::ast::Expr) -> bool {
    match expr {
        fe_sql_parser::ast::Expr::FunctionCall { name, .. } => {
            matches!(
                name.to_uppercase().as_str(),
                "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "BITMAP_UNION" | "HLL_UNION"
            )
        }
        _ => false,
    }
}

fn create_mv_scan_plan(mv: &MaterializedView, _query: &QueryStmt) -> PlanNode {
    PlanNode {
        id: PlanNodeId(0),
        node_type: PlanNodeType::Scan(ScanNode {
            catalog: None,
            table_name: mv.name.clone(),
            database: Some(mv.database.clone()),
            columns: mv.schema.iter().map(|c| c.name.clone()).collect(),
            predicates: vec![],
            limit: None,
        }),
        children: vec![],
        stats: PlanStats::default(),
    }
}

pub fn extract_base_tables(query: &QueryStmt) -> Vec<(String, String)> {
    let mut tables = Vec::new();
    if let Some(ref from) = query.from {
        extract_tables_recursive(from, &mut tables);
    }
    tables
}

fn extract_tables_recursive(table_ref: &fe_sql_parser::ast::TableRef, tables: &mut Vec<(String, String)>) {
    match table_ref {
        fe_sql_parser::ast::TableRef::Table { name, .. } => {
            let parts: Vec<&str> = name.splitn(2, '.').collect();
            if parts.len() == 2 {
                tables.push((parts[0].to_string(), parts[1].to_string()));
            } else {
                tables.push((String::new(), name.clone()));
            }
        }
        fe_sql_parser::ast::TableRef::Join { left, right, .. } => {
            extract_tables_recursive(left, tables);
            extract_tables_recursive(right, tables);
        }
        fe_sql_parser::ast::TableRef::Subquery { query, .. } => {
            if let Some(ref from) = query.from {
                extract_tables_recursive(from, tables);
            }
        }
    }
}

pub fn extract_refresh_strategy(mv_definition: &str) -> RefreshStrategy {
    let upper = mv_definition.to_uppercase();
    if upper.contains("REFRESH COMPLETE") {
        RefreshStrategy::Immediate
    } else if upper.contains("REFRESH FAST") {
        RefreshStrategy::Manual
    } else if upper.contains("SCHEDULE") {
        RefreshStrategy::Scheduled(String::new())
    } else {
        RefreshStrategy::Manual
    }
}