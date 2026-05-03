use types::{Bitmap, Block, DataType, ScalarValue, Vector};
use crate::expr::{BinaryOperator, Expr, UnaryOperator};
use crate::functions::FunctionRegistry;

pub struct ExprEvaluator {
    functions: FunctionRegistry,
}

impl ExprEvaluator {
    pub fn new() -> Self { Self { functions: FunctionRegistry::new() } }

    pub fn evaluate(&self, expr: &Expr, block: &Block) -> Vector {
        match expr {
            Expr::ColumnRef(col) => block.column(col.index).cloned().unwrap_or_else(|| Vector::Null(types::vector::NullVector::new(0))),
            Expr::Literal(val) => Vector::from_scalar(val, block.num_rows()),
            Expr::BinaryOp { op, left, right } => {
                let lv = self.evaluate(left, block);
                let rv = self.evaluate(right, block);
                self.eval_binary(*op, &lv, &rv)
            }
            Expr::UnaryOp { op, expr: inner } => {
                let v = self.evaluate(inner, block);
                self.eval_unary(*op, &v)
            }
            Expr::Cast { expr: inner, target_type } => {
                let v = self.evaluate(inner, block);
                self.eval_cast(&v, target_type)
            }
            Expr::IsNull { expr: inner, negated } => {
                let v = self.evaluate(inner, block);
                let len = v.len();
                let data: Vec<bool> = (0..len).map(|i| {
                    let null = v.scalar_at(i) == ScalarValue::Null;
                    if *negated { !null } else { null }
                }).collect();
                bool_vec(data)
            }
            Expr::FunctionCall(call) => {
                let args: Vec<Vector> = call.args.iter().map(|a| self.evaluate(a, block)).collect();
                self.functions.call(&call.name, &args)
            }
            Expr::InList { expr: inner, list, negated } => {
                let v = self.evaluate(inner, block);
                let len = v.len();
                let mut result = vec![false; len];
                for e in list {
                    let ev = self.evaluate(e, block);
                    for i in 0..len { if !result[i] && v.scalar_at(i) == ev.scalar_at(i) { result[i] = true; } }
                }
                if *negated { result.iter_mut().for_each(|b| *b = !*b); }
                bool_vec(result)
            }
            Expr::Between { expr: inner, low, high, negated } => {
                let v = self.evaluate(inner, block);
                let lv = self.evaluate(low, block);
                let hv = self.evaluate(high, block);
                let r1 = self.eval_binary(BinaryOperator::Ge, &v, &lv);
                let r2 = self.eval_binary(BinaryOperator::Le, &v, &hv);
                let c = self.eval_binary(BinaryOperator::And, &r1, &r2);
                if *negated { self.eval_unary(UnaryOperator::Not, &c) } else { c }
            }
            Expr::Like { expr: inner, pattern, negated } => {
                let v = self.evaluate(inner, block);
                let pv = self.evaluate(pattern, block);
                let len = v.len();
                let r: Vec<bool> = (0..len).map(|i| {
                    let s = match v.scalar_at(i) { ScalarValue::String(s) => s, _ => return false };
                    let p = match pv.scalar_at(i) { ScalarValue::String(p) => p, _ => return false };
                    match_like(&s, &p)
                }).collect();
                let res = bool_vec(r);
                if *negated { self.eval_unary(UnaryOperator::Not, &res) } else { res }
            }
            Expr::CaseWhen { cases, else_expr } => {
                let len = block.num_rows();
                let mut result: Vec<ScalarValue> = vec![ScalarValue::Null; len];
                let mut matched = Bitmap::all_set(len);
                for case in cases {
                    let cond = self.evaluate(&case.when, block);
                    let val = self.evaluate(&case.then, block);
                    if let Vector::Boolean(bv) = &cond {
                        for i in 0..len {
                            if matched.get(i) && bv.get(i).unwrap_or(false) {
                                result[i] = val.scalar_at(i);
                                matched.set(i, false);
                            }
                        }
                    }
                }
                if let Some(ee) = else_expr {
                    let val = self.evaluate(ee, block);
                    for i in 0..len { if matched.get(i) { result[i] = val.scalar_at(i); } }
                }
                result.into_iter().next().map(|v| Vector::from_scalar(&v, 1)).unwrap_or_else(|| bool_vec(vec![]))
            }
            Expr::Exists { .. } => bool_vec(vec![false; block.num_rows()]),
        }
    }

