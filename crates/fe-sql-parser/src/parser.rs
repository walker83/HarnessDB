use crate::ast::*;
use crate::error::ParseError;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let dialect = sqlparser::dialect::MySqlDialect {};

    let (clean_sql, partition_override, distribution_override) = strip_clauses(sql);
    let statements = sqlparser::parser::Parser::parse_sql(&dialect, &clean_sql)
        .map_err(|e| ParseError::SyntaxError {
            position: 0,
            message: e.to_string(),
        })?;

    statements
        .into_iter()
        .enumerate()
        .map(|(i, stmt)| convert_statement(stmt, &clean_sql, &partition_override, &distribution_override, i))
        .collect()
}

fn strip_clauses(sql: &str) -> (String, Option<String>, Option<String>) {
    let upper = sql.to_uppercase();

    let pos_pby = upper.find(" PARTITION BY ");
    let pos_dby = upper.find(" DISTRIBUTED BY ");

    let pby = pos_pby.map(|p| {
        let part_str = &sql[p..];
        let part_end = find_clause_end(part_str);
        part_str[..part_end].to_string()
    });
    let dby = pos_dby.map(|p| {
        let dist_str = &sql[p..];
        let dist_end = find_clause_end(dist_str);
        dist_str[..dist_end].to_string()
    });

    let mut clean = sql.to_string();
    if let Some(ref p) = pby {
        if let Some(pos) = clean.find(p.as_str()) {
            clean.replace_range(pos..pos + p.len(), "");
            clean = clean.trim().to_string();
        }
    }
    if let Some(ref d) = dby {
        if let Some(pos) = clean.find(d.as_str()) {
            clean.replace_range(pos..pos + d.len(), "");
            clean = clean.trim().to_string();
        }
    }

    (clean, pby, dby)
}

fn find_clause_end(s: &str) -> usize {
    let trimmed = s.trim();
    let leading_ws = s.len() - trimmed.len();
    let upper = trimmed.to_uppercase();
    let is_dist = upper.starts_with("DISTRIBUTED BY");
    if !upper.starts_with("PARTITION BY") && !is_dist {
        return s.len();
    }

    let props_pos = upper.find(" PROPERTIES ");
    let search_end = props_pos.unwrap_or(trimmed.len());

    if is_dist {
        leading_ws + search_end
    } else {
        let partitions_pos = upper[..search_end].find(" PARTITIONS ");
        if let Some(p) = partitions_pos {
            let num_len = trimmed[p + " PARTITIONS ".len()..]
                .split_whitespace()
                .next()
                .map(|num| num.len())
                .unwrap_or(0);
            return leading_ws + p + " PARTITIONS ".len() + num_len;
        }
        leading_ws + search_end
    }
}

