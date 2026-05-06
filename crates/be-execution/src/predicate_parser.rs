use be_storage::index::{ColumnPredicate, PredicateOp};
use fe_sql_parser::ast::{Expr, LiteralValue};
use types::{Block, DataType, Field, ScalarValue, Schema, Vector};
use types::vector::{
    BooleanVector, DateVector, DateTimeVector, Float32Vector, Float64Vector, Int128Vector,
    Int16Vector, Int32Vector, Int64Vector, Int8Vector, JsonVector, StringVector,
};

/// Parse a predicate string (from FE `expr_to_string`) into a list of ColumnPredicates.
///
/// Handles AND by splitting on ` AND ` and parsing each part independently.
/// OR is not supported at the storage predicate level — callers should handle it
/// by evaluating each OR branch separately and unioning the bitmaps.
pub fn parse_predicates(pred_str: &str) -> Vec<ColumnPredicate> {
    let trimmed = pred_str.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    // Split on top-level AND (not inside quotes or parens)
    let parts = split_on_and(trimmed);
    parts.iter()
        .filter_map(|p| parse_single_predicate(p.trim()))
        .collect()
}

/// Split a predicate string on ` AND ` while respecting quoted strings, parentheses, and BETWEEN.
fn split_on_and(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let mut paren_depth = 0u32;
    let mut in_between = false;
    let chars: Vec<char> = s.chars().collect();
    let bytes: Vec<usize> = chars.iter().map(|c| c.len_utf8()).collect();
    let mut byte_pos = 0usize;

    for i in 0..chars.len() {
        let c = chars[i];
        if c == '\'' {
            in_quote = !in_quote;
        } else if !in_quote {
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth = paren_depth.saturating_sub(1);
            } else if paren_depth == 0 && c == 'B' {
                // Check for BETWEEN keyword
                let remaining = &s[byte_pos..];
                if remaining.starts_with("BETWEEN") {
                    let before_ok = byte_pos == 0 || s.as_bytes()[byte_pos - 1] == b' ';
                    let after_pos = byte_pos + 7;
                    let after_ok = after_pos >= s.len() || s.as_bytes()[after_pos] == b' ';
                    if before_ok && after_ok {
                        in_between = true;
                    }
                }
            } else if paren_depth == 0 && c == 'A' && !in_between {
                // Check for " AND " starting at position i
                let remaining = &s[byte_pos..];
                if remaining.starts_with("AND") && i + 3 <= chars.len() {
                    // Check space before and after
                    let space_before = i > 0 && chars[i - 1] == ' ';
                    let space_after = i + 3 < chars.len() && chars[i + 3] == ' ';
                    if space_before && space_after {
                        let end_byte = byte_pos - 1; // exclude the leading space
                        if end_byte > start {
                            parts.push(&s[start..end_byte]);
                        }
                        start = byte_pos + 4; // skip "AND "
                    }
                }
            }
            // Reset in_between after we see " AND " inside a BETWEEN (i.e., the BETWEEN's own AND)
            if in_between && c == 'A' {
                let remaining = &s[byte_pos..];
                if remaining.starts_with("AND") {
                    let space_before = byte_pos > 0 && s.as_bytes()[byte_pos - 1] == b' ';
                    let after_pos = byte_pos + 3;
                    let space_after = after_pos >= s.len() || s.as_bytes()[after_pos] == b' ';
                    if space_before && space_after {
                        in_between = false;
                    }
                }
            }
        }
        byte_pos += bytes[i];
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

/// Parse a single predicate (no AND/OR at the top level).
fn parse_single_predicate(s: &str) -> Option<ColumnPredicate> {
    let s = s.trim();

    // IS NOT NULL
    if let Some(col) = s.strip_suffix("IS NOT NULL") {
        return Some(ColumnPredicate::new_is_not_null(col.trim().to_string()));
    }

    // IS NULL
    if let Some(col) = s.strip_suffix("IS NULL") {
        return Some(ColumnPredicate::new_is_null(col.trim().to_string()));
    }

    // NOT BETWEEN ... AND ...
    if let Some(idx) = find_keyword(s, "NOT BETWEEN") {
        let col = s[..idx].trim();
        let rest = &s[idx + "NOT BETWEEN".len()..].trim();
        return parse_between(col, rest, true);
    }

    // BETWEEN ... AND ...
    if let Some(idx) = find_keyword(s, "BETWEEN") {
        let col = s[..idx].trim();
        let rest = &s[idx + "BETWEEN".len()..].trim();
        return parse_between(col, rest, false);
    }

    // NOT IN (...)
    if let Some(idx) = find_keyword(s, "NOT IN") {
        let col = s[..idx].trim();
        let rest = &s[idx + "NOT IN".len()..].trim();
        return parse_in_list(col, rest, true);
    }

    // IN (...)
    if let Some(idx) = find_keyword(s, "IN") {
        let col = s[..idx].trim();
        let rest = &s[idx + "IN".len()..].trim();
        // Make sure this isn't part of "NOT IN" or "BETWEEN"
        if !rest.starts_with('(') {
            // Not actually an IN clause, fall through to comparison
        } else {
            return parse_in_list(col, rest, false);
        }
    }

    // NOT LIKE
    if let Some(idx) = find_keyword(s, "NOT LIKE") {
        let col = s[..idx].trim();
        let pattern = s[idx + "NOT LIKE".len()..].trim();
        let pattern_str = strip_quotes(pattern);
        return Some(ColumnPredicate::new_like(col.to_string(), pattern_str));
    }

    // LIKE
    if let Some(idx) = find_keyword(s, "LIKE") {
        let col = s[..idx].trim();
        let pattern = s[idx + "LIKE".len()..].trim();
        let pattern_str = strip_quotes(pattern);
        return Some(ColumnPredicate::new_like(col.to_string(), pattern_str));
    }

    // Comparison operators: try longest match first
    for (op_str, op) in [
        ("!=", PredicateOp::NotEq),
        ("<=", PredicateOp::Le),
        (">=", PredicateOp::Ge),
        ("=", PredicateOp::Eq),
        ("<", PredicateOp::Lt),
        (">", PredicateOp::Gt),
    ] {
        if let Some(pos) = find_op(s, op_str) {
            let col = s[..pos].trim();
            let val = s[pos + op_str.len()..].trim();
            return Some(ColumnPredicate::new(
                col.to_string(),
                op,
                parse_value_string(val),
            ));
        }
    }

    None
}

/// Find a keyword in the string that is surrounded by spaces (or at start/end).
fn find_keyword(s: &str, keyword: &str) -> Option<usize> {
    let lower = s.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let mut start = 0;
    while let Some(pos) = lower[start..].find(&kw_upper) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0 || s.as_bytes()[abs_pos - 1] == b' ';
        let after_pos = abs_pos + kw_upper.len();
        let after_ok = after_pos >= s.len() || s.as_bytes()[after_pos] == b' ';
        if before_ok && after_ok {
            return Some(abs_pos);
        }
        start = abs_pos + 1;
    }
    None
}

