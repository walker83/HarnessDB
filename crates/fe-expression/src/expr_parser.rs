use crate::expr::{BinaryOperator, Expr, UnaryOperator, ColumnRef, FunctionCall};
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

        // Handle literals
        self.parse_literal(trimmed).map(Expr::Literal)
    }

    fn parse_binary_op(&self, s: &str) -> Option<Expr> {
        // Check for parentheses to avoid misparsing inside function args
        let paren_count = s.chars().filter(|&c| c == '(' || c == ')').count();
        if paren_count > 0 {
            return None;
        }

        // Try operators from lowest to highest precedence
        // OR, AND, LIKE/NOT LIKE, comparisons, then arithmetic (+ -, * / %)
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
            if let Some(pos) = s.find(op_str) {
                let left = s[..pos].trim();
                let right = s[pos + op_str.len()..].trim();
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
        let args = self.split_args(args_str);

        let args_exprs: Vec<Expr> = args.iter()
            .filter_map(|arg| self.parse(arg))
            .collect();

        if args_exprs.len() == args.len() {
            Some(Expr::FunctionCall(FunctionCall {
                name: name.to_lowercase(),
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