fn convert_statement(
    stmt: sqlparser::ast::Statement,
    _clean_sql: &str,
    partition_sql: &Option<String>,
    distribution_sql: &Option<String>,
    _stmt_index: usize,
) -> Result<Statement, ParseError> {
    match stmt {
        sqlparser::ast::Statement::Query(query) => {
            let query_stmt = convert_query(*query)?;
            Ok(Statement::Query(query_stmt))
        }
        sqlparser::ast::Statement::Insert(stmt) => {
            let table_name = stmt.table_name.to_string();
            let cols: Vec<String> = stmt.columns.iter().map(|c| c.value.clone()).collect();
            // Handle VALUES via source query
            let query_opt: Option<QueryStmt> = stmt.source.as_ref().and_then(|q| {
                if let sqlparser::ast::SetExpr::Values(_) = &*q.body {
                    None
                } else {
                    convert_query(*q.clone()).ok()
                }
            });
            let values_list: Vec<Vec<Expr>> = stmt.source.as_ref().and_then(|q| {
                if let sqlparser::ast::SetExpr::Values(values) = &*q.body {
                    Some(values.rows.iter().map(|row| {
                        row.iter().map(|e| convert_expr(e.clone())).collect()
                    }).collect())
                } else {
                    None
                }
            }).unwrap_or_default();
            Ok(Statement::Insert(InsertStmt {
                table: table_name,
                columns: cols,
                values: values_list,
                query: query_opt,
                is_overwrite: stmt.overwrite,
            }))
        }
        sqlparser::ast::Statement::CreateTable(stmt) => {
            let name_str = stmt.name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (database, table_name) = if parts.len() == 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
            };
            let col_defs: Vec<ColumnDef> = stmt.columns.iter().map(|c| ColumnDef {
                name: c.name.value.clone(),
                data_type: c.data_type.to_string(),
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: None,
            }).collect();

            let partition = partition_sql.as_ref().and_then(|s| extract_partition_def(s))
                .or_else(|| extract_partition_def(&stmt.to_string()));

            let distribution = distribution_sql.as_ref().and_then(|s| extract_distribution_def(s));

            Ok(Statement::CreateTable(CreateTableStmt {
                database,
                name: table_name,
                if_not_exists: stmt.if_not_exists,
                columns: col_defs,
                keys_type: KeysType::Duplicate,
                partition,
                distribution,
                properties: vec![],
            }))
        }
        sqlparser::ast::Statement::Drop {
            names, if_exists, ..
        } => {
            let name = names.first().map(|n| n.to_string()).unwrap_or_default();
            if name.contains('.') {
                let parts: Vec<&str> = name.splitn(2, '.').collect();
                Ok(Statement::DropTable(DropTableStmt {
                    database: Some(parts[0].to_string()),
                    name: parts[1].to_string(),
                    if_exists,
                }))
            } else {
                Ok(Statement::DropTable(DropTableStmt {
                    database: None,
                    name,
                    if_exists,
                }))
            }
        }
        sqlparser::ast::Statement::ExplainTable {
            table_name, ..
        } => {
            let name_str = table_name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (db, tbl) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (String::new(), parts[0].to_string())
            };
            Ok(Statement::Describe(db, tbl))
        }
        sqlparser::ast::Statement::ShowCreate {
            obj_name, ..
        } => {
            let name_str = obj_name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (db, tbl) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (String::new(), parts[0].to_string())
            };
            Ok(Statement::ShowCreateTable(db, tbl))
        }
        sqlparser::ast::Statement::ShowColumns {
            show_options: _, ..
        } => {
            // SHOW COLUMNS FROM table - table name is in the filter
            Ok(Statement::ShowColumns(None, None))
        }
        sqlparser::ast::Statement::Use(db) => {
            Ok(Statement::UseDatabase(db.to_string()))
        }
        sqlparser::ast::Statement::SetVariable {
            variables, value, ..
        } => {
            let var_name = match variables {
                sqlparser::ast::OneOrManyWithParens::One(o) => o.to_string(),
                sqlparser::ast::OneOrManyWithParens::Many(v) => v.first().map(|s: &sqlparser::ast::ObjectName| s.to_string()).unwrap_or_default(),
            };
            let value_expr = value.first().map(|e: &sqlparser::ast::Expr| convert_expr(e.clone())).unwrap_or(Expr::Literal(LiteralValue::Null));
            Ok(Statement::SetVariable(SetVariableStmt {
                variable: var_name,
                value: value_expr,
                is_global: false,
            }))
        }
        sqlparser::ast::Statement::Explain {
            statement, verbose, ..
        } => {
            let inner = convert_statement(*statement, _clean_sql, partition_sql, distribution_sql, _stmt_index)?;
            Ok(Statement::Explain(ExplainStmt {
                statement: Box::new(inner),
                verbose,
            }))
        }
        sqlparser::ast::Statement::Truncate { table_names, .. } => {
            let first_table = table_names.first();
            if let Some(table) = first_table {
                let name_str = table.name.to_string();
                let parts: Vec<&str> = name_str.split('.').collect();
                let (database, table) = if parts.len() == 2 {
                    (Some(parts[0].to_string()), parts[1].to_string())
                } else {
                    (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
                };
                Ok(Statement::TruncateTable {
                    database,
                    table,
                    if_exists: false,
                })
            } else {
                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "TRUNCATE requires at least one table".to_string(),
                })
            }
        }
        sqlparser::ast::Statement::CreateView {
            or_replace: _,
            materialized: _,
            name,
            columns,
            query,
            if_not_exists,
            ..
        } => {
            let name_str = name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (database, view_name) = if parts.len() == 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
            };
            let col_names: Vec<String> = columns.iter().map(|c: &sqlparser::ast::ViewColumnDef| c.name.value.clone()).collect();
            Ok(Statement::CreateView {
                database,
                name: view_name,
                if_not_exists,
                query: query.to_string(),
                columns: col_names,
            })
        }
        sqlparser::ast::Statement::Update { table, assignments, from: _, selection, returning: _, or: _ } => {
            let table_name = table.to_string();
            let set_clauses: Vec<SetClause> = assignments.iter().map(|s| {
                let column = match &s.target {
                    sqlparser::ast::AssignmentTarget::ColumnName(name) => name.to_string(),
                    sqlparser::ast::AssignmentTarget::Tuple(_) => String::new(),
                };
                let value = convert_expr(s.value.clone());
                SetClause { column, value }
            }).collect();
            let selection = selection.map(convert_expr);
            Ok(Statement::Update(UpdateStmt {
                table: table_name,
                set_clauses,
                selection,
            }))
        }
        sqlparser::ast::Statement::Delete(delete) => {
            let table_name = delete.tables.first().map(|t: &sqlparser::ast::ObjectName| t.to_string()).unwrap_or_default();
            let selection = delete.selection.map(convert_expr);
            Ok(Statement::Delete(DeleteStmt {
                table: table_name,
                selection,
            }))
        }
        _ => Err(ParseError::Unsupported(format!(
            "statement type: {:?}",
            stmt
        ))),
    }
}

