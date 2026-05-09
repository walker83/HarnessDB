use fe_sql_parser::ast::*;
use types::DataType;

/// Convert an AST expression to a human-readable SQL string.
pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit) => literal_to_string(lit),
        Expr::ColumnRef { table, column } => match table {
            Some(t) => format!("{}.{}", t, column),
            None => column.clone(),
        },
        Expr::BinaryOp { left, op, right } => {
            let l = expr_to_string(left);
            let r = expr_to_string(right);
            let o = binary_op_to_string(op);
            format!("{} {} {}", l, o, r)
        }
        Expr::UnaryOp { op, expr } => match op {
            UnaryOp::Not => format!("NOT ({})", expr_to_string(expr)),
            UnaryOp::Negate => format!("-({})", expr_to_string(expr)),
        },
        Expr::FunctionCall {
            name,
            args,
            distinct,
        } => {
            let distinct_str = if *distinct { "DISTINCT " } else { "" };
            let args_str: Vec<String> = args.iter().map(expr_to_string).collect();
            format!("{}({}{})", name, distinct_str, args_str.join(", "))
        }
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let neg = if *negated { "NOT " } else { "" };
            format!(
                "{} {}BETWEEN {} AND {}",
                expr_to_string(expr),
                neg,
                expr_to_string(low),
                expr_to_string(high)
            )
        }
        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let neg = if *negated { "NOT " } else { "" };
            let vals: Vec<String> = list.iter().map(expr_to_string).collect();
            format!("{} {}IN ({})", expr_to_string(expr), neg, vals.join(", "))
        }
        Expr::InSubquery {
            expr,
            query: _,
            negated,
        } => {
            let neg = if *negated { "NOT " } else { "" };
            format!("{} {}IN (subquery)", expr_to_string(expr), neg)
        }
        Expr::Exists(_) => "EXISTS (subquery)".to_string(),
        Expr::Subquery(_) => "(subquery)".to_string(),
        Expr::IsNull { expr, negated } => {
            let neg = if *negated { "NOT " } else { "" };
            format!("{} IS {}NULL", expr_to_string(expr), neg)
        }
        Expr::Like {
            expr,
            pattern,
            negated,
        } => {
            let neg = if *negated { "NOT " } else { "" };
            format!(
                "{} {}LIKE {}",
                expr_to_string(expr),
                neg,
                expr_to_string(pattern)
            )
        }
        Expr::Cast { expr, target_type } => {
            format!("CAST({} AS {})", expr_to_string(expr), target_type)
        }
        Expr::CaseWhen { cases, else_expr } => {
            let mut s = String::from("CASE ");
            for case in cases {
                s.push_str("WHEN ");
                s.push_str(&expr_to_string(&case.when));
                s.push_str(" THEN ");
                s.push_str(&expr_to_string(&case.then));
                s.push(' ');
            }
            if let Some(ee) = else_expr {
                s.push_str("ELSE ");
                s.push_str(&expr_to_string(ee));
                s.push(' ');
            }
            s.push_str("END");
            s
        }
        Expr::Wildcard => "*".to_string(),
        Expr::Default => "DEFAULT".to_string(),
    }
}

fn literal_to_string(lit: &LiteralValue) -> String {
    match lit {
        LiteralValue::Null => "NULL".to_string(),
        LiteralValue::Boolean(b) => b.to_string(),
        LiteralValue::Int64(n) => n.to_string(),
        LiteralValue::Float64(n) => n.to_string(),
        LiteralValue::String(s) => format!("'{}'", s),
        LiteralValue::Date(s) => format!("DATE '{}'", s),
    }
}

fn binary_op_to_string(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Eq => "=",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "AND",
        BinaryOp::Or => "OR",
        BinaryOp::Plus => "+",
        BinaryOp::Minus => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::Like => "LIKE",
        BinaryOp::NotLike => "NOT LIKE",
        BinaryOp::In => "IN",
        BinaryOp::NotIn => "NOT IN",
    }
}

