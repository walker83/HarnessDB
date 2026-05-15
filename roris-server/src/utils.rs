use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::{DataType as ADT, TimeUnit};
use mysql_protocol::server::{ColumnDef, ColumnType};
use mysql_protocol::QueryResult;
use ::types::DataType;

pub(crate) fn literal_to_string(lit: &fe_sql_parser::ast::LiteralValue) -> String {
    match lit {
        fe_sql_parser::ast::LiteralValue::Null => "NULL".to_string(),
        fe_sql_parser::ast::LiteralValue::Boolean(b) => b.to_string(),
        fe_sql_parser::ast::LiteralValue::Int64(i) => i.to_string(),
        fe_sql_parser::ast::LiteralValue::Float64(f) => f.to_string(),
        fe_sql_parser::ast::LiteralValue::String(s) => s.clone(),
        fe_sql_parser::ast::LiteralValue::Date(d) => d.clone(),
    }
}

pub(crate) fn parse_data_type(s: &str) -> DataType {
    match s.to_uppercase().as_str() {
        "INT8" | "TINYINT" => DataType::Int8,
        "INT16" | "SMALLINT" => DataType::Int16,
        "INT32" | "INT" => DataType::Int32,
        "INT64" | "BIGINT" => DataType::Int64,
        "FLOAT32" | "FLOAT" => DataType::Float32,
        "FLOAT64" | "DOUBLE" => DataType::Float64,
        "STRING" | "VARCHAR" | "TEXT" => DataType::String,
        "BOOLEAN" | "BOOL" => DataType::Boolean,
        "DATE" => DataType::Date,
        "DATETIME" | "TIMESTAMP" => DataType::DateTime,
        _ => DataType::String,
    }
}

pub(crate) fn like_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '%' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            if p[i - 1] == '%' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p[i - 1] == '_' {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = dp[i - 1][j - 1] && p[i - 1].to_ascii_lowercase() == t[j - 1].to_ascii_lowercase();
            }
        }
    }
    dp[p.len()][t.len()]
}

pub(crate) fn record_batches_to_query_result(batches: &[datafusion::arrow::record_batch::RecordBatch]) -> QueryResult {
    if batches.is_empty() {
        return QueryResult::new(Vec::new());
    }

    let schema = batches[0].schema();
    let columns: Vec<ColumnDef> = schema.fields().iter().map(|f| {
        let col_type = match f.data_type() {
            ADT::Int8 | ADT::Int16 | ADT::Int32 | ADT::Int64 => ColumnType::Int,
            ADT::Float32 => ColumnType::Float,
            ADT::Float64 => ColumnType::Double,
            ADT::Boolean => ColumnType::Int,
            ADT::Date32 | ADT::Date64 => ColumnType::Date,
            ADT::Timestamp(_, _) => ColumnType::DateTime,
            _ => ColumnType::String,
        };
        ColumnDef { name: f.name().clone(), col_type }
    }).collect();

    let mut string_rows: Vec<Vec<Option<String>>> = Vec::new();
    for batch in batches {
        if batch.num_rows() == 0 {
            continue;
        }
        for row_idx in 0..batch.num_rows() {
            let row: Vec<Option<String>> = batch.columns().iter().map(|col| {
                arrow_value_to_string(col, row_idx)
            }).collect();
            string_rows.push(row);
        }
    }

    QueryResult::with_rows(columns, string_rows)
}

pub(crate) fn record_batches_to_query_result_with_df_schema(
    batches: &[datafusion::arrow::record_batch::RecordBatch],
    df_schema: &datafusion::common::DFSchema,
) -> QueryResult {
    let schema = df_schema.as_arrow();

    let columns: Vec<ColumnDef> = schema.fields().iter().map(|f| {
        let col_type = match f.data_type() {
            ADT::Int8 | ADT::Int16 | ADT::Int32 | ADT::Int64 => ColumnType::Int,
            ADT::Float32 => ColumnType::Float,
            ADT::Float64 => ColumnType::Double,
            ADT::Boolean => ColumnType::Int,
            ADT::Date32 | ADT::Date64 => ColumnType::Date,
            ADT::Timestamp(_, _) => ColumnType::DateTime,
            _ => ColumnType::String,
        };
        ColumnDef { name: f.name().clone(), col_type }
    }).collect();

    if columns.is_empty() {
        return QueryResult::new(Vec::new());
    }

    let mut string_rows: Vec<Vec<Option<String>>> = Vec::new();
    for batch in batches {
        if batch.num_rows() == 0 {
            continue;
        }
        for row_idx in 0..batch.num_rows() {
            let row: Vec<Option<String>> = batch.columns().iter().map(|col| {
                arrow_value_to_string(col, row_idx)
            }).collect();
            string_rows.push(row);
        }
    }

    QueryResult::with_rows(columns, string_rows)
}