fn convert_query(
    query: sqlparser::ast::Query,
) -> Result<QueryStmt, ParseError> {
    let cte = query.with.as_ref().and_then(|w| {
        w.cte_tables.first().map(|c| Cte {
            name: c.alias.name.value.clone(),
            columns: vec![],
            query: Box::new(convert_query(*c.query.clone()).unwrap_or_else(|_| QueryStmt {
                select_list: vec![],
                from: None,
                r#where: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                with: None,
            })),
        })
    });

    match *query.body {
        sqlparser::ast::SetExpr::Select(select) => {
            let order_by = query.order_by.map(|ob| ob.exprs).unwrap_or_default();
            let limit = query.limit;
            let offset = query.offset;
            convert_select(*select, order_by, limit, offset, cte)
        }
        sqlparser::ast::SetExpr::SetOperation { op, set_quantifier, left, right } => {
            let left_query = convert_set_expr(*left)?;
            let right_query = convert_set_expr(*right)?;
            let union_op = match op {
                sqlparser::ast::SetOperator::Union => UnionOperator::Union,
                sqlparser::ast::SetOperator::Except => UnionOperator::Except,
                sqlparser::ast::SetOperator::Intersect => UnionOperator::Intersect,
            };
            let _ = (union_op, set_quantifier);
            let _ = right_query;
            let order_by = query.order_by.map(|ob| ob.exprs).unwrap_or_default();
            Ok(QueryStmt {
                select_list: left_query.select_list,
                from: left_query.from,
                r#where: left_query.r#where,
                group_by: left_query.group_by,
                having: left_query.having,
                order_by: order_by.into_iter().map(|o| OrderByItem {
                    expr: convert_expr(o.expr),
                    ascending: o.asc.unwrap_or(true),
                    nulls_first: o.nulls_first.unwrap_or(true),
                }).collect(),
                limit: query.limit.and_then(|l| match l {
                    sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
                    _ => None,
                }),
                offset: query.offset.and_then(|o| match o.value {
                    sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
                    _ => None,
                }),
                with: cte,
            })
        }
        _ => Err(ParseError::Unsupported("non-SELECT query body".to_string())),
    }
}