    fn eval_binary(&self, op: BinaryOperator, left: &Vector, right: &Vector) -> Vector {
        match op {
            BinaryOperator::Add => arith(left, right, |a, b| a + b, |a, b| a.wrapping_add(b)),
            BinaryOperator::Subtract => arith(left, right, |a, b| a - b, |a, b| a.wrapping_sub(b)),
            BinaryOperator::Multiply => arith(left, right, |a, b| a * b, |a, b| a.wrapping_mul(b)),
            BinaryOperator::Divide => arith(left, right, |a, b| a / b, |a, b| a / b),
            BinaryOperator::Modulo => arith(left, right, |a, b| a % b, |a, b| a % b),
            BinaryOperator::Eq => cmp(left, right, |a, b| a == b, |a, b| a == b),
            BinaryOperator::Ne => cmp(left, right, |a, b| a != b, |a, b| a != b),
            BinaryOperator::Lt => cmp(left, right, |a, b| a < b, |a, b| a < b),
            BinaryOperator::Le => cmp(left, right, |a, b| a <= b, |a, b| a <= b),
            BinaryOperator::Gt => cmp(left, right, |a, b| a > b, |a, b| a > b),
            BinaryOperator::Ge => cmp(left, right, |a, b| a >= b, |a, b| a >= b),
            BinaryOperator::And => logic_and(left, right),
            BinaryOperator::Or => logic_or(left, right),
            _ => bool_vec(vec![false; left.len()]),
        }
    }

    fn eval_unary(&self, op: UnaryOperator, v: &Vector) -> Vector {
        match op {
            UnaryOperator::Not => match v {
                Vector::Boolean(bv) => bool_vec(bv.data().iter().map(|b| !b).collect()),
                _ => bool_vec(vec![false; v.len()]),
            },
            UnaryOperator::Neg => match v {
                Vector::Int64(iv) => int64_vec(iv.data().iter().map(|n| n.wrapping_neg()).collect()),
                Vector::Float64(fv) => float64_vec(fv.data().iter().map(|n| -n).collect()),
                _ => v.clone(),
            },
            _ => v.clone(),
        }
    }

    fn eval_cast(&self, v: &Vector, target: &DataType) -> Vector {
        match target {
            DataType::Int64 => match v {
                Vector::Float64(fv) => int64_vec(fv.data().iter().map(|n| *n as i64).collect()),
                Vector::String(sv) => int64_vec((0..sv.len()).map(|i| sv.get(i).and_then(|s| s.parse().ok()).unwrap_or(0)).collect()),
                _ => v.clone(),
            },
            DataType::Float64 => match v {
                Vector::Int64(iv) => float64_vec(iv.data().iter().map(|n| *n as f64).collect()),
                Vector::String(sv) => float64_vec((0..sv.len()).map(|i| sv.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0)).collect()),
                _ => v.clone(),
            },
            DataType::String => match v {
                Vector::Int64(iv) => string_vec(iv.data().iter().map(|n| Some(n.to_string())).collect()),
                Vector::Float64(fv) => string_vec(fv.data().iter().map(|n| Some(n.to_string())).collect()),
                _ => v.clone(),
            },
            _ => v.clone(),
        }
    }
}

fn arith(left: &Vector, right: &Vector, ff: fn(f64, f64) -> f64, fi: fn(i64, i64) -> i64) -> Vector {
    match (left, right) {
        (Vector::Float64(l), Vector::Float64(r)) => float64_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| ff(a, b)).collect()),
        (Vector::Int64(l), Vector::Int64(r)) => int64_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| fi(a, b)).collect()),
        (Vector::Int32(l), Vector::Int32(r)) => Vector::Int32(types::vector::Int32Vector::from_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| fi(a as i64, b as i64) as i32).collect())),
        _ => float64_vec(vec![0.0; left.len()]),
    }
}

fn cmp<F, G>(left: &Vector, right: &Vector, fi: F, ff: G) -> Vector
where F: Fn(i64, i64) -> bool, G: Fn(f64, f64) -> bool {
    match (left, right) {
        (Vector::Int64(l), Vector::Int64(r)) => bool_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| fi(a, b)).collect()),
        (Vector::Float64(l), Vector::Float64(r)) => bool_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| ff(a, b)).collect()),
        (Vector::String(l), Vector::String(r)) => bool_vec((0..l.len()).map(|i| l.get(i).unwrap_or("") == r.get(i).unwrap_or("")).collect()),
        _ => bool_vec(vec![false; left.len()]),
    }
}

fn logic_and(l: &Vector, r: &Vector) -> Vector {
    match (l, r) {
        (Vector::Boolean(l), Vector::Boolean(r)) => bool_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| a && b).collect()),
        _ => bool_vec(vec![false; l.len()]),
    }
}
fn logic_or(l: &Vector, r: &Vector) -> Vector {
    match (l, r) {
        (Vector::Boolean(l), Vector::Boolean(r)) => bool_vec(l.data().iter().zip(r.data()).map(|(&a, &b)| a || b).collect()),
        _ => bool_vec(vec![false; l.len()]),
    }
}