/// Infer the output DataType of an expression.
/// Returns None if the type cannot be determined from static analysis alone.
pub fn infer_type(expr: &Expr) -> Option<DataType> {
    match expr {
        Expr::Literal(lit) => Some(literal_type(lit)),
        Expr::ColumnRef { .. } => {
            // Column type depends on the table schema; caller must resolve it.
            None
        }
        Expr::BinaryOp { op, .. } => {
            match op {
                // Comparison / logical operators produce Boolean.
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::Like
                | BinaryOp::NotLike
                | BinaryOp::In
                | BinaryOp::NotIn => Some(DataType::Boolean),
                // Arithmetic operators: for now default to Int64 if unknown.
                BinaryOp::Plus
                | BinaryOp::Minus
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo => Some(DataType::Int64),
            }
        }
        Expr::UnaryOp { op, expr } => match op {
            UnaryOp::Not => Some(DataType::Boolean),
            UnaryOp::Negate => infer_type(expr).or(Some(DataType::Int64)),
        },
        Expr::FunctionCall { name, .. } => aggregate_return_type(name),
        Expr::Between { .. } => Some(DataType::Boolean),
        Expr::InList { .. } | Expr::InSubquery { .. } => Some(DataType::Boolean),
        Expr::Exists(_) => Some(DataType::Boolean),
        Expr::Subquery(_) => None,
        Expr::IsNull { .. } => Some(DataType::Boolean),
        Expr::Like { .. } => Some(DataType::Boolean),
        Expr::Cast { target_type, .. } => parse_type_name(target_type),
        Expr::CaseWhen { .. } => None,
        Expr::Wildcard => None,
        Expr::Default => None,
    }
}

fn literal_type(lit: &LiteralValue) -> DataType {
    match lit {
        LiteralValue::Null => DataType::Null,
        LiteralValue::Boolean(_) => DataType::Boolean,
        LiteralValue::Int64(_) => DataType::Int64,
        LiteralValue::Float64(_) => DataType::Float64,
        LiteralValue::String(_) => DataType::String,
        LiteralValue::Date(_) => DataType::Date,
    }
}

fn aggregate_return_type(name: &str) -> Option<DataType> {
    match name.to_uppercase().as_str() {
        "COUNT" => Some(DataType::Int64),
        "SUM" => Some(DataType::Int64),
        "AVG" => Some(DataType::Float64),
        "MIN" | "MAX" => None, // Same as input type; needs schema to resolve.
        "BITMAP_UNION" | "HLL_UNION" => None,
        _ => None,
    }
}

fn parse_type_name(type_name: &str) -> Option<DataType> {
    match type_name.to_uppercase().as_str() {
        "BOOLEAN" | "BOOL" => Some(DataType::Boolean),
        "TINYINT" | "INT8" => Some(DataType::Int8),
        "SMALLINT" | "INT16" => Some(DataType::Int16),
        "INT" | "INTEGER" | "INT32" => Some(DataType::Int32),
        "BIGINT" | "INT64" => Some(DataType::Int64),
        "FLOAT" | "FLOAT32" => Some(DataType::Float32),
        "DOUBLE" | "FLOAT64" => Some(DataType::Float64),
        "DATE" => Some(DataType::Date),
        "DATETIME" => Some(DataType::DateTime),
        "VARCHAR" | "STRING" | "TEXT" => Some(DataType::String),
        _ => None,
    }
}

/// Attempt to resolve a column reference type using a list of (table_name, columns) pairs.
/// Each column entry is (column_name, DataType).
pub fn resolve_column_type<'a>(
    table: Option<&str>,
    column: &str,
    schemas: &'a [(&str, Vec<(&str, DataType)>)],
) -> Option<&'a DataType> {
    for (tbl_name, cols) in schemas {
        if let Some(t) = table
            && t != *tbl_name {
                continue;
            }
        for (col_name, dt) in cols {
            if col_name == &column {
                return Some(dt);
            }
        }
    }
    None
}