fn convert_set_expr(expr: sqlparser::ast::SetExpr) -> Result<QueryStmt, ParseError> {
    match expr {
        sqlparser::ast::SetExpr::Select(select) => convert_select(*select, vec![], None, None, None),
        sqlparser::ast::SetExpr::Query(query) => convert_query(*query),
        _ => Err(ParseError::Unsupported("set operation not supported".to_string())),
    }
}

fn convert_select(
    select: sqlparser::ast::Select,
    order_by: Vec<sqlparser::ast::OrderByExpr>,
    limit: Option<sqlparser::ast::Expr>,
    offset: Option<sqlparser::ast::Offset>,
    cte: Option<Cte>,
) -> Result<QueryStmt, ParseError> {
    let select_list = select.projection.into_iter().map(|item| {
        match item {
            sqlparser::ast::SelectItem::UnnamedExpr(expr) => SelectItem {
                expr: convert_expr(expr),
                alias: None,
            },
            sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => SelectItem {
                expr: convert_expr(expr),
                alias: Some(alias.value),
            },
            sqlparser::ast::SelectItem::Wildcard(_) => SelectItem {
                expr: Expr::Wildcard,
                alias: None,
            },
            _ => SelectItem {
                expr: Expr::Wildcard,
                alias: None,
            },
        }
    }).collect();

    let from = select.from.into_iter().next().map(convert_table_ref);

    let group_by = match select.group_by {
        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => {
            exprs.into_iter().map(convert_expr).collect()
        }
        _ => vec![],
    };

    let order_by: Vec<OrderByItem> = order_by.into_iter().map(|o| OrderByItem {
        expr: convert_expr(o.expr),
        ascending: o.asc.unwrap_or(true),
        nulls_first: o.nulls_first.unwrap_or(true),
    }).collect();

    let limit = limit.and_then(|l| match l {
        sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
        _ => None,
    });

    let offset = offset.and_then(|o| match o.value {
        sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
        _ => None,
    });

    Ok(QueryStmt {
        select_list,
        from,
        r#where: select.selection.map(convert_expr),
        group_by,
        having: select.having.map(convert_expr),
        order_by,
        limit,
        offset,
        with: cte,
    })
}

fn extract_join_condition(op: &sqlparser::ast::JoinOperator) -> Option<sqlparser::ast::Expr> {
    use sqlparser::ast::JoinOperator;
    match op {
        JoinOperator::Inner(constraint) => extract_constraint_expr(constraint),
        JoinOperator::LeftOuter(constraint) => extract_constraint_expr(constraint),
        JoinOperator::RightOuter(constraint) => extract_constraint_expr(constraint),
        JoinOperator::FullOuter(constraint) => extract_constraint_expr(constraint),
        _ => None,
    }
}

fn extract_constraint_expr(constraint: &sqlparser::ast::JoinConstraint) -> Option<sqlparser::ast::Expr> {
    match constraint {
        sqlparser::ast::JoinConstraint::On(expr) => Some(expr.clone()),
        _ => None,
    }
}