fn bool_vec(d: Vec<bool>) -> Vector { Vector::Boolean(types::vector::BooleanVector::from_vec(d)) }
fn int64_vec(d: Vec<i64>) -> Vector { Vector::Int64(types::vector::Int64Vector::from_vec(d)) }
fn float64_vec(d: Vec<f64>) -> Vector { Vector::Float64(types::vector::Float64Vector::from_vec(d)) }
fn string_vec(d: Vec<Option<String>>) -> Vector { Vector::String(types::vector::StringVector::from_option_vec(d)) }

fn match_like(s: &str, pattern: &str) -> bool {
    let ss: Vec<char> = s.chars().collect();
    let pp: Vec<char> = pattern.chars().collect();
    like_dp(&ss, &pp, 0, 0)
}
fn like_dp(s: &[char], p: &[char], si: usize, pi: usize) -> bool {
    if pi == p.len() { return si == s.len(); }
    if p[pi] == '%' { (si..=s.len()).any(|i| like_dp(s, p, i, pi + 1)) }
    else if si < s.len() && (p[pi] == '_' || p[pi] == s[si]) { like_dp(s, p, si + 1, pi + 1) }
    else { false }
}

impl Default for ExprEvaluator { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use types::{Field, Schema};
    use crate::expr::{ColumnRef, FunctionCall};

    fn make_block() -> Block {
        let schema = Schema::new(vec![
            Field::new("a", DataType::Int64, false),
            Field::new("b", DataType::Int64, false),
            Field::new("x", DataType::Float64, false),
            Field::new("s", DataType::String, false),
        ]);
        let cols = vec![
            Vector::Int64(types::vector::Int64Vector::from_vec(vec![10, 20, 30, 40, 50])),
            Vector::Int64(types::vector::Int64Vector::from_vec(vec![1, 2, 3, 4, 5])),
            Vector::Float64(types::vector::Float64Vector::from_vec(vec![1.5, 2.5, 3.5, 4.5, 5.5])),
            Vector::String(types::vector::StringVector::from_vec(vec!["hello", "world", "foo", "bar", "baz"])),
        ];
        Block::new(schema, cols)
    }

    // ---- Arithmetic ----

