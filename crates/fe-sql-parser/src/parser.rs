use crate::ast::*;
use crate::error::ParseError;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let dialect = sqlparser::dialect::MySqlDialect {};
    let statements = sqlparser::parser::Parser::parse_sql(&dialect, sql)
        .map_err(|e| ParseError::SyntaxError {
            position: 0,
            message: e.to_string(),
        })?;

    statements
        .into_iter()
        .map(|stmt| convert_statement(stmt))
        .collect()
}

fn convert_statement(
    stmt: sqlparser::ast::Statement,
) -> Result<Statement, ParseError> {
    match stmt {
        sqlparser::ast::Statement::Query(query) => {
            let query_stmt = convert_query(*query)?;
            Ok(Statement::Query(query_stmt))
        }
        sqlparser::ast::Statement::Insert(stmt) => {
            let table_name = stmt.table_name.to_string();
            let cols: Vec<String> = stmt.columns.iter().map(|c| c.value.clone()).collect();
            let values_list: Vec<Vec<Expr>> = vec![];
            Ok(Statement::Insert(InsertStmt {
                table: table_name,
                columns: cols,
                values: values_list,
                query: stmt.source.map(|q| convert_query(*q).ok().unwrap()),
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
            Ok(Statement::CreateTable(CreateTableStmt {
                database,
                name: table_name,
                if_not_exists: stmt.if_not_exists,
                columns: col_defs,
                keys_type: KeysType::Duplicate,
                partition: None,
                distribution: None,
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
            show_options, ..
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
            let inner = convert_statement(*statement)?;
            Ok(Statement::Explain(ExplainStmt {
                statement: Box::new(inner),
                verbose,
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
    match *query.body {
        sqlparser::ast::SetExpr::Select(select) => {
            let order_by = query.order_by.map(|ob| ob.exprs).unwrap_or_default();
            let limit = query.limit;
            let offset = query.offset;
            convert_select(*select, order_by, limit, offset)
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
            })
        }
        _ => Err(ParseError::Unsupported("non-SELECT query body".to_string())),
    }
}

fn convert_set_expr(expr: sqlparser::ast::SetExpr) -> Result<QueryStmt, ParseError> {
    match expr {
        sqlparser::ast::SetExpr::Select(select) => convert_select(*select, vec![], None, None),
        sqlparser::ast::SetExpr::Query(query) => convert_query(*query),
        _ => Err(ParseError::Unsupported("set operation not supported".to_string())),
    }
}

fn convert_select(
    select: sqlparser::ast::Select,
    order_by: Vec<sqlparser::ast::OrderByExpr>,
    limit: Option<sqlparser::ast::Expr>,
    offset: Option<sqlparser::ast::Offset>,
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

    let from = select.from.into_iter().next().map(|t| convert_table_ref(t));

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
                    sqlparser::ast::FunctionArg::Unnamed(arg_expr) => {
                        match arg_expr {
                            sqlparser::ast::FunctionArgExpr::Expr(expr) => convert_expr(expr),
                            _ => Expr::Wildcard,
                        }
                    }
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