/// Find a comparison operator in the string, respecting quoted strings.
fn find_op(s: &str, op: &str) -> Option<usize> {
    let mut in_quote = false;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            in_quote = !in_quote;
        } else if !in_quote && s[i..].starts_with(op) {
            // Make sure we're not matching part of a longer operator
            // (e.g., don't match "=" inside "!=" or "<=")
            if op == "=" && i > 0 && (bytes[i - 1] == b'!' || bytes[i - 1] == b'<' || bytes[i - 1] == b'>') {
                i += 1;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_between(col: &str, rest: &str, negated: bool) -> Option<ColumnPredicate> {
    // rest is like "1 AND 10"
    // Split on AND to get low and high
    let and_pos = rest.find(" AND ")?;
    let low_str = rest[..and_pos].trim();
    let high_str = rest[and_pos + 5..].trim();
    let low = parse_value_string(low_str);
    let high = parse_value_string(high_str);

    if negated {
        // NOT BETWEEN: we can't express this as a single ColumnPredicate,
        // so return None and let the caller handle it
        return None;
    }

    Some(ColumnPredicate::new_between(col.to_string(), low, high))
}

fn parse_in_list(col: &str, rest: &str, negated: bool) -> Option<ColumnPredicate> {
    // rest is like "(1, 2, 3)"
    let inner = rest.trim().strip_prefix('(')?.strip_suffix(')')?;
    let values: Vec<ScalarValue> = inner.split(',')
        .map(|v| parse_value_string(v.trim()))
        .collect();

    if negated {
        let mut pred = ColumnPredicate::new_in(col.to_string(), values);
        pred.op = PredicateOp::NotIn;
        Some(pred)
    } else {
        Some(ColumnPredicate::new_in(col.to_string(), values))
    }
}

/// Parse a literal value string into a ScalarValue.
///
/// Matches the format produced by `literal_to_string()` in fe-sql-planner:
/// - `"NULL"` → Null
/// - `"true"` / `"false"` → Boolean
/// - `"123"` → Int64
/// - `"1.5"` → Float64
/// - `"'hello'"` → String
/// - `"DATE '2024-01-01'"` → Date
pub fn parse_value_string(s: &str) -> ScalarValue {
    let s = s.trim();

    if s.eq_ignore_ascii_case("NULL") {
        return ScalarValue::Null;
    }
    if s.eq_ignore_ascii_case("true") {
        return ScalarValue::Boolean(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return ScalarValue::Boolean(false);
    }

    // DATE '2024-01-01'
    if s.starts_with("DATE ") || s.starts_with("date ") {
        let date_str = strip_quotes(s[5..].trim());
        // Parse YYYY-MM-DD to days since epoch (simplified)
        if let Ok(days) = date_str.replace('-', "").parse::<i32>() {
            return ScalarValue::Date(days);
        }
        return ScalarValue::Date(0);
    }

    // Single-quoted string: 'hello'
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return ScalarValue::String(s[1..s.len() - 1].to_string());
    }

    // Negative number
    if s.starts_with('-') {
        if let Ok(n) = s.parse::<i64>() {
            return ScalarValue::Int64(n);
        }
        if let Ok(f) = s.parse::<f64>() {
            return ScalarValue::Float64(f);
        }
    }

    // Integer (try i64 first, then i32)
    if let Ok(n) = s.parse::<i64>() {
        return ScalarValue::Int64(n);
    }

    // Float
    if let Ok(f) = s.parse::<f64>() {
        return ScalarValue::Float64(f);
    }

    // Fallback: treat as string
    ScalarValue::String(s.to_string())
}

/// Strip surrounding single quotes from a string.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse a SET clause value string with type coercion to match the target column type.
pub fn parse_set_value(value_str: &str, target_type: &DataType) -> ScalarValue {
    let raw = parse_value_string(value_str);

    match target_type {
        DataType::Int8 => coerce_to_int8(&raw),
        DataType::Int16 => coerce_to_int16(&raw),
        DataType::Int32 => coerce_to_int32(&raw),
        DataType::Int64 => coerce_to_int64(&raw),
        DataType::Int128 => coerce_to_int128(&raw),
        DataType::Float32 => coerce_to_float32(&raw),
        DataType::Float64 => coerce_to_float64(&raw),
        DataType::String => coerce_to_string(&raw),
        DataType::Boolean => coerce_to_boolean(&raw),
        DataType::Date => match &raw {
            ScalarValue::Date(_) => raw,
            ScalarValue::Int64(n) => ScalarValue::Date(*n as i32),
            ScalarValue::String(s) => s.replace('-', "").parse::<i32>()
                .map(ScalarValue::Date)
                .unwrap_or(raw),
            _ => raw,
        },
        DataType::DateTime => match &raw {
            ScalarValue::DateTime(_) => raw,
            ScalarValue::Int64(_) => raw,
            _ => raw,
        },
        _ => raw,
    }
}

fn coerce_to_int64(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Int64(_) => raw.clone(),
        ScalarValue::Int32(n) => ScalarValue::Int64(*n as i64),
        ScalarValue::Int16(n) => ScalarValue::Int64(*n as i64),
        ScalarValue::Int8(n) => ScalarValue::Int64(*n as i64),
        ScalarValue::Float64(f) => ScalarValue::Int64(*f as i64),
        ScalarValue::Float32(f) => ScalarValue::Int64(*f as i64),
        ScalarValue::String(s) => s.parse::<i64>()
            .map(ScalarValue::Int64)
            .unwrap_or_else(|_| ScalarValue::Int64(0)),
        _ => ScalarValue::Int64(0),
    }
}

fn coerce_to_int32(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Int32(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Int32(*n as i32),
        ScalarValue::Int16(n) => ScalarValue::Int32(*n as i32),
        ScalarValue::Int8(n) => ScalarValue::Int32(*n as i32),
        ScalarValue::Float64(f) => ScalarValue::Int32(*f as i32),
        ScalarValue::String(s) => s.parse::<i32>()
            .map(ScalarValue::Int32)
            .unwrap_or_else(|_| ScalarValue::Int32(0)),
        _ => ScalarValue::Int32(0),
    }
}

fn coerce_to_int16(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Int16(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Int16(*n as i16),
        ScalarValue::Int32(n) => ScalarValue::Int16(*n as i16),
        ScalarValue::Int8(n) => ScalarValue::Int16(*n as i16),
        ScalarValue::String(s) => s.parse::<i16>()
            .map(ScalarValue::Int16)
            .unwrap_or_else(|_| ScalarValue::Int16(0)),
        _ => ScalarValue::Int16(0),
    }
}

fn coerce_to_int8(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Int8(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Int8(*n as i8),
        ScalarValue::Int32(n) => ScalarValue::Int8(*n as i8),
        ScalarValue::String(s) => s.parse::<i8>()
            .map(ScalarValue::Int8)
            .unwrap_or_else(|_| ScalarValue::Int8(0)),
        _ => ScalarValue::Int8(0),
    }
}

fn coerce_to_int128(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Int128(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Int128(*n as i128),
        ScalarValue::Int32(n) => ScalarValue::Int128(*n as i128),
        ScalarValue::Float64(f) => ScalarValue::Int128(*f as i128),
        ScalarValue::String(s) => s.parse::<i128>()
            .map(ScalarValue::Int128)
            .unwrap_or_else(|_| ScalarValue::Int128(0)),
        _ => ScalarValue::Int128(0),
    }
}

fn coerce_to_float64(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Float64(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Float64(*n as f64),
        ScalarValue::Int32(n) => ScalarValue::Float64(*n as f64),
        ScalarValue::Float32(f) => ScalarValue::Float64(*f as f64),
        ScalarValue::String(s) => s.parse::<f64>()
            .map(ScalarValue::Float64)
            .unwrap_or_else(|_| ScalarValue::Float64(0.0)),
        _ => ScalarValue::Float64(0.0),
    }
}

fn coerce_to_float32(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Float32(_) => raw.clone(),
        ScalarValue::Float64(f) => ScalarValue::Float32(*f as f32),
        ScalarValue::Int64(n) => ScalarValue::Float32(*n as f32),
        ScalarValue::String(s) => s.parse::<f32>()
            .map(ScalarValue::Float32)
            .unwrap_or_else(|_| ScalarValue::Float32(0.0)),
        _ => ScalarValue::Float32(0.0),
    }
}