fn convert_table_ref(t: sqlparser::ast::TableWithJoins) -> TableRef {
    let name = match &t.relation {
        sqlparser::ast::TableFactor::Table { name, alias, .. } => {
            let table_name = name.to_string();
            TableRef::Table {
                name: table_name,
                alias: alias.as_ref().map(|a| a.name.value.clone()),
            }
        }
        sqlparser::ast::TableFactor::Derived { subquery, alias, .. } => {
            let query = convert_query(*subquery.clone()).ok().unwrap();
            return TableRef::Subquery {
                query: Box::new(query),
                alias: alias.as_ref().map(|a| a.name.value.clone()).unwrap_or_default(),
            };
        }
        _ => TableRef::Table { name: "unknown".into(), alias: None },
    };

    t.joins.into_iter().fold(name, |left, join| {
        let right = convert_table_ref_simple(join.relation);
        let condition = extract_join_condition(&join.join_operator);
        TableRef::Join {
            left: Box::new(left),
            right: Box::new(right),
            r#type: match join.join_operator {
                sqlparser::ast::JoinOperator::Inner(_) => JoinType::Inner,
                sqlparser::ast::JoinOperator::LeftOuter(_) => JoinType::LeftOuter,
                sqlparser::ast::JoinOperator::RightOuter(_) => JoinType::RightOuter,
                sqlparser::ast::JoinOperator::FullOuter(_) => JoinType::FullOuter,
                sqlparser::ast::JoinOperator::CrossJoin => JoinType::Cross,
                _ => JoinType::Inner,
            },
            condition: condition.map(convert_expr),
        }
    })
}

fn convert_table_ref_simple(factor: sqlparser::ast::TableFactor) -> TableRef {
    match factor {
        sqlparser::ast::TableFactor::Table { name, alias, .. } => TableRef::Table {
            name: name.to_string(),
            alias: alias.map(|a| a.name.value),
        },
        _ => TableRef::Table { name: "unknown".into(), alias: None },
    }
}

fn convert_function_args(args: sqlparser::ast::FunctionArguments) -> Vec<Expr> {
    match args {
        sqlparser::ast::FunctionArguments::None => vec![],
        sqlparser::ast::FunctionArguments::Subquery(_) => vec![],
        sqlparser::ast::FunctionArguments::List(list) => {
            list.args.into_iter().map(|arg| {
                match arg {
                    sqlparser::ast::FunctionArg::Unnamed(arg_expr) |
                    sqlparser::ast::FunctionArg::Named { arg: arg_expr, .. } => {
                        match arg_expr {
                            sqlparser::ast::FunctionArgExpr::Expr(expr) => convert_expr(expr),
                            _ => Expr::Wildcard,
                        }
                    }
                    _ => Expr::Wildcard,
                }
            }).collect()
        }
    }
}

fn convert_expr(expr: sqlparser::ast::Expr) -> Expr {
    match expr {
        sqlparser::ast::Expr::Value(v) => Expr::Literal(match v {
            sqlparser::ast::Value::Number(n, _) => {
                if n.contains('.') || n.contains('e') || n.contains('E') {
                    LiteralValue::Float64(n.parse().unwrap_or(0.0))
                } else {
                    LiteralValue::Int64(n.parse().unwrap_or(0))
                }
            }
            sqlparser::ast::Value::SingleQuotedString(s) => LiteralValue::String(s),
            sqlparser::ast::Value::DoubleQuotedString(s) => LiteralValue::String(s),
            sqlparser::ast::Value::Boolean(b) => LiteralValue::Boolean(b),
            sqlparser::ast::Value::Null => LiteralValue::Null,
            _ => LiteralValue::Null,
        }),
        sqlparser::ast::Expr::Identifier(id) => Expr::ColumnRef {
            table: None,
            column: id.value,
        },
        sqlparser::ast::Expr::CompoundIdentifier(ids) => {
            let len = ids.len();
            if len == 2 {
                Expr::ColumnRef {
                    table: Some(ids[0].value.clone()),
                    column: ids[1].value.clone(),
                }
            } else {
                Expr::ColumnRef { table: None, column: ids.last().map(|i| i.value.clone()).unwrap_or_default() }
            }
        }
        sqlparser::ast::Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(convert_expr(*left)),
            op: convert_binary_op(op),
            right: Box::new(convert_expr(*right)),
        },
        sqlparser::ast::Expr::UnaryOp { op, expr } => Expr::UnaryOp {
            op: match op {
                sqlparser::ast::UnaryOperator::Not => UnaryOp::Not,
                sqlparser::ast::UnaryOperator::Minus => UnaryOp::Negate,
                _ => UnaryOp::Not,
            },
            expr: Box::new(convert_expr(*expr)),
        },
        sqlparser::ast::Expr::Function(fun) => {
            let name = fun.name.to_string();
            let args = convert_function_args(fun.args);
            Expr::FunctionCall { name, args, distinct: false }
        }
        _ => Expr::Wildcard,
    }
}

