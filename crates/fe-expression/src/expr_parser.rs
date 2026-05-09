use crate::expr::{BinaryOperator, Expr, UnaryOperator, ColumnRef, FunctionCall, WhenThen};
use crate::evaluator::ExprEvaluator;
use types::{Block, Vector, ScalarValue};

/// Parser for expression strings (like "a + b" or "ABS(x)").
/// This allows BE to parse expressions stored as strings.
pub struct ExprStringParser {
    evaluator: ExprEvaluator,
}

impl ExprStringParser {
    pub fn new() -> Self {
        Self { evaluator: ExprEvaluator::new() }
    }

    /// Parse an expression string into an Expr.
    /// Returns None if parsing fails.
    pub fn parse(&self, expr_str: &str) -> Option<Expr> {
        let trimmed = expr_str.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Handle parentheses first: "(a + b)"
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            let inner = &trimmed[1..trimmed.len() - 1];
            return self.parse(inner);
        }

        // Handle binary operations
        if let Some(expr) = self.parse_binary_op(trimmed) {
            return Some(expr);
        }

        // Handle unary operations
        if let Some(expr) = self.parse_unary_op(trimmed) {
            return Some(expr);
        }

        // Handle CASE WHEN expressions
        if let Some(expr) = self.parse_case_when(trimmed) {
            return Some(expr);
        }

        // Handle function calls
        if let Some(expr) = self.parse_function_call(trimmed) {
            return Some(expr);
        }

        // Handle column references
        if self.is_identifier(trimmed) && !self.is_literal(trimmed) {
            return Some(Expr::ColumnRef(ColumnRef {
                index: 0,
                name: trimmed.to_string(),
            }));
        }

        // Handle wildcard '*' for COUNT(*)
        if trimmed == "*" {
            return Some(Expr::Wildcard);
        }