fn coerce_to_string(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::String(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::String(n.to_string()),
        ScalarValue::Float64(f) => ScalarValue::String(f.to_string()),
        ScalarValue::Boolean(b) => ScalarValue::String(b.to_string()),
        _ => ScalarValue::String(format!("{:?}", raw)),
    }
}

fn coerce_to_boolean(raw: &ScalarValue) -> ScalarValue {
    match raw {
        ScalarValue::Boolean(_) => raw.clone(),
        ScalarValue::Int64(n) => ScalarValue::Boolean(*n != 0),
        ScalarValue::String(s) => ScalarValue::Boolean(s == "true" || s == "1"),
        _ => ScalarValue::Boolean(false),
    }
}

/// Evaluate an ON DUPLICATE KEY UPDATE expression against an existing row.
pub fn eval_on_duplicate_key_expr(
    expr_str: &str,
    schema: &Schema,
    row_values: &[ScalarValue],
) -> ScalarValue {
    let expr_str = expr_str.trim();

    // First try to parse as a simple literal
    let literal_val = parse_value_string(expr_str);
    if !matches!(literal_val, ScalarValue::String(ref s) if s.is_empty() || s == expr_str) {
        return literal_val;
    }

    // Try to parse as "column op value" or "column op column" expression
    let ops: &[(&str, fn(&ScalarValue, &ScalarValue) -> ScalarValue)] = &[
        (" + ", binary_op_add),
        (" - ", binary_op_sub),
        (" * ", binary_op_mul),
        (" / ", binary_op_div),
        (" % ", binary_op_mod),
    ];
    for (op_str, op_fn) in ops {
        if let Some(pos) = expr_str.find(op_str) {
            let left_str = expr_str[..pos].trim();
            let right_str = expr_str[pos + op_str.len()..].trim();

            let left_val = if let Some(col_idx) = schema.index_of(left_str) {
                row_values.get(col_idx).cloned().unwrap_or(ScalarValue::Null)
            } else {
                parse_value_string(left_str)
            };

            let right_val = if let Some(col_idx) = schema.index_of(right_str) {
                row_values.get(col_idx).cloned().unwrap_or(ScalarValue::Null)
            } else {
                parse_value_string(right_str)
            };

            return op_fn(&left_val, &right_val);
        }
    }

    // If it's a bare column reference, return that column's value
    if let Some(col_idx) = schema.index_of(expr_str) {
        return row_values.get(col_idx).cloned().unwrap_or(ScalarValue::Null);
    }

    ScalarValue::String(expr_str.to_string())
}

