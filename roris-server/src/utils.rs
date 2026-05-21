use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::compute::kernels::cmp;
use datafusion::arrow::datatypes::{DataType as ADT, TimeUnit};
use fe_sql_parser::ast::BinaryOp;
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
        ADT::Int8 => {
            let arr = col.as_any().downcast_ref::<Int8Array>().unwrap();
            let val = val_str.parse::<i8>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Int8Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Int16 => {
            let arr = col.as_any().downcast_ref::<Int16Array>().unwrap();
            let val = val_str.parse::<i16>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(Int16Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::UInt8 => {
            let arr = col.as_any().downcast_ref::<UInt8Array>().unwrap();
            let val = val_str.parse::<u8>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(UInt8Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::UInt16 => {
            let arr = col.as_any().downcast_ref::<UInt16Array>().unwrap();
            let val = val_str.parse::<u16>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(UInt16Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::UInt32 => {
            let arr = col.as_any().downcast_ref::<UInt32Array>().unwrap();
            let val = val_str.parse::<u32>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(UInt32Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::UInt64 => {
            let arr = col.as_any().downcast_ref::<UInt64Array>().unwrap();
            let val = val_str.parse::<u64>().map_err(|e| format!("Parse error: {}", e))?;
            Arc::new(UInt64Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Boolean => {
            let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
            let val = match val_str.to_lowercase().as_str() {
                "true" | "1" | "yes" => true,
                _ => false,
            };
            Arc::new(BooleanArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Date32 => {
            let arr = col.as_any().downcast_ref::<Date32Array>().unwrap();
            let val = parse_date_to_days(val_str).ok_or_else(|| format!("Invalid date: {}", val_str))?;
            Arc::new(Date32Array::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Timestamp(TimeUnit::Second, _) => {
            let arr = col.as_any().downcast_ref::<TimestampSecondArray>().unwrap();
            let val = parse_datetime_to_seconds(val_str).ok_or_else(|| format!("Invalid datetime: {}", val_str))?;
            Arc::new(TimestampSecondArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Timestamp(TimeUnit::Millisecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampMillisecondArray>().unwrap();
            let seconds = parse_datetime_to_seconds(val_str).ok_or_else(|| format!("Invalid datetime: {}", val_str))?;
            let val = seconds * 1000;
            Arc::new(TimestampMillisecondArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Timestamp(TimeUnit::Microsecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampMicrosecondArray>().unwrap();
            let seconds = parse_datetime_to_seconds(val_str).ok_or_else(|| format!("Invalid datetime: {}", val_str))?;
            let val = seconds * 1_000_000;
            Arc::new(TimestampMicrosecondArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
            ))
        }
        ADT::Timestamp(TimeUnit::Nanosecond, _) => {
            let arr = col.as_any().downcast_ref::<TimestampNanosecondArray>().unwrap();
            let seconds = parse_datetime_to_seconds(val_str).ok_or_else(|| format!("Invalid datetime: {}", val_str))?;
            let val = seconds * 1_000_000_000;
            Arc::new(TimestampNanosecondArray::from_iter(
                (0..arr.len()).map(|i| if update_mask[i] { Some(val) } else { Some(arr.value(i)) })
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

/// Build an Arrow array directly from parser expressions, using the target Arrow DataType.
/// This avoids the string intermediate round-trip (Expr -> String -> typed parse).
pub(crate) fn build_arrow_array_from_exprs(
    arrow_type: &ADT,
    exprs: &[&fe_sql_parser::ast::Expr],
) -> ArrayRef {
    use fe_sql_parser::ast::{Expr, LiteralValue};

    match arrow_type {
        ADT::Int8 => {
            let arr: Int8Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as i8),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Int16 => {
            let arr: Int16Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as i16),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Int32 => {
            let arr: Int32Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as i32),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Int64 => {
            let arr: Int64Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Float32 => {
            let arr: Float32Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Float64(f)) => Some(*f as f32),
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as f32),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Float64 => {
            let arr: Float64Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Float64(f)) => Some(*f),
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as f64),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Boolean => {
            let arr: BooleanArray = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Boolean(b)) => Some(*b),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Date32 => {
            let arr: Date32Array = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Date(s)) => parse_date_to_days(s),
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n as i32),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Timestamp(TimeUnit::Second, _) => {
            let arr: TimestampSecondArray = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::Date(s)) => parse_datetime_to_seconds(s),
                Expr::Literal(LiteralValue::Int64(n)) => Some(*n),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        ADT::Utf8 => {
            let arr: StringArray = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::String(s)) => Some(s.clone()),
                Expr::Literal(LiteralValue::Int64(n)) => Some(n.to_string()),
                Expr::Literal(LiteralValue::Float64(f)) => Some(f.to_string()),
                Expr::Literal(LiteralValue::Boolean(b)) => Some(b.to_string()),
                Expr::Literal(LiteralValue::Date(d)) => Some(d.clone()),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
            Arc::new(arr)
        }
        _ => {
            // Fallback: build a StringArray for unsupported types
            let arr: StringArray = exprs.iter().map(|e| match e {
                Expr::Literal(LiteralValue::String(s)) => Some(s.clone()),
                Expr::Literal(LiteralValue::Int64(n)) => Some(n.to_string()),
                Expr::Literal(LiteralValue::Float64(f)) => Some(f.to_string()),
                Expr::Literal(LiteralValue::Boolean(b)) => Some(b.to_string()),
                Expr::Literal(LiteralValue::Date(d)) => Some(d.clone()),
                Expr::Literal(LiteralValue::Null) => None,
                _ => None,
            }).collect();
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

/// Evaluate a WHERE filter against a RecordBatch.
/// Returns a mask where `true` means the row **matches** the condition.
/// Recursively handles AND/OR compound conditions.
pub(crate) fn evaluate_where_filter(
    batch: &datafusion::arrow::record_batch::RecordBatch,
    where_expr: &fe_sql_parser::ast::Expr,
) -> Result<Vec<bool>, String> {
    use fe_sql_parser::ast::{Expr, LiteralValue};
    let num_rows = batch.num_rows();

    match where_expr {
        Expr::BinaryOp { left, op, right } => {
            match op {
                BinaryOp::And => {
                    let left_mask = evaluate_where_filter(batch, left)?;
                    let right_mask = evaluate_where_filter(batch, right)?;
                    Ok(left_mask.iter().zip(right_mask.iter()).map(|(l, r)| *l && *r).collect())
                }
                BinaryOp::Or => {
                    let left_mask = evaluate_where_filter(batch, left)?;
                    let right_mask = evaluate_where_filter(batch, right)?;
                    Ok(left_mask.iter().zip(right_mask.iter()).map(|(l, r)| *l || *r).collect())
                }
                // Comparison ops: column vs literal
                _ => {
                    let mut matches = vec![false; num_rows];
                    let col_name = match left.as_ref() {
                        Expr::ColumnRef { table: _, column } => column.clone(),
                        _ => return Ok(matches),
                    };
                    let col_idx = match batch.schema().index_of(&col_name) {
                        Ok(idx) => idx,
                        Err(_) => return Ok(matches),
                    };
                    let col = batch.column(col_idx);

                    let val_str = match right.as_ref() {
                        Expr::Literal(LiteralValue::Int64(n)) => n.to_string(),
                        Expr::Literal(LiteralValue::Float64(f)) => f.to_string(),
                        Expr::Literal(LiteralValue::String(s)) => s.clone(),
                        _ => return Ok(matches),
                    };

                    apply_cmp(&mut matches, col, &val_str, op);
                    Ok(matches)
                }
            }
        }
        _ => Ok(vec![false; num_rows])
    }
}

/// Compare array column with a scalar value using Arrow typed compute kernels.
/// Avoids string-based comparison for numeric types (the old approach caused
/// bugs like `WHERE age > 2` returning false for age=10 because "10" < "2" lexicographically).
fn apply_cmp(mask: &mut [bool], col: &ArrayRef, val: &str, op: &BinaryOp) {
    /// Macro to dispatch the Arrow compute comparison kernel for a given op.
    macro_rules! cmp_op {
        ($arr:expr, $fa:expr, $op:expr) => {
            match $op {
                BinaryOp::Eq => match cmp::eq($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                BinaryOp::Gt => match cmp::gt($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                BinaryOp::Lt => match cmp::lt($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                BinaryOp::GtEq => match cmp::gt_eq($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                BinaryOp::LtEq => match cmp::lt_eq($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                BinaryOp::NotEq => match cmp::neq($arr, &$fa) {
                    Ok(b) => b,
                    Err(_) => return,
                },
                _ => return,
            }
        };
    }

    let result: BooleanArray = match col.data_type() {
        ADT::Int8 => {
            let arr = col.as_any().downcast_ref::<Int8Array>().unwrap();
            let fv: i8 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Int8Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Int16 => {
            let arr = col.as_any().downcast_ref::<Int16Array>().unwrap();
            let fv: i16 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Int16Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            let fv: i32 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Int32Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            let fv: i64 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Int64Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::UInt8 => {
            let arr = col.as_any().downcast_ref::<UInt8Array>().unwrap();
            let fv: u8 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: UInt8Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::UInt16 => {
            let arr = col.as_any().downcast_ref::<UInt16Array>().unwrap();
            let fv: u16 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: UInt16Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::UInt32 => {
            let arr = col.as_any().downcast_ref::<UInt32Array>().unwrap();
            let fv: u32 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: UInt32Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::UInt64 => {
            let arr = col.as_any().downcast_ref::<UInt64Array>().unwrap();
            let fv: u64 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: UInt64Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Float32 => {
            let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
            let fv: f32 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Float32Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let fv: f64 = match val.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fa: Float64Array = std::iter::repeat(Some(fv)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Date32 => {
            let arr = col.as_any().downcast_ref::<Date32Array>().unwrap();
            let days = match parse_date_to_days(val) {
                Some(d) => d,
                None => return,
            };
            let fa: Date32Array = std::iter::repeat(Some(days)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        ADT::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            let fa: StringArray = std::iter::repeat(Some(val)).take(arr.len()).collect();
            cmp_op!(arr, fa, op)
        }
        _ => return,
    };

    for (i, m) in mask.iter_mut().enumerate() {
        if !col.is_null(i) && result.value(i) {
            *m = true;
        }
    }
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
}