//! T-SQL data type mapping to HarnessDB internal types.

use crate::ast::TsqlDataType;
use types::DataType;
use types::data_type::DecimalType;

/// Convert a T-SQL data type to the corresponding HarnessDB DataType.
pub fn tsql_type_to_datatype(t: &TsqlDataType) -> DataType {
    match t {
        TsqlDataType::Int => DataType::Int32,
        TsqlDataType::SmallInt => DataType::Int16,
        TsqlDataType::TinyInt => DataType::UInt8,
        TsqlDataType::BigInt => DataType::Int64,
        TsqlDataType::Bit => DataType::Boolean,
        TsqlDataType::Decimal(p, s) => DataType::Decimal(DecimalType {
            precision: p.unwrap_or(18),
            scale: s.unwrap_or(0),
        }),
        TsqlDataType::Numeric(p, s) => DataType::Decimal(DecimalType {
            precision: p.unwrap_or(18),
            scale: s.unwrap_or(0),
        }),
        TsqlDataType::Money => DataType::Money,
        TsqlDataType::SmallMoney => DataType::SmallMoney,
        TsqlDataType::Float(p) => {
            if p.unwrap_or(53) <= 24 {
                DataType::Float32
            } else {
                DataType::Float64
            }
        }
        TsqlDataType::Real => DataType::Float32,
        TsqlDataType::Char(n) => DataType::Char(n.unwrap_or(1)),
        TsqlDataType::Varchar(n) => DataType::Varchar(n.unwrap_or(8000)),
        TsqlDataType::NChar(n) => DataType::Char(n.unwrap_or(1)),
        TsqlDataType::NVarchar(n) => DataType::Varchar(n.unwrap_or(4000)),
        TsqlDataType::Text | TsqlDataType::NText => DataType::String,
        TsqlDataType::Binary(n) => DataType::FixedSizeBinary(n.unwrap_or(1)),
        TsqlDataType::VarBinary(_) => DataType::Binary,
        TsqlDataType::Image => DataType::Binary,
        TsqlDataType::Date => DataType::Date,
        TsqlDataType::Time(_) => DataType::Time,
        TsqlDataType::DateTime | TsqlDataType::SmallDateTime | TsqlDataType::DateTime2(_) => {
            DataType::DateTime
        }
        TsqlDataType::DateTimeOffset(_) => DataType::DateTimeOffset,
        TsqlDataType::UniqueIdentifier => DataType::UniqueIdentifier,
        TsqlDataType::Xml => DataType::String,
        TsqlDataType::SqlVariant => DataType::String,
        TsqlDataType::Table => DataType::String,
        TsqlDataType::CursorType => DataType::String,
        TsqlDataType::UserDefined(_) => DataType::String,
    }
}

/// Convert a HarnessDB DataType to a T-SQL type name string.
pub fn datatype_to_tsql_type(dt: &DataType) -> String {
    match dt {
        DataType::Null => "NULL".to_string(),
        DataType::Boolean => "BIT".to_string(),
        DataType::Int8 => "TINYINT".to_string(),
        DataType::Int16 => "SMALLINT".to_string(),
        DataType::Int32 => "INT".to_string(),
        DataType::Int64 => "BIGINT".to_string(),
        DataType::Int128 => "DECIMAL(38, 0)".to_string(),
        DataType::UInt8 => "TINYINT".to_string(),
        DataType::UInt16 => "SMALLINT".to_string(),
        DataType::UInt32 => "INT".to_string(),
        DataType::UInt64 => "BIGINT".to_string(),
        DataType::Float32 => "REAL".to_string(),
        DataType::Float64 => "FLOAT".to_string(),
        DataType::Decimal(d) => format!("DECIMAL({}, {})", d.precision, d.scale),
        DataType::Date => "DATE".to_string(),
        DataType::DateTime => "DATETIME".to_string(),
        DataType::Time => "TIME".to_string(),
        DataType::DateTimeOffset => "DATETIMEOFFSET".to_string(),
        DataType::Varchar(n) => format!("VARCHAR({})", n),
        DataType::Char(n) => format!("CHAR({})", n),
        DataType::String => "VARCHAR(MAX)".to_string(),
        DataType::Binary => "VARBINARY(MAX)".to_string(),
        DataType::FixedSizeBinary(n) => format!("BINARY({})", n),
        DataType::Money => "MONEY".to_string(),
        DataType::SmallMoney => "SMALLMONEY".to_string(),
        DataType::UniqueIdentifier => "UNIQUEIDENTIFIER".to_string(),
        DataType::Json => "VARCHAR(MAX)".to_string(),
        DataType::Array(inner) => format!("VARCHAR(MAX) /* ARRAY of {} */", datatype_to_tsql_type(inner)),
        DataType::Map(_, _) => "VARCHAR(MAX) /* MAP */".to_string(),
        DataType::Struct(_) => "VARCHAR(MAX) /* STRUCT */".to_string(),
        DataType::Float32Vector(dim) => format!("BINARY({}) /* FLOAT32_VECTOR({}) */", dim * 4, dim),
    }
}