pub(crate) fn arrow_value_to_string(col: &datafusion::arrow::array::ArrayRef, idx: usize) -> Option<String> {
    if col.is_null(idx) {
        return None;
    }

    match col.data_type() {
        ADT::Boolean => {
            let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
            Some(if arr.value(idx) { "1" } else { "0" }.to_string())
        }
        ADT::Int8 => {
            let arr = col.as_any().downcast_ref::<Int8Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Int16 => {
            let arr = col.as_any().downcast_ref::<Int16Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Float32 => {
            let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            Some(arr.value(idx).to_string())
        }
        ADT::Date32 => {
            let arr = col.as_any().downcast_ref::<Date32Array>().unwrap();
            let days = arr.value(idx);
            let base = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let date = base + chrono::Duration::days(days as i64);
            Some(date.format("%Y-%m-%d").to_string())
        }
        ADT::Timestamp(TimeUnit::Second, _) => {
            let arr = col.as_any().downcast_ref::<TimestampSecondArray>().unwrap();
            let ts = arr.value(idx);
            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
            Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
        }
        ADT::Timestamp(TimeUnit::Millisecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampMillisecondArray>().unwrap();
            let ts = arr.value(idx);
            let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap_or_default();
            Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
        }
        ADT::Timestamp(TimeUnit::Microsecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampMicrosecondArray>().unwrap();
            let ts = arr.value(idx);
            let dt = chrono::DateTime::from_timestamp_micros(ts).unwrap_or_default();
            Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
        }
        ADT::Timestamp(TimeUnit::Nanosecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampNanosecondArray>().unwrap();
            let ts = arr.value(idx);
            let secs = ts / 1_000_000_000;
            let nsecs = (ts % 1_000_000_000) as u32;
            match chrono::DateTime::from_timestamp(secs, nsecs) {
                Some(dt) => Some(dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                None => Some("1970-01-01 00:00:00".to_string()),
            }
        }
        _ => {
            let arr = col.as_any().downcast_ref::<StringArray>();
            arr.map(|a| a.value(idx).to_string())
        }
    }
}

pub(crate) fn expr_to_string_value(expr: &fe_sql_parser::ast::Expr) -> Option<String> {
    use fe_sql_parser::ast::{Expr, LiteralValue};
    match expr {
        Expr::Literal(LiteralValue::Int64(n)) => Some(n.to_string()),
        Expr::Literal(LiteralValue::Float64(f)) => Some(f.to_string()),
        Expr::Literal(LiteralValue::String(s)) => Some(s.clone()),
        Expr::Literal(LiteralValue::Boolean(b)) => Some(b.to_string()),
        Expr::Literal(LiteralValue::Null) => None,
        Expr::Literal(LiteralValue::Date(d)) => Some(d.clone()),
        _ => None,
    }
}

pub(crate) fn update_column_in_batch(
    batch: &mut datafusion::arrow::record_batch::RecordBatch,
    col_idx: usize,
    val_str: &str,
    update_mask: &[bool],
) -> Result<(), String> {
    let col = batch.column(col_idx);
    let new_col: ArrayRef = match col.data_type() {
        ADT::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            let val = val_str.parse::<i32>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Int32Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            let val = val_str.parse::<i64>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Int64Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Float32 => {
            let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
            let val = val_str.parse::<f32>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Float32Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let val = val_str.parse::<f64>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Float64Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            Arc::new(StringArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val_str) } else { Some(arr.value(i)) })
            ))
        }
        _ => return Err(format!("Unsupported column type for UPDATE: {}", col.data_type())),
    };

    let mut new_columns: Vec<ArrayRef> = batch.columns().iter().cloned().collect();
    new_columns[col_idx] = new_col;
    *batch = datafusion::arrow::record_batch::RecordBatch::try_new(
        batch.schema(),
        new_columns,
    ).map_err(|e| format!("Failed to create new batch: {}", e))?;

    Ok(())
}