fn convert_binary_op(op: sqlparser::ast::BinaryOperator) -> BinaryOp {
    use sqlparser::ast::BinaryOperator;
    match op {
        BinaryOperator::Eq => BinaryOp::Eq,
        BinaryOperator::NotEq => BinaryOp::NotEq,
        BinaryOperator::Lt => BinaryOp::Lt,
        BinaryOperator::LtEq => BinaryOp::LtEq,
        BinaryOperator::Gt => BinaryOp::Gt,
        BinaryOperator::GtEq => BinaryOp::GtEq,
        BinaryOperator::And => BinaryOp::And,
        BinaryOperator::Or => BinaryOp::Or,
        BinaryOperator::Plus => BinaryOp::Plus,
        BinaryOperator::Minus => BinaryOp::Minus,
        BinaryOperator::Multiply => BinaryOp::Multiply,
        BinaryOperator::Divide => BinaryOp::Divide,
        BinaryOperator::Modulo => BinaryOp::Modulo,
        _ => BinaryOp::Eq,
    }
}

fn extract_partition_def(sql: &str) -> Option<PartitionDef> {
    let upper = sql.to_uppercase();
    let pby_pos = upper.find("PARTITION BY ")?;
    let after_pby = &sql[pby_pos + "PARTITION BY ".len()..].trim();

    if after_pby.to_uppercase().starts_with("RANGE") {
        let rest = &after_pby["RANGE".len()..].trim();
        let (columns, rest) = extract_parenthesized(rest)?;

        let range_partitions: Vec<RangePartition> = extract_range_partitions(rest.trim());
        if range_partitions.is_empty() {
            return None;
        }
        Some(PartitionDef::Range(RangePartitionDef {
            columns: split_columns(&columns).into_iter().map(|s| s.to_string()).collect(),
            partitions: range_partitions,
        }))
    } else if after_pby.to_uppercase().starts_with("LIST") {
        let rest = &after_pby["LIST".len()..].trim();
        let (columns, rest) = extract_parenthesized(rest)?;

        let list_partitions: Vec<ListPartition> = extract_list_partitions(rest.trim());
        if list_partitions.is_empty() {
            return None;
        }
        Some(PartitionDef::List(ListPartitionDef {
            columns: split_columns(&columns).into_iter().map(|s| s.to_string()).collect(),
            partitions: list_partitions,
        }))
    } else if after_pby.to_uppercase().starts_with("HASH") {
        let rest = &after_pby["HASH".len()..].trim();
        let (columns, rest) = extract_parenthesized(rest)?;

        let num = extract_partitions_count(rest.trim()).unwrap_or(4);
        Some(PartitionDef::Hash(HashPartitionDef {
            columns: split_columns(&columns).into_iter().map(|s| s.to_string()).collect(),
            num_partitions: num,
        }))
    } else {
        None
    }
}

fn extract_parenthesized(s: &str) -> Option<(String, &str)> {
    let s = s.trim();
    let start = s.find('(')?;
    let mut depth = 0;
    for (i, ch) in s[start..].char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
            if depth == 0 {
                return Some((s[start + 1..start + i].to_string(), &s[start + i + 1..]));
            }
        }
    }
    None
}

fn split_columns(s: &str) -> Vec<&str> {
    s.split(',')
        .map(|col| col.trim().trim_matches(|c| c == '\'' || c == '"' || c == '`'))
        .filter(|c| !c.is_empty())
        .collect()
}