fn binary_op_add(left: &ScalarValue, right: &ScalarValue) -> ScalarValue {
    match (left, right) {
        (ScalarValue::Int64(l), ScalarValue::Int64(r)) => ScalarValue::Int64(l + r),
        (ScalarValue::Int32(l), ScalarValue::Int32(r)) => ScalarValue::Int32(l + r),
        (ScalarValue::Int64(l), ScalarValue::Int32(r)) => ScalarValue::Int64(l + *r as i64),
        (ScalarValue::Int32(l), ScalarValue::Int64(r)) => ScalarValue::Int64(*l as i64 + r),
        (ScalarValue::Float64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(l + r),
        (ScalarValue::Float32(l), ScalarValue::Float32(r)) => ScalarValue::Float32(l + r),
        (ScalarValue::Int64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(*l as f64 + r),
        (ScalarValue::Float64(l), ScalarValue::Int64(r)) => ScalarValue::Float64(l + *r as f64),
        _ => ScalarValue::Null,
    }
}

fn binary_op_sub(left: &ScalarValue, right: &ScalarValue) -> ScalarValue {
    match (left, right) {
        (ScalarValue::Int64(l), ScalarValue::Int64(r)) => ScalarValue::Int64(l - r),
        (ScalarValue::Int32(l), ScalarValue::Int32(r)) => ScalarValue::Int32(l - r),
        (ScalarValue::Int64(l), ScalarValue::Int32(r)) => ScalarValue::Int64(l - *r as i64),
        (ScalarValue::Int32(l), ScalarValue::Int64(r)) => ScalarValue::Int64(*l as i64 - r),
        (ScalarValue::Float64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(l - r),
        (ScalarValue::Float32(l), ScalarValue::Float32(r)) => ScalarValue::Float32(l - r),
        (ScalarValue::Int64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(*l as f64 - r),
        (ScalarValue::Float64(l), ScalarValue::Int64(r)) => ScalarValue::Float64(l - *r as f64),
        _ => ScalarValue::Null,
    }
}

fn binary_op_mul(left: &ScalarValue, right: &ScalarValue) -> ScalarValue {
    match (left, right) {
        (ScalarValue::Int64(l), ScalarValue::Int64(r)) => ScalarValue::Int64(l * r),
        (ScalarValue::Int32(l), ScalarValue::Int32(r)) => ScalarValue::Int32(l * r),
        (ScalarValue::Int64(l), ScalarValue::Int32(r)) => ScalarValue::Int64(l * *r as i64),
        (ScalarValue::Int32(l), ScalarValue::Int64(r)) => ScalarValue::Int64(*l as i64 * r),
        (ScalarValue::Float64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(l * r),
        (ScalarValue::Float32(l), ScalarValue::Float32(r)) => ScalarValue::Float32(l * r),
        (ScalarValue::Int64(l), ScalarValue::Float64(r)) => ScalarValue::Float64(*l as f64 * r),
        (ScalarValue::Float64(l), ScalarValue::Int64(r)) => ScalarValue::Float64(l * *r as f64),
        _ => ScalarValue::Null,
    }
}

fn binary_op_div(left: &ScalarValue, right: &ScalarValue) -> ScalarValue {
    match (left, right) {
        (ScalarValue::Int64(l), ScalarValue::Int64(r)) => {
            if *r == 0 { ScalarValue::Null } else { ScalarValue::Int64(l / r) }
        }
        (ScalarValue::Int32(l), ScalarValue::Int32(r)) => {
            if *r == 0 { ScalarValue::Null } else { ScalarValue::Int32(l / r) }
        }
        (ScalarValue::Float64(l), ScalarValue::Float64(r)) => {
            if *r == 0.0 { ScalarValue::Null } else { ScalarValue::Float64(l / r) }
        }
        (ScalarValue::Float32(l), ScalarValue::Float32(r)) => {
            if *r == 0.0 { ScalarValue::Null } else { ScalarValue::Float32(l / r) }
        }
        _ => ScalarValue::Null,
    }
}

fn binary_op_mod(left: &ScalarValue, right: &ScalarValue) -> ScalarValue {
    match (left, right) {
        (ScalarValue::Int64(l), ScalarValue::Int64(r)) => {
            if *r == 0 { ScalarValue::Null } else { ScalarValue::Int64(l % r) }
        }
        (ScalarValue::Int32(l), ScalarValue::Int32(r)) => {
            if *r == 0 { ScalarValue::Null } else { ScalarValue::Int32(l % r) }
        }
        _ => ScalarValue::Null,
    }
}

/// Create a result block with the number of affected rows.
pub fn make_affected_rows_block(count: usize) -> Block {
    let schema = Schema::new(vec![Field::new("rows_affected", DataType::Int64, false)]);
    let col = Vector::Int64(Int64Vector::from_vec(vec![count as i64]));
    Block::new(schema, vec![col])
}

/// Convert a VALUES clause (Vec<Vec<Expr>>) into a Block for INSERT execution.
///
/// Transposes row-oriented expressions to column-oriented storage:
/// - Each inner Vec is a row with expressions for each column
/// - The result is a Block with one Vector per column
///
/// Returns an error if the number of columns in any row doesn't match the schema.
pub fn values_to_block(rows: Vec<Vec<Expr>>, schema: &Schema) -> Result<Block, String> {
    if rows.is_empty() {
        return Err(" VALUES clause cannot be empty".to_string());
    }

    let num_cols = schema.num_fields();
    let num_rows = rows.len();

    // Validate row length and transpose to column-oriented
    let mut columns: Vec<Vec<ScalarValue>> = vec![Vec::with_capacity(num_rows); num_cols];

    for (row_idx, row) in rows.iter().enumerate() {
        if row.len() != num_cols {
            return Err(format!(
                " Row {} has {} columns but schema expects {}",
                row_idx + 1,
                row.len(),
                num_cols
            ));
        }

        for (col_idx, expr) in row.iter().enumerate() {
            let scalar = expr_to_scalar_value(expr, schema.field(col_idx).map(|f| &f.data_type))?;
            columns[col_idx].push(scalar);
        }
    }

    // Build Vectors for each column based on data type
    let vectors: Vec<Vector> = schema
        .fields()
        .iter()
        .zip(columns.into_iter())
        .map(|(field, values)| scalar_values_to_vector(&values, &field.data_type))
        .collect();

    Ok(Block::new(schema.clone(), vectors))
}

/// Convert an Expr into a ScalarValue, optionally using target_type for coercion.
fn expr_to_scalar_value(expr: &Expr, target_type: Option<&DataType>) -> Result<ScalarValue, String> {
    match expr {
        Expr::Literal(lv) => literal_to_scalar_value(lv),
        Expr::Cast { expr, target_type: cast_type } => {
            // Recursively get the inner value, then apply cast
            let inner_scalar = expr_to_scalar_value(expr, target_type)?;
            coerce_to_type(&inner_scalar, cast_type)
        }
        _ => Err(format!(" Unsupported expression type in VALUES: {:?}", expr)),
    }
}

/// Convert a LiteralValue to a ScalarValue.
fn literal_to_scalar_value(lv: &LiteralValue) -> Result<ScalarValue, String> {
    match lv {
        LiteralValue::Null => Ok(ScalarValue::Null),
        LiteralValue::Boolean(b) => Ok(ScalarValue::Boolean(*b)),
        LiteralValue::Int64(n) => Ok(ScalarValue::Int64(*n)),
        LiteralValue::Float64(f) => Ok(ScalarValue::Float64(*f)),
        LiteralValue::String(s) => Ok(ScalarValue::String(s.clone())),
        LiteralValue::Date(s) => {
            // Parse date string YYYY-MM-DD to i32 days
            let days = s.replace('-', "").parse::<i32>()
                .map_err(|_| format!(" Invalid date format: {}", s))?;
            Ok(ScalarValue::Date(days))
        }
    }
}

/// Convert a list of ScalarValues into a Vector, using the data type to determine
/// which Vector variant to create.
fn scalar_values_to_vector(values: &[ScalarValue], data_type: &DataType) -> Vector {
    match data_type {
        DataType::Boolean => Vector::Boolean(BooleanVector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Boolean(b) = v { Some(*b) } else { None }).collect()
        )),
        DataType::Int8 => Vector::Int8(Int8Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Int8(n) = v { Some(*n) } else { None }).collect()
        )),
        DataType::Int16 => Vector::Int16(Int16Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Int16(n) = v { Some(*n) } else { None }).collect()
        )),
        DataType::Int32 => Vector::Int32(Int32Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Int32(n) = v { Some(*n) } else { None }).collect()
        )),
        DataType::Int64 => Vector::Int64(Int64Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Int64(n) = v { Some(*n) } else { None }).collect()
        )),
        DataType::Int128 => Vector::Int128(Int128Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Int128(n) = v { Some(*n) } else { None }).collect()
        )),
        DataType::Float32 => Vector::Float32(Float32Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Float32(f) = v { Some(*f) } else { None }).collect()
        )),
        DataType::Float64 => Vector::Float64(Float64Vector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Float64(f) = v { Some(*f) } else { None }).collect()
        )),
        DataType::Date => Vector::Date(DateVector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::Date(d) = v { Some(*d) } else { None }).collect()
        )),
        DataType::DateTime => Vector::DateTime(DateTimeVector::from_nullable_vec(
            values.iter().map(|v| if let ScalarValue::DateTime(d) = v { Some(*d) } else { None }).collect()
        )),
        DataType::String | DataType::Varchar(_) | DataType::Char(_) => {
            let strs: Vec<Option<String>> = values.iter().map(|v| {
                if let ScalarValue::String(s) = v { Some(s.clone()) } else { None }
            }).collect();
            Vector::String(StringVector::from_option_vec(strs))
        }
        DataType::Json => Vector::Json(JsonVector::from_option_vec(
            values.iter().map(|v| if let ScalarValue::Json(j) = v { Some(ScalarValue::Json(j.clone())) } else { None }).collect()
        )),
        _ => Vector::Null(types::vector::NullVector::new(values.len())),
    }
}