/// Parse a T-SQL type name string into TsqlDataType.
pub fn parse_tsql_type_name(name: &str) -> Option<TsqlDataType> {
    let upper = name.trim().to_uppercase();
    match upper.as_str() {
        "INT" | "INTEGER" => Some(TsqlDataType::Int),
        "SMALLINT" => Some(TsqlDataType::SmallInt),
        "TINYINT" => Some(TsqlDataType::TinyInt),
        "BIGINT" => Some(TsqlDataType::BigInt),
        "BIT" => Some(TsqlDataType::Bit),
        "MONEY" => Some(TsqlDataType::Money),
        "SMALLMONEY" => Some(TsqlDataType::SmallMoney),
        "REAL" => Some(TsqlDataType::Real),
        "DATE" => Some(TsqlDataType::Date),
        "DATETIME" => Some(TsqlDataType::DateTime),
        "SMALLDATETIME" => Some(TsqlDataType::SmallDateTime),
        "UNIQUEIDENTIFIER" => Some(TsqlDataType::UniqueIdentifier),
        "XML" => Some(TsqlDataType::Xml),
        "SQL_VARIANT" => Some(TsqlDataType::SqlVariant),
        "TEXT" => Some(TsqlDataType::Text),
        "NTEXT" => Some(TsqlDataType::NText),
        "IMAGE" => Some(TsqlDataType::Image),
        _ => {
            // Handle parameterized types: VARCHAR(n), DECIMAL(p,s), etc.
            if let Some(rest) = upper.strip_prefix("VARCHAR") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::Varchar(n))
            } else if let Some(rest) = upper.strip_prefix("CHAR") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::Char(n))
            } else if let Some(rest) = upper.strip_prefix("NVARCHAR") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::NVarchar(n))
            } else if let Some(rest) = upper.strip_prefix("NCHAR") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::NChar(n))
            } else if let Some(rest) = upper.strip_prefix("VARBINARY") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::VarBinary(n))
            } else if let Some(rest) = upper.strip_prefix("BINARY") {
                let n = parse_optional_size(rest);
                Some(TsqlDataType::Binary(n))
            } else if let Some(rest) = upper.strip_prefix("DECIMAL") {
                let (p, s) = parse_precision_scale(rest);
                Some(TsqlDataType::Decimal(p, s))
            } else if let Some(rest) = upper.strip_prefix("NUMERIC") {
                let (p, s) = parse_precision_scale(rest);
                Some(TsqlDataType::Numeric(p, s))
            } else if let Some(rest) = upper.strip_prefix("FLOAT") {
                let p = parse_optional_size(rest).map(|n| n as u8);
                Some(TsqlDataType::Float(p))
            } else if let Some(rest) = upper.strip_prefix("DATETIME2") {
                let p = parse_optional_size(rest).map(|n| n as u8);
                Some(TsqlDataType::DateTime2(p))
            } else if let Some(rest) = upper.strip_prefix("DATETIMEOFFSET") {
                let p = parse_optional_size(rest).map(|n| n as u8);
                Some(TsqlDataType::DateTimeOffset(p))
            } else if let Some(rest) = upper.strip_prefix("TIME") {
                let p = parse_optional_size(rest).map(|n| n as u8);
                Some(TsqlDataType::Time(p))
            } else {
                Some(TsqlDataType::UserDefined(name.to_string()))
            }
        }
    }
}

fn parse_optional_size(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1].trim();
        if inner.eq_ignore_ascii_case("MAX") {
            Some(8000) // MAX → large value
        } else {
            inner.parse().ok()
        }
    } else {
        None
    }
}

fn parse_precision_scale(s: &str) -> (Option<u8>, Option<u8>) {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        match parts.len() {
            1 => (parts[0].trim().parse().ok(), None),
            2 => (
                parts[0].trim().parse().ok(),
                parts[1].trim().parse().ok(),
            ),
            _ => (None, None),
        }
    } else {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_mapping() {
        assert_eq!(tsql_type_to_datatype(&TsqlDataType::Int), DataType::Int32);
        assert_eq!(tsql_type_to_datatype(&TsqlDataType::BigInt), DataType::Int64);
        assert_eq!(tsql_type_to_datatype(&TsqlDataType::Bit), DataType::Boolean);
        assert_eq!(tsql_type_to_datatype(&TsqlDataType::Money), DataType::Money);
    }

    #[test]
    fn test_parse_type_name() {
        assert_eq!(parse_tsql_type_name("INT"), Some(TsqlDataType::Int));
        assert_eq!(parse_tsql_type_name("VARCHAR(100)"), Some(TsqlDataType::Varchar(Some(100))));
        assert_eq!(parse_tsql_type_name("DECIMAL(18, 2)"), Some(TsqlDataType::Decimal(Some(18), Some(2))));
    }
}
