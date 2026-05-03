use types::{DataType, ScalarValue};
use chrono::{NaiveDate, NaiveDateTime, Datelike};

/// Infers the DataType from a string value
pub fn infer_type(value: &str) -> DataType {
    if value.is_empty() || value.eq_ignore_ascii_case("null") || value == "\\N" {
        return DataType::String;
    }

    // Try parsing as i64 first (integers are most common)
    if value.parse::<i64>().is_ok() {
        return DataType::Int64;
    }

    // Try parsing as f64 (float/double)
    if value.parse::<f64>().is_ok() {
        return DataType::Float64;
    }

    // Try parsing as date
    if is_date(value) {
        return DataType::Date;
    }

    // Try parsing as datetime
    if is_datetime(value) {
        return DataType::DateTime;
    }

    DataType::String
}

/// Checks if a string represents a valid date
pub fn is_date(value: &str) -> bool {
    // Try various date formats
    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d-%m-%Y",
        "%d/%m/%Y",
        "%m-%d-%Y",
        "%m/%d/%Y",
        "%Y%m%d",
        "%B %d, %Y",     // January 15, 2024
        "%b %d, %Y",     // Jan 15, 2024
        "%d %B %Y",      // 15 January 2024
        "%d %b %Y",      // 15 Jan 2024
    ];

    for format in &formats {
        if NaiveDate::parse_from_str(value, format).is_ok() {
            return true;
        }
    }
    false
}

/// Checks if a string represents a valid datetime
pub fn is_datetime(value: &str) -> bool {
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y%m%d %H:%M:%S",
        "%Y%m%d%H%M%S",
    ];

    for format in &formats {
        if NaiveDateTime::parse_from_str(value, format).is_ok() {
            return true;
        }
    }
    false
}

/// Infers a ScalarValue from a string, using the provided DataType as hint
pub fn infer_value(value: &str, data_type: &DataType) -> ScalarValue {
    if value.is_empty() || value.eq_ignore_ascii_case("null") || value == "\\N" {
        return ScalarValue::Null;
    }

    match data_type {
        DataType::Int64 => {
            value.parse::<i64>()
                .map(ScalarValue::Int64)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Int32 => {
            value.parse::<i32>()
                .map(ScalarValue::Int32)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Int16 => {
            value.parse::<i16>()
                .map(ScalarValue::Int16)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Int8 => {
            value.parse::<i8>()
                .map(ScalarValue::Int8)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Float64 => {
            value.parse::<f64>()
                .map(ScalarValue::Float64)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Float32 => {
            value.parse::<f32>()
                .map(ScalarValue::Float32)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::Date => {
            parse_date(value)
                .map(ScalarValue::Date)
                .unwrap_or(ScalarValue::Null)
        }
        DataType::DateTime => {
            parse_datetime(value)
                .map(ScalarValue::DateTime)
                .unwrap_or(ScalarValue::Null)
        }
        _ => ScalarValue::String(value.to_string()),
    }
}

/// Parses a date string into an ordinal day (day of year, 1-indexed)
fn parse_date(value: &str) -> Option<i32> {
    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d-%m-%Y",
        "%d/%m/%Y",
        "%m-%d-%Y",
        "%m/%d/%Y",
        "%Y%m%d",
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            // Return ordinal day (day of year, 1-indexed)
            return date.ordinal().try_into().ok();
        }
    }
    None
}

/// Parses a datetime string into milliseconds since epoch
fn parse_datetime(value: &str) -> Option<i64> {
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y%m%d %H:%M:%S",
        "%Y%m%d%H%M%S",
    ];

    for format in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return Some(dt.and_utc().timestamp_millis());
        }
    }
    None
}

/// Infers schema from a set of column values
pub fn infer_schema(column_names: &[String], column_values: &[Vec<String>]) -> types::Schema {
    let fields: Vec<types::Field> = column_values.iter().enumerate().map(|(col_idx, values)| {
        // Infer type from all values in the column
        let mut inferred_type = DataType::String;
        let mut nullable = false;

        for value in values {
            if value.is_empty() || value.eq_ignore_ascii_case("null") || value == "\\N" {
                nullable = true;
            } else {
                let ty = infer_type(value);
                // Use the most specific type that can accommodate all values
                inferred_type = unify_types(&inferred_type, &ty);
            }
        }

        let name = column_names.get(col_idx).cloned().unwrap_or_else(|| format!("col_{}", col_idx));
        types::Field {
            name,
            data_type: inferred_type,
            nullable,
        }
    }).collect();

    types::Schema::new(fields)
}

/// Unifies two data types to the most specific common type
fn unify_types(a: &DataType, b: &DataType) -> DataType {
    use DataType::*;

    match (a, b) {
        // Same types
        (x, y) if x == y => x.clone(),

        // Any type with Null is the other type
        (Null, x) | (x, Null) => x.clone(),

        // Numeric unify: promote to the wider type
        (Int64, Float64) | (Float64, Int64) => Float64,
        (Int64, x) | (x, Int64) if x.is_numeric() => x.clone(),
        (Float64, x) | (x, Float64) if x.is_numeric() => Float64,

        // Date and DateTime unify to DateTime
        (Date, DateTime) | (DateTime, Date) => DateTime,

        // Everything else defaults to String
        _ => String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_int() {
        assert_eq!(infer_type("123"), DataType::Int64);
        assert_eq!(infer_type("-456"), DataType::Int64);
    }

    #[test]
    fn test_infer_float() {
        assert_eq!(infer_type("123.45"), DataType::Float64);
        assert_eq!(infer_type(".5"), DataType::Float64);
        assert_eq!(infer_type("1e10"), DataType::Float64);
    }

    #[test]
    fn test_infer_date() {
        assert_eq!(infer_type("2024-01-15"), DataType::Date);
        assert_eq!(infer_type("2024/01/15"), DataType::Date);
    }

    #[test]
    fn test_infer_datetime() {
        assert_eq!(infer_type("2024-01-15 10:30:00"), DataType::DateTime);
    }

    #[test]
    fn test_infer_string() {
        assert_eq!(infer_type("hello"), DataType::String);
        assert_eq!(infer_type(""), DataType::String);
        assert_eq!(infer_type("null"), DataType::String);
    }
}