/// Apply type coercion to match the target column type.
fn coerce_to_type(scalar: &ScalarValue, target_type: &str) -> Result<ScalarValue, String> {
    // Parse the target type string to DataType
    let dt = match target_type.to_uppercase().as_str() {
        "BOOLEAN" | "BOOL" => DataType::Boolean,
        "TINYINT" | "INT8" => DataType::Int8,
        "SMALLINT" | "INT16" => DataType::Int16,
        "INT" | "INTEGER" | "INT32" => DataType::Int32,
        "BIGINT" | "INT64" => DataType::Int64,
        "LARGEINT" | "INT128" => DataType::Int128,
        "FLOAT" | "FLOAT32" => DataType::Float32,
        "DOUBLE" | "FLOAT64" => DataType::Float64,
        "DATE" => DataType::Date,
        "DATETIME" | "TIMESTAMP" => DataType::DateTime,
        "VARCHAR" | "CHAR" | "STRING" | "TEXT" => DataType::String,
        "JSON" => DataType::Json,
        _ => return Err(format!(" Unsupported cast target type: {}", target_type)),
    };

    match scalar {
        ScalarValue::Null => Ok(ScalarValue::Null),
        ScalarValue::Boolean(b) => coerce_boolean_to(b, &dt),
        ScalarValue::Int8(n) => coerce_int_to(*n as i64, &dt),
        ScalarValue::Int16(n) => coerce_int_to(*n as i64, &dt),
        ScalarValue::Int32(n) => coerce_int_to(*n as i64, &dt),
        ScalarValue::Int64(n) => coerce_int_to(*n, &dt),
        ScalarValue::Int128(n) => coerce_int_to(*n as i64, &dt),
        ScalarValue::Float32(f) => coerce_float_to(*f as f64, &dt),
        ScalarValue::Float64(f) => coerce_float_to(*f, &dt),
        ScalarValue::Date(d) => coerce_date_to(*d, &dt),
        ScalarValue::DateTime(dt_inner) => coerce_datetime_to(*dt_inner, &dt),
        ScalarValue::String(s) => coerce_string_to(s, &dt),
        _ => Err(format!(" Cannot cast {:?} to {:?}", scalar, target_type)),
    }
}