        // Handle literals
        self.parse_literal(trimmed).map(Expr::Literal)
    }

    fn parse_binary_op(&self, s: &str) -> Option<Expr> {
        for (op_str, op) in [
            (" OR ", BinaryOperator::Or),
            (" AND ", BinaryOperator::And),
            (" LIKE ", BinaryOperator::Like),
            (" NOT LIKE ", BinaryOperator::NotLike),
            (" >= ", BinaryOperator::Ge),
            (" <= ", BinaryOperator::Le),
            (" != ", BinaryOperator::Ne),
            (" = ", BinaryOperator::Eq),
            (" > ", BinaryOperator::Gt),
            (" < ", BinaryOperator::Lt),
            (" * ", BinaryOperator::Multiply),
            (" / ", BinaryOperator::Divide),
            (" % ", BinaryOperator::Modulo),
            (" + ", BinaryOperator::Add),
            (" - ", BinaryOperator::Subtract),
        ] {
            let mut paren_depth = 0i32;
            let bytes = s.as_bytes();
            let op_bytes = op_str.as_bytes();
            for i in 0..=s.len().saturating_sub(op_str.len()) {
                let ch = bytes[i] as char;
                if ch == '(' { paren_depth += 1; }
                else if ch == ')' { paren_depth -= 1; }
                if paren_depth == 0 && bytes[i..].starts_with(op_bytes) {
                    let left = s[..i].trim();
                    let right = s[i + op_str.len()..].trim();
                    if !left.is_empty() && !right.is_empty() {
                        if let Some(left_expr) = self.parse(left) {
                            if let Some(right_expr) = self.parse(right) {
                                return Some(Expr::BinaryOp {
                                    op,
                                    left: Box::new(left_expr),
                                    right: Box::new(right_expr),
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_unary_op(&self, s: &str) -> Option<Expr> {
        let upper = s.to_uppercase();
        if upper.starts_with("NOT (") && s.ends_with(')') {
            let inner = &s[4..s.len() - 1];
            if let Some(inner_expr) = self.parse(inner) {
                return Some(Expr::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(inner_expr),
                });
            }
        }
        if s.starts_with('-') {
            let rest = s[1..].trim();
            if let Some(inner_expr) = self.parse(rest) {
                return Some(Expr::UnaryOp {
                    op: UnaryOperator::Neg,
                    expr: Box::new(inner_expr),
                });
            }
        }
        None
    }

    fn parse_case_when(&self, s: &str) -> Option<Expr> {
        let s = s.trim();
        let upper = s.to_uppercase();
        if !upper.starts_with("CASE ") || !upper.ends_with(" END") {
            return None;
        }

        let inner = &s[5..s.len() - 4].trim();
        let (cases, else_expr) = self.parse_case_when_parts(inner)?;

        Some(Expr::CaseWhen {
            cases,
            else_expr,
        })
    }

    fn parse_case_when_parts(&self, inner: &str) -> Option<(Vec<WhenThen>, Option<Box<Expr>>)> {
        let mut cases = Vec::new();
        let mut remaining = inner;
        let mut else_expr = None;

        loop {
            let upper = remaining.trim().to_uppercase();
            if upper.starts_with("ELSE ") {
                let else_str = &remaining[5..].trim();
                if else_str.ends_with(" END") {
                    else_expr = Some(Box::new(self.parse(&else_str[..else_str.len() - 4].trim())?));
                } else {
                    else_expr = Some(Box::new(self.parse(else_str)?));
                }
                break;
            }
            if !upper.starts_with("WHEN ") {
                break;
            }

            let after_when = &remaining[5..].trim();
            if let Some(then_pos) = Self::find_keyword_position(after_when, " THEN ") {
                let when_cond = &after_when[..then_pos].trim();
                let after_then = &after_when[then_pos + 6..].trim();
                let (then_expr, next_part) = self.parse_then_and_next(after_then)?;
                let when_expr = self.parse(when_cond)?;
                cases.push(WhenThen {
                    when: when_expr,
                    then: then_expr,
                });
                remaining = next_part;
            } else {
                break;
            }
        }

        Some((cases, else_expr))
    }

    fn find_keyword_position(s: &str, keyword: &str) -> Option<usize> {
        let upper = s.to_uppercase();
        let key_upper = keyword.to_uppercase();
        let mut depth: i32 = 0;
        let mut in_string = false;
        for (i, ch) in s.chars().enumerate() {
            match ch {
                '\'' => in_string = !in_string,
                '(' if !in_string => depth += 1,
                ')' if !in_string => depth = depth.saturating_sub(1),
                _ if !in_string && depth == 0 => {
                    if upper[i..].starts_with(&key_upper) {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn parse_then_and_next<'a>(&self, s: &'a str) -> Option<(Expr, &'a str)> {
        let upper = s.to_uppercase();

        if upper.starts_with("WHEN ") {
            return None;
        }
        if upper.starts_with("ELSE ") {
            return None;
        }

        if let Some(else_pos) = Self::find_keyword_position(s, " ELSE ") {
            let then_expr_str = &s[..else_pos].trim();
            let next_part = &s[else_pos + 6..].trim();
            let then_expr = self.parse(then_expr_str)?;
            return Some((then_expr, next_part));
        }

        if let Some(end_pos) = s.to_uppercase().find(" END") {
            let then_expr_str = &s[..end_pos].trim();
            let then_expr = self.parse(then_expr_str)?;
            return Some((then_expr, &s[end_pos..]));
        }

        let then_expr = self.parse(s.trim())?;
        Some((then_expr, ""))
    }

    fn parse_function_call(&self, s: &str) -> Option<Expr> {
        if !s.ends_with(')') {
            return None;
        }

        let open_paren = s.find('(')?;
        let name = s[..open_paren].trim().to_uppercase();
        if name.is_empty() || !self.is_identifier(&name) {
            return None;
        }

        let args_str = &s[open_paren + 1..s.len() - 1];
        let split_args = self.split_args(args_str);

        let mut distinct = false;
        let mut filtered_args: Vec<String> = split_args;
        if !filtered_args.is_empty() {
            let first = filtered_args[0].trim().to_uppercase();
            if first == "DISTINCT" {
                distinct = true;
                filtered_args.remove(0);
            } else if first.starts_with("DISTINCT ") {
                distinct = true;
                filtered_args[0] = filtered_args[0].trim()[9..].trim().to_string();
            }
        }

        let args_exprs: Vec<Expr> = filtered_args.iter()
            .filter_map(|arg| self.parse(arg))
            .collect();

        if args_exprs.len() == filtered_args.len() {
            Some(Expr::FunctionCall(FunctionCall {
                name: if distinct { format!("{}_distinct", name.to_lowercase()) } else { name.to_lowercase() },
                args: args_exprs,
                distinct: false,
            }))
        } else {
            None
        }
    }

    fn split_args(&self, args_str: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut current = String::new();

        for ch in args_str.chars() {
            match ch {
                '\'' => {
                    in_string = !in_string;
                    current.push(ch);
                }
                '(' if !in_string => {
                    depth += 1;
                    current.push(ch);
                }
                ')' if !in_string => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 && !in_string => {
                    args.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        args
    }

    fn is_identifier(&self, s: &str) -> bool {
        let s = s.trim();
        if s.is_empty() {
            return false;
        }
        let first = s.chars().next().unwrap();
        first.is_alphabetic() || first == '_'
    }

    fn is_literal(&self, s: &str) -> bool {
        let s = s.trim();
        if s.parse::<f64>().is_ok() {
            return true;
        }
        if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
            return true;
        }
        let upper = s.to_uppercase();
        matches!(upper.as_str(), "NULL" | "TRUE" | "FALSE")
    }

    fn parse_literal(&self, s: &str) -> Option<ScalarValue> {
        let s = s.trim();

        let upper = s.to_uppercase();
        match upper.as_str() {
            "NULL" => return Some(ScalarValue::Null),
            "TRUE" => return Some(ScalarValue::Boolean(true)),
            "FALSE" => return Some(ScalarValue::Boolean(false)),
            _ => {}
        }

        if let Ok(n) = s.parse::<i64>() {
            return Some(ScalarValue::Int64(n));
        }
        if let Ok(f) = s.parse::<f64>() {
            return Some(ScalarValue::Float64(f));
        }

        if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
            let content = &s[1..s.len() - 1];
            return Some(ScalarValue::String(content.to_string()));
        }

        None
    }

    /// Parse expression string and evaluate against block.
    pub fn evaluate(&self, expr_str: &str, block: &Block) -> Option<Vector> {
        let expr = self.parse(expr_str)?;
        Some(self.evaluator.evaluate(&expr, block))
    }
}

impl Default for ExprStringParser {
    fn default() -> Self {
        Self::new()
    }
}