/// Simplify an expression tree by applying algebraic identities.
/// Returns the (possibly same) simplified expression.
pub fn simplify(expr: Expr) -> Expr {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOp::And,
            right,
        } => {
            let l = simplify(*left);
            let r = simplify(*right);
            // TRUE AND x => x
            if is_true(&l) {
                return r;
            }
            if is_true(&r) {
                return l;
            }
            // FALSE AND x => FALSE
            if is_false(&l) || is_false(&r) {
                return Expr::Literal(LiteralValue::Boolean(false));
            }
            Expr::BinaryOp {
                left: Box::new(l),
                op: BinaryOp::And,
                right: Box::new(r),
            }
        }
        Expr::BinaryOp {
            left,
            op: BinaryOp::Or,
            right,
        } => {
            let l = simplify(*left);
            let r = simplify(*right);
            // FALSE OR x => x
            if is_false(&l) {
                return r;
            }
            if is_false(&r) {
                return l;
            }
            // TRUE OR x => TRUE
            if is_true(&l) || is_true(&r) {
                return Expr::Literal(LiteralValue::Boolean(true));
            }
            Expr::BinaryOp {
                left: Box::new(l),
                op: BinaryOp::Or,
                right: Box::new(r),
            }
        }
        Expr::BinaryOp {
            left,
            op,
            right,
        } => {
            let l = simplify(*left);
            let r = simplify(*right);
            // Try constant folding for arithmetic ops.
            if let Some(result) = try_fold_arithmetic(&l, op, &r) {
                return result;
            }
            Expr::BinaryOp {
                left: Box::new(l),
                op,
                right: Box::new(r),
            }
        }
        Expr::UnaryOp {
            op: UnaryOp::Not,
            expr,
        } => {
            let inner = simplify(*expr);
            // NOT TRUE => FALSE, NOT FALSE => TRUE
            match &inner {
                Expr::Literal(LiteralValue::Boolean(b)) => {
                    return Expr::Literal(LiteralValue::Boolean(!b));
                }
                // NOT (NOT x) => x
                Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: e,
                } => {
                    return *e.clone();
                }
                _ => {}
            }
            Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(inner),
            }
        }
        other => other,
    }
}

/// Attempt constant folding on arithmetic binary operations.
fn try_fold_arithmetic(left: &Expr, op: BinaryOp, right: &Expr) -> Option<Expr> {
    let lv = eval_int_literal(left)?;
    let rv = eval_int_literal(right)?;
    let result = match op {
        BinaryOp::Plus => lv.checked_add(rv)?,
        BinaryOp::Minus => lv.checked_sub(rv)?,
        BinaryOp::Multiply => lv.checked_mul(rv)?,
        BinaryOp::Divide => {
            if rv == 0 {
                return None;
            }
            lv.checked_div(rv)?
        }
        BinaryOp::Modulo => {
            if rv == 0 {
                return None;
            }
            lv % rv
        }
        _ => return None,
    };
    Some(Expr::Literal(LiteralValue::Int64(result)))
}

fn eval_int_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Literal(LiteralValue::Int64(n)) => Some(*n),
        _ => None,
    }
}

fn is_true(expr: &Expr) -> bool {
    matches!(expr, Expr::Literal(LiteralValue::Boolean(true)))
}

fn is_false(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Literal(LiteralValue::Boolean(false)) | Expr::Literal(LiteralValue::Null)
    )
}

/// Collect all column references in the expression.
/// Returns Vec of (table_alias, column_name).
pub fn collect_columns(expr: &Expr) -> Vec<(Option<String>, String)> {
    let mut cols = Vec::new();
    collect_columns_recursive(expr, &mut cols);
    cols
}

fn collect_columns_recursive(expr: &Expr, cols: &mut Vec<(Option<String>, String)>) {
    match expr {
        Expr::ColumnRef { table, column } => {
            cols.push((table.clone(), column.clone()));
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_columns_recursive(left, cols);
            collect_columns_recursive(right, cols);
        }
        Expr::UnaryOp { expr, .. } => {
            collect_columns_recursive(expr, cols);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_columns_recursive(arg, cols);
            }
        }
        Expr::Between { expr, low, high, .. } => {
            collect_columns_recursive(expr, cols);
            collect_columns_recursive(low, cols);
            collect_columns_recursive(high, cols);
        }
        Expr::InList { expr, list, .. } => {
            collect_columns_recursive(expr, cols);
            for item in list {
                collect_columns_recursive(item, cols);
            }
        }
        Expr::IsNull { expr, .. } => {
            collect_columns_recursive(expr, cols);
        }
        Expr::Like { expr, pattern, .. } => {
            collect_columns_recursive(expr, cols);
            collect_columns_recursive(pattern, cols);
        }
        Expr::Cast { expr, .. } => {
            collect_columns_recursive(expr, cols);
        }
        Expr::CaseWhen { cases, else_expr } => {
            for case in cases {
                collect_columns_recursive(&case.when, cols);
                collect_columns_recursive(&case.then, cols);
            }
            if let Some(ee) = else_expr {
                collect_columns_recursive(ee, cols);
            }
        }
        _ => {}
    }
}