fn coerce_int_to(n: i64, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::Int8 => Ok(ScalarValue::Int8(n as i8)),
        DataType::Int16 => Ok(ScalarValue::Int16(n as i16)),
        DataType::Int32 => Ok(ScalarValue::Int32(n as i32)),
        DataType::Int64 => Ok(ScalarValue::Int64(n)),
        DataType::Int128 => Ok(ScalarValue::Int128(n as i128)),
        DataType::Float32 => Ok(ScalarValue::Float32(n as f32)),
        DataType::Float64 => Ok(ScalarValue::Float64(n as f64)),
        DataType::Boolean => Ok(ScalarValue::Boolean(n != 0)),
        DataType::String => Ok(ScalarValue::String(n.to_string())),
        _ => Err(format!(" Cannot cast Int64 to {:?}", target)),
    }
}

fn coerce_float_to(f: f64, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::Float32 => Ok(ScalarValue::Float32(f as f32)),
        DataType::Float64 => Ok(ScalarValue::Float64(f)),
        DataType::Int8 => Ok(ScalarValue::Int8(f as i8)),
        DataType::Int16 => Ok(ScalarValue::Int16(f as i16)),
        DataType::Int32 => Ok(ScalarValue::Int32(f as i32)),
        DataType::Int64 => Ok(ScalarValue::Int64(f as i64)),
        DataType::Int128 => Ok(ScalarValue::Int128(f as i128)),
        DataType::String => Ok(ScalarValue::String(f.to_string())),
        _ => Err(format!(" Cannot cast Float64 to {:?}", target)),
    }
}

fn coerce_boolean_to(b: &bool, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::Boolean => Ok(ScalarValue::Boolean(*b)),
        DataType::Int8 => Ok(ScalarValue::Int8(*b as i8)),
        DataType::Int16 => Ok(ScalarValue::Int16(*b as i16)),
        DataType::Int32 => Ok(ScalarValue::Int32(*b as i32)),
        DataType::Int64 => Ok(ScalarValue::Int64(*b as i64)),
        DataType::Int128 => Ok(ScalarValue::Int128(*b as i128)),
        DataType::Float32 => Ok(ScalarValue::Float32(*b as i32 as f32)),
        DataType::Float64 => Ok(ScalarValue::Float64(*b as i64 as f64)),
        DataType::String => Ok(ScalarValue::String(b.to_string())),
        _ => Err(format!(" Cannot cast Boolean to {:?}", target)),
    }
}