pub(crate) fn build_arrow_array(col_type: &DataType, values: &[Option<String>]) -> datafusion::arrow::array::ArrayRef {
    match col_type {
        DataType::Int8 => {
            let arr: Int8Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<i8>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Int16 => {
            let arr: Int16Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<i16>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Int32 => {
            let arr: Int32Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<i32>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Int64 => {
            let arr: Int64Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Float32 => {
            let arr: Float32Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<f32>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Float64 => {
            let arr: Float64Array = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<f64>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Boolean => {
            let arr: BooleanArray = values.iter().map(|v| v.as_ref().and_then(|s| s.parse::<bool>().ok())).collect();
            Arc::new(arr)
        }
        DataType::Date => {
            let arr: Date32Array = values.iter().map(|v| {
                v.as_ref().and_then(|s| parse_date_to_days(s))
            }).collect();
            Arc::new(arr)
        }
        DataType::DateTime => {
            let arr: TimestampSecondArray = values.iter().map(|v| {
                v.as_ref().and_then(|s| parse_datetime_to_seconds(s))
            }).collect();
            Arc::new(arr)
        }
        _ => {
            let arr: StringArray = values.iter().map(|v| v.as_ref().map(|s| s.as_str())).collect();
            Arc::new(arr)
        }
    }
}

pub(crate) fn parse_date_to_days(s: &str) -> Option<i32> {
    let s = s.trim().trim_start_matches("'").trim_end_matches("'")
              .trim_start_matches("\"").trim_end_matches("\"");

    use chrono::NaiveDate;

    let date = if s.contains('-') {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?
    } else if s.contains('/') {
        NaiveDate::parse_from_str(s, "%Y/%m/%d").ok()?
    } else {
        return None;
    };

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?;
    let days = (date - epoch).num_days() as i32;
    Some(days)
}

pub(crate) fn parse_datetime_to_seconds(s: &str) -> Option<i64> {
    let s = s.trim().trim_start_matches("'").trim_end_matches("'")
              .trim_start_matches("\"").trim_end_matches("\"");

    use chrono::NaiveDateTime;

    if s.contains(':') {
        let datetime = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()?;
        return Some(datetime.and_utc().timestamp());
    }

    let days = parse_date_to_days(s)?;
    Some(days as i64 * 86400)
}

pub(crate) fn evaluate_delete_filter_simple(
    batch: &datafusion::arrow::record_batch::RecordBatch,
    where_expr: &fe_sql_parser::ast::Expr,
) -> Result<Vec<bool>, String> {
    use fe_sql_parser::ast::{BinaryOp, Expr, LiteralValue};
    let num_rows = batch.num_rows();
    let mut keep = vec![true; num_rows];

    if let Expr::BinaryOp { left, op, right } = where_expr {
        let col_name = match left.as_ref() {
            Expr::ColumnRef { table: _, column } => column.clone(),
            _ => return Ok(keep),
        };
        let col_idx = batch.schema().index_of(&col_name).map_err(|e| format!("{}", e))?;
        let col = batch.column(col_idx);

        let val_str = match right.as_ref() {
            Expr::Literal(LiteralValue::Int64(n)) => n.to_string(),
            Expr::Literal(LiteralValue::Float64(f)) => f.to_string(),
            Expr::Literal(LiteralValue::String(s)) => s.clone(),
            _ => return Ok(keep),
        };

        match op {
            BinaryOp::Eq => apply_cmp(&mut keep, col, &val_str, |a, b| a == b),
            BinaryOp::Gt => apply_cmp(&mut keep, col, &val_str, |a, b| a > b),
            BinaryOp::Lt => apply_cmp(&mut keep, col, &val_str, |a, b| a < b),
            BinaryOp::GtEq => apply_cmp(&mut keep, col, &val_str, |a, b| a >= b),
            BinaryOp::LtEq => apply_cmp(&mut keep, col, &val_str, |a, b| a <= b),
            BinaryOp::NotEq => apply_cmp(&mut keep, col, &val_str, |a, b| a != b),
            _ => {}
        }
    }

    Ok(keep)
}

pub(crate) fn apply_cmp<F: Fn(&str, &str) -> bool>(keep: &mut [bool], col: &datafusion::arrow::array::ArrayRef, val: &str, cmp: F) {
    match col.data_type() {
        ADT::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            for (i, k) in keep.iter_mut().enumerate() {
                if !arr.is_null(i) {
                    let v = arr.value(i).to_string();
                    if !cmp(&v, val) { *k = false; }
                }
            }
        }
        ADT::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            for (i, k) in keep.iter_mut().enumerate() {
                if !arr.is_null(i) {
                    let v = arr.value(i).to_string();
                    if !cmp(&v, val) { *k = false; }
                }
            }
        }
        ADT::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            for (i, k) in keep.iter_mut().enumerate() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if !cmp(v, val) { *k = false; }
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_sql_fallback(sql: &str) -> fe_sql_parser::Statement {
    let trimmed = sql.trim();
    let upper = trimmed.to_uppercase();

    if upper.starts_with("SET") {
        if let Some(eq_pos) = upper.find('=') {
            let var_name = trimmed[3..eq_pos].trim().to_uppercase();
            let value = trimmed[eq_pos + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
            return fe_sql_parser::Statement::SetVariable(fe_sql_parser::ast::SetVariableStmt {
                variable: var_name,
                value: fe_sql_parser::ast::Expr::Literal(fe_sql_parser::ast::LiteralValue::String(value)),
                is_global: false,
            });
        }
    }
    fe_sql_parser::Statement::Query(fe_sql_parser::ast::QueryStmt {
        select_list: vec![],
        from: None,
        r#where: None,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        with: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_like_match_exact() {
        assert!(like_match("abc", "abc"));
        assert!(!like_match("abc", "abd"));
    }

    #[test]
    fn test_like_match_wildcard_percent() {
        assert!(like_match("%", "anything"));
        assert!(like_match("a%", "abc"));
        assert!(like_match("%c", "abc"));
        assert!(like_match("%or%", "roris"));
    }

    #[test]
    fn test_like_match_wildcard_underscore() {
        assert!(like_match("a_c", "abc"));
        assert!(like_match("___", "abc"));
        assert!(!like_match("a_c", "abcd"));
    }

    #[test]
    fn test_like_match_case_insensitive() {
        assert!(like_match("ABC", "abc"));
        assert!(like_match("abc", "ABC"));
    }

    #[test]
    fn test_parse_data_type() {
        assert!(matches!(parse_data_type("INT"), DataType::Int32));
        assert!(matches!(parse_data_type("BIGINT"), DataType::Int64));
        assert!(matches!(parse_data_type("VARCHAR"), DataType::String));
        assert!(matches!(parse_data_type("DOUBLE"), DataType::Float64));
        assert!(matches!(parse_data_type("BOOL"), DataType::Boolean));
        assert!(matches!(parse_data_type("DATE"), DataType::Date));
        assert!(matches!(parse_data_type("UNKNOWN"), DataType::String));
    }

    #[test]
    fn test_parse_date_to_days() {
        assert_eq!(parse_date_to_days("1970-01-01"), Some(0));
        assert_eq!(parse_date_to_days("1970-01-02"), Some(1));
        assert_eq!(parse_date_to_days("1970/01/01"), Some(0));
        assert_eq!(parse_date_to_days("invalid"), None);
    }

    #[test]
    fn test_parse_datetime_to_seconds() {
        assert_eq!(parse_datetime_to_seconds("1970-01-01 00:00:00"), Some(0));
        assert_eq!(parse_datetime_to_seconds("1970-01-01 01:00:00"), Some(3600));
        assert_eq!(parse_datetime_to_seconds("1970-01-02"), Some(86400));
    }

    #[test]
    fn test_literal_to_string() {
        use fe_sql_parser::ast::LiteralValue;
        assert_eq!(literal_to_string(&LiteralValue::Null), "NULL");
        assert_eq!(literal_to_string(&LiteralValue::Int64(42)), "42");
        assert_eq!(literal_to_string(&LiteralValue::Float64(3.14)), "3.14");
        assert_eq!(literal_to_string(&LiteralValue::String("hello".into())), "hello");
        assert_eq!(literal_to_string(&LiteralValue::Boolean(true)), "true");
    }

    #[test]
    fn test_parse_sql_fallback_set() {
        if let fe_sql_parser::Statement::SetVariable(stmt) = parse_sql_fallback("SET autocommit = 1") {
            assert_eq!(stmt.variable, "AUTOCOMMIT");
            assert!(matches!(stmt.value, fe_sql_parser::ast::Expr::Literal(fe_sql_parser::ast::LiteralValue::String(ref s)) if s == "1"));
        } else {
            panic!("Expected SetVariable");
        }
    }
}