fn extract_distribution_def(sql: &str) -> Option<DistributionDef> {
    let upper = sql.to_uppercase();
    let dby_pos = upper.find("DISTRIBUTED BY ")?;
    let after_dby = &sql[dby_pos + "DISTRIBUTED BY ".len()..].trim();

    if after_dby.to_uppercase().starts_with("HASH") {
        let rest = &after_dby["HASH".len()..].trim();
        let (columns, rest) = extract_parenthesized(rest)?;

        let buckets = extract_buckets_count(rest.trim()).unwrap_or(10);
        Some(DistributionDef {
            dist_type: "HASH".to_string(),
            columns: split_columns(&columns).into_iter().map(|s| s.to_string()).collect(),
            buckets,
        })
    } else if after_dby.to_uppercase().starts_with("RANDOM") {
        let buckets = extract_buckets_count(after_dby.trim()).unwrap_or(10);
        Some(DistributionDef {
            dist_type: "RANDOM".to_string(),
            columns: vec![],
            buckets,
        })
    } else {
        None
    }
}

fn extract_range_partitions(s: &str) -> Vec<RangePartition> {
    let s = s.trim();
    let inner = s
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(s);
    let mut partitions = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let upper = rest.to_uppercase();
        if let Some(part_pos) = upper.find("PARTITION ") {
            let after_part = &rest[part_pos + "PARTITION ".len()..].trim();
            let name_end = after_part
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_part.len());
            let part_name = after_part[..name_end].to_string();

            let after_name = after_part[name_end..].trim();
            let after_name_upper = after_name.to_uppercase();
            if let Some(vlt_pos) = after_name_upper.find("VALUES LESS THAN") {
                let after_vlt = &after_name[vlt_pos + "VALUES LESS THAN".len()..].trim();
                if let Some((less_than, _)) = extract_parenthesized(after_vlt) {
                    let lt_val = less_than.trim().trim_matches(|c: char| c == '\'' || c == '"').to_string();
                    partitions.push(RangePartition {
                        name: part_name.clone(),
                        less_than: lt_val,
                    });
                    rest = advance_to_next_partition(rest);
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }
    partitions
}

fn extract_list_partitions(s: &str) -> Vec<ListPartition> {
    let s = s.trim();
    let inner = s
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(s);
    let mut partitions = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let upper = rest.to_uppercase();
        if let Some(part_pos) = upper.find("PARTITION ") {
            let after_part = &rest[part_pos + "PARTITION ".len()..].trim();
            let name_end = after_part
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_part.len());
            let part_name = after_part[..name_end].to_string();

            let after_name = after_part[name_end..].trim();
            let after_name_upper = after_name.to_uppercase();
            if let Some(vi_pos) = after_name_upper.find("VALUES IN") {
                let after_vi = &after_name[vi_pos + "VALUES IN".len()..].trim();
                if let Some((values_str, _)) = extract_parenthesized(after_vi) {
                    let values: Vec<String> = values_str
                        .split(',')
                        .map(|v| v.trim().trim_matches(|c: char| c == '\'' || c == '"').to_string())
                        .filter(|v| !v.is_empty())
                        .collect();
                    partitions.push(ListPartition {
                        name: part_name.clone(),
                        values,
                    });
                    rest = advance_to_next_partition(rest);
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }
    partitions
}

fn advance_to_next_partition(s: &str) -> &str {
    let upper = s.to_uppercase();
    let skip = "PARTITION ".len();
    if let Some(pos) = upper[skip..].find("PARTITION ") {
        s[pos + skip..].trim()
    } else {
        ""
    }
}

fn extract_partitions_count(s: &str) -> Option<usize> {
    let upper = s.to_uppercase();
    let pos = upper.find("PARTITIONS ")?;
    let after = &s[pos + "PARTITIONS ".len()..].trim();
    after
        .split_whitespace()
        .next()?
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
}

fn extract_buckets_count(s: &str) -> Option<usize> {
    let upper = s.to_uppercase();
    let pos = upper.find("BUCKETS ")?;
    let after = &s[pos + "BUCKETS ".len()..].trim();
    after
        .split_whitespace()
        .next()?
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
}