fn coerce_date_to(d: i32, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::Date => Ok(ScalarValue::Date(d)),
        DataType::DateTime => Ok(ScalarValue::DateTime(d as i64)),
        DataType::Int32 => Ok(ScalarValue::Int32(d)),
        DataType::Int64 => Ok(ScalarValue::Int64(d as i64)),
        DataType::String => Ok(ScalarValue::String(d.to_string())),
        _ => Err(format!(" Cannot cast Date to {:?}", target)),
    }
}

fn coerce_datetime_to(dt: i64, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::DateTime => Ok(ScalarValue::DateTime(dt)),
        DataType::Date => Ok(ScalarValue::Date(dt as i32)),
        DataType::Int64 => Ok(ScalarValue::Int64(dt)),
        DataType::String => Ok(ScalarValue::String(format!("{:?}", dt))),
        _ => Err(format!(" Cannot cast DateTime to {:?}", target)),
    }
}

fn coerce_string_to(s: &str, target: &DataType) -> Result<ScalarValue, String> {
    match target {
        DataType::String => Ok(ScalarValue::String(s.to_string())),
        DataType::Int8 => s.parse::<i8>()
            .map(ScalarValue::Int8)
            .map_err(|_| format!(" Cannot parse '{}' as Int8", s)),
        DataType::Int16 => s.parse::<i16>()
            .map(ScalarValue::Int16)
            .map_err(|_| format!(" Cannot parse '{}' as Int16", s)),
        DataType::Int32 => s.parse::<i32>()
            .map(ScalarValue::Int32)
            .map_err(|_| format!(" Cannot parse '{}' as Int32", s)),
        DataType::Int64 => s.parse::<i64>()
            .map(ScalarValue::Int64)
            .map_err(|_| format!(" Cannot parse '{}' as Int64", s)),
        DataType::Int128 => s.parse::<i128>()
            .map(ScalarValue::Int128)
            .map_err(|_| format!(" Cannot parse '{}' as Int128", s)),
        DataType::Float32 => s.parse::<f32>()
            .map(ScalarValue::Float32)
            .map_err(|_| format!(" Cannot parse '{}' as Float32", s)),
        DataType::Float64 => s.parse::<f64>()
            .map(ScalarValue::Float64)
            .map_err(|_| format!(" Cannot parse '{}' as Float64", s)),
        DataType::Boolean => Ok(ScalarValue::Boolean(s == "true" || s == "1")),
        DataType::Date => s.replace('-', "").parse::<i32>()
            .map(ScalarValue::Date)
            .map_err(|_| format!(" Cannot parse '{}' as Date", s)),
        _ => Err(format!(" Cannot cast String to {:?}", target)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_eq_int() {
        let preds = parse_predicates("id = 1");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].column_name, "id");
        assert_eq!(preds[0].op, PredicateOp::Eq);
        assert_eq!(preds[0].value, ScalarValue::Int64(1));
    }

    #[test]
    fn test_parse_eq_string() {
        let preds = parse_predicates("name = 'alice'");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].column_name, "name");
        assert_eq!(preds[0].value, ScalarValue::String("alice".to_string()));
    }

    #[test]
    fn test_parse_comparison_ops() {
        assert_eq!(parse_predicates("a < 5")[0].op, PredicateOp::Lt);
        assert_eq!(parse_predicates("a <= 5")[0].op, PredicateOp::Le);
        assert_eq!(parse_predicates("a > 5")[0].op, PredicateOp::Gt);
        assert_eq!(parse_predicates("a >= 5")[0].op, PredicateOp::Ge);
        assert_eq!(parse_predicates("a != 5")[0].op, PredicateOp::NotEq);
    }

    #[test]
    fn test_parse_is_null() {
        let preds = parse_predicates("name IS NULL");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].op, PredicateOp::IsNull);
    }

    #[test]
    fn test_parse_is_not_null() {
        let preds = parse_predicates("name IS NOT NULL");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].op, PredicateOp::IsNotNull);
    }

    #[test]
    fn test_parse_and() {
        let preds = parse_predicates("id = 1 AND name = 'alice'");
        assert_eq!(preds.len(), 2);
        assert_eq!(preds[0].column_name, "id");
        assert_eq!(preds[1].column_name, "name");
    }

    #[test]
    fn test_parse_in_list() {
        let preds = parse_predicates("id IN (1, 2, 3)");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].op, PredicateOp::In);
        assert_eq!(preds[0].values.len(), 3);
    }

    #[test]
    fn test_parse_between() {
        let preds = parse_predicates("age BETWEEN 18 AND 30");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].op, PredicateOp::Between);
    }

    #[test]
    fn test_parse_like() {
        let preds = parse_predicates("name LIKE '%alice%'");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].op, PredicateOp::Like);
        assert_eq!(preds[0].value, ScalarValue::String("%alice%".to_string()));
    }

    #[test]
    fn test_parse_null_value() {
        assert_eq!(parse_value_string("NULL"), ScalarValue::Null);
    }

    #[test]
    fn test_parse_boolean_value() {
        assert_eq!(parse_value_string("true"), ScalarValue::Boolean(true));
        assert_eq!(parse_value_string("false"), ScalarValue::Boolean(false));
    }

    #[test]
    fn test_parse_negative_int() {
        assert_eq!(parse_value_string("-5"), ScalarValue::Int64(-5));
    }

    #[test]
    fn test_parse_float_value() {
        assert_eq!(parse_value_string("3.14"), ScalarValue::Float64(3.14));
    }

    #[test]
    fn test_coerce_to_int64() {
        assert_eq!(parse_set_value("42", &DataType::Int64), ScalarValue::Int64(42));
        assert_eq!(parse_set_value("3.14", &DataType::Int64), ScalarValue::Int64(3));
        assert_eq!(parse_set_value("'100'", &DataType::Int64), ScalarValue::Int64(100));
    }

    #[test]
    fn test_coerce_to_string() {
        assert_eq!(parse_set_value("42", &DataType::String), ScalarValue::String("42".to_string()));
    }

    #[test]
    fn test_make_affected_rows_block() {
        let block = make_affected_rows_block(5);
        assert_eq!(block.num_rows(), 1);
        assert_eq!(block.num_columns(), 1);
        let val = block.column(0).unwrap().scalar_at(0);
        assert_eq!(val, ScalarValue::Int64(5));
    }

    #[test]
    fn test_values_to_block_integers() {
        use fe_sql_parser::ast::{Expr, LiteralValue};

        // INSERT INTO t VALUES (1), (2), (3)
        let rows = vec![
            vec![Expr::Literal(LiteralValue::Int64(1))],
            vec![Expr::Literal(LiteralValue::Int64(2))],
            vec![Expr::Literal(LiteralValue::Int64(3))],
        ];
        let schema = Schema::new(vec![Field::new("a", DataType::Int64, false)]);

        let block = values_to_block(rows, &schema).unwrap();
        assert_eq!(block.num_rows(), 3);
        assert_eq!(block.num_columns(), 1);
        assert_eq!(block.column(0).unwrap().scalar_at(0), ScalarValue::Int64(1));
        assert_eq!(block.column(0).unwrap().scalar_at(1), ScalarValue::Int64(2));
        assert_eq!(block.column(0).unwrap().scalar_at(2), ScalarValue::Int64(3));
    }

    #[test]
    fn test_values_to_block_with_nulls() {
        use fe_sql_parser::ast::{Expr, LiteralValue};

        // INSERT INTO t VALUES (1, 'hello'), (NULL, 'world')
        let rows = vec![
            vec![Expr::Literal(LiteralValue::Int64(1)), Expr::Literal(LiteralValue::String("hello".to_string()))],
            vec![Expr::Literal(LiteralValue::Null), Expr::Literal(LiteralValue::String("world".to_string()))],
        ];
        let schema = Schema::new(vec![
            Field::new("a", DataType::Int64, true),
            Field::new("b", DataType::String, true),
        ]);

        let block = values_to_block(rows, &schema).unwrap();
        assert_eq!(block.num_rows(), 2);
        assert_eq!(block.num_columns(), 2);
        assert_eq!(block.column(0).unwrap().scalar_at(0), ScalarValue::Int64(1));
        assert_eq!(block.column(0).unwrap().scalar_at(1), ScalarValue::Null);
        assert_eq!(block.column(1).unwrap().scalar_at(0), ScalarValue::String("hello".to_string()));
        assert_eq!(block.column(1).unwrap().scalar_at(1), ScalarValue::String("world".to_string()));
    }

    #[test]
    fn test_values_to_block_mixed_types() {
        use fe_sql_parser::ast::{Expr, LiteralValue};

        // INSERT INTO t VALUES (1, 'alice', true, 3.14)
        let rows = vec![
            vec![
                Expr::Literal(LiteralValue::Int64(1)),
                Expr::Literal(LiteralValue::String("alice".to_string())),
                Expr::Literal(LiteralValue::Boolean(true)),
                Expr::Literal(LiteralValue::Float64(3.14)),
            ],
        ];
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::String, false),
            Field::new("active", DataType::Boolean, false),
            Field::new("score", DataType::Float64, false),
        ]);

        let block = values_to_block(rows, &schema).unwrap();
        assert_eq!(block.num_rows(), 1);
        assert_eq!(block.num_columns(), 4);
        assert_eq!(block.column(0).unwrap().scalar_at(0), ScalarValue::Int64(1));
        assert_eq!(block.column(1).unwrap().scalar_at(0), ScalarValue::String("alice".to_string()));
        assert_eq!(block.column(2).unwrap().scalar_at(0), ScalarValue::Boolean(true));
        assert_eq!(block.column(3).unwrap().scalar_at(0), ScalarValue::Float64(3.14));
    }

    #[test]
    fn test_values_to_block_wrong_column_count() {
        use fe_sql_parser::ast::{Expr, LiteralValue};

        let rows = vec![
            vec![Expr::Literal(LiteralValue::Int64(1)), Expr::Literal(LiteralValue::Int64(2))],
        ];
        let schema = Schema::new(vec![Field::new("a", DataType::Int64, false)]);

        let result = values_to_block(rows, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has 2 columns but schema expects 1"));
    }

    #[test]
    fn test_values_to_block_empty() {
        let rows = vec![];
        let schema = Schema::new(vec![Field::new("a", DataType::Int64, false)]);

        let result = values_to_block(rows, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("VALUES clause cannot be empty"));
    }
}