    #[test]
    fn test_int64_add() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Add, Expr::column(0, "a"), Expr::column(1, "b"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[11, 22, 33, 44, 55]),
            _ => panic!("expected Int64 vector"),
        }
    }

    #[test]
    fn test_int64_subtract() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Subtract, Expr::column(0, "a"), Expr::column(1, "b"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[9, 18, 27, 36, 45]),
            _ => panic!("expected Int64 vector"),
        }
    }

    #[test]
    fn test_int64_multiply() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Multiply, Expr::column(0, "a"), Expr::column(1, "b"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[10, 40, 90, 160, 250]),
            _ => panic!("expected Int64 vector"),
        }
    }

    #[test]
    fn test_float64_divide() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Divide, Expr::column(2, "x"), Expr::literal(ScalarValue::Float64(2.0)));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Float64(v) => {
                let d = v.data();
                assert!((d[0] - 0.75).abs() < 0.001);
                assert!((d[4] - 2.75).abs() < 0.001);
            }
            _ => panic!("expected Float64 vector"),
        }
    }

    #[test]
    fn test_literal_arithmetic() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Add, Expr::literal(ScalarValue::Int64(100)), Expr::column(0, "a"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[110, 120, 130, 140, 150]),
            _ => panic!("expected Int64 vector"),
        }
    }

    // ---- Comparison ----

    #[test]
    fn test_int64_eq() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Eq, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(30)));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, false, true, false, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_int64_gt() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Gt, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(25)));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, false, true, true, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_float64_comparison() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Le, Expr::column(2, "x"), Expr::literal(ScalarValue::Float64(3.5)));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, true, true, false, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- Logical ----

    #[test]
    fn test_and() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let gt = Expr::binary(BinaryOperator::Gt, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(15)));
        let lt = Expr::binary(BinaryOperator::Lt, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(45)));
        let expr = Expr::binary(BinaryOperator::And, gt, lt);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, true, true, true, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_or() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let eq10 = Expr::binary(BinaryOperator::Eq, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(10)));
        let eq50 = Expr::binary(BinaryOperator::Eq, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(50)));
        let expr = Expr::binary(BinaryOperator::Or, eq10, eq50);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, false, false, false, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_not() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let eq = Expr::binary(BinaryOperator::Eq, Expr::column(0, "a"), Expr::literal(ScalarValue::Int64(30)));
        let expr = Expr::unary(UnaryOperator::Not, eq);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, true, false, true, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- IS NULL / IS NOT NULL ----

    #[test]
    fn test_is_null() {
        let ev = ExprEvaluator::new();
        let schema = Schema::new(vec![Field::new("v", DataType::Int64, true)]);
        let block = Block::new(schema, vec![
            Vector::Int64(types::vector::Int64Vector::from_nullable_vec(vec![Some(1), None, Some(3)])),
        ]);
        let expr = Expr::is_null(Expr::column(0, "v"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, true, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_is_not_null() {
        let ev = ExprEvaluator::new();
        let schema = Schema::new(vec![Field::new("v", DataType::Int64, true)]);
        let block = Block::new(schema, vec![
            Vector::Int64(types::vector::Int64Vector::from_nullable_vec(vec![Some(1), None, Some(3)])),
        ]);
        let expr = Expr::is_not_null(Expr::column(0, "v"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, false, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- IN list ----

    #[test]
    fn test_in_list() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::in_list(
            Expr::column(1, "b"),
            vec![Expr::literal(ScalarValue::Int64(1)), Expr::literal(ScalarValue::Int64(3)), Expr::literal(ScalarValue::Int64(5))],
            false,
        );
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, false, true, false, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_not_in_list() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::in_list(
            Expr::column(1, "b"),
            vec![Expr::literal(ScalarValue::Int64(1)), Expr::literal(ScalarValue::Int64(3))],
            true,
        );
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, true, false, true, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- BETWEEN ----

    #[test]
    fn test_between() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::between(
            Expr::column(0, "a"),
            Expr::literal(ScalarValue::Int64(20)),
            Expr::literal(ScalarValue::Int64(40)),
            false,
        );
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, true, true, true, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- LIKE ----

    #[test]
    fn test_like_pattern() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::like(
            Expr::column(3, "s"),
            Expr::literal(ScalarValue::String("h%".to_string())),
            false,
        );
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[true, false, false, false, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    #[test]
    fn test_like_underscore() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::like(
            Expr::column(3, "s"),
            Expr::literal(ScalarValue::String("___".to_string())),
            false,
        );
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, false, true, true, true]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- CAST ----

    #[test]
    fn test_cast_int64_to_float64() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::cast(Expr::column(0, "a"), DataType::Float64);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Float64(v) => assert_eq!(v.data(), &[10.0, 20.0, 30.0, 40.0, 50.0]),
            _ => panic!("expected Float64 vector"),
        }
    }

    #[test]
    fn test_cast_float64_to_int64() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::cast(Expr::column(2, "x"), DataType::Int64);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[1, 2, 3, 4, 5]),
            _ => panic!("expected Int64 vector"),
        }
    }

    #[test]
    fn test_cast_int64_to_string() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::cast(Expr::column(0, "a"), DataType::String);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::String(v) => {
                assert_eq!(v.get(0), Some("10"));
                assert_eq!(v.get(4), Some("50"));
            }
            _ => panic!("expected String vector"),
        }
    }

    // ---- Unary negate ----

    #[test]
    fn test_negate_int64() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::unary(UnaryOperator::Neg, Expr::column(0, "a"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[-10, -20, -30, -40, -50]),
            _ => panic!("expected Int64 vector"),
        }
    }

    // ---- Functions ----

    #[test]
    fn test_function_abs() {
        let ev = ExprEvaluator::new();
        let schema = Schema::new(vec![Field::new("v", DataType::Int64, false)]);
        let block = Block::new(schema, vec![
            Vector::Int64(types::vector::Int64Vector::from_vec(vec![-5, 10, -3, 0, 7])),
        ]);
        let expr = Expr::call("abs", vec![Expr::column(0, "v")]);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[5, 10, 3, 0, 7]),
            _ => panic!("expected Int64 vector, got {:?}", result),
        }
    }

    #[test]
    fn test_function_upper() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::call("upper", vec![Expr::column(3, "s")]);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::String(v) => {
                assert_eq!(v.get(0), Some("HELLO"));
                assert_eq!(v.get(1), Some("WORLD"));
            }
            _ => panic!("expected String vector"),
        }
    }

    #[test]
    fn test_function_length() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::call("length", vec![Expr::column(3, "s")]);
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[5, 5, 3, 3, 3]),
            _ => panic!("expected Int64 vector"),
        }
    }

    // ---- String comparison ----

    #[test]
    fn test_string_eq() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Eq, Expr::column(3, "s"), Expr::literal(ScalarValue::String("foo".to_string())));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Boolean(v) => assert_eq!(v.data(), &[false, false, true, false, false]),
            _ => panic!("expected Boolean vector"),
        }
    }

    // ---- Modulo ----

    #[test]
    fn test_int64_modulo() {
        let ev = ExprEvaluator::new();
        let block = make_block();
        let expr = Expr::binary(BinaryOperator::Modulo, Expr::column(0, "a"), Expr::column(1, "b"));
        let result = ev.evaluate(&expr, &block);
        match result {
            Vector::Int64(v) => assert_eq!(v.data(), &[0, 0, 0, 0, 0]),
            _ => panic!("expected Int64 vector"),
        }
    }
}
