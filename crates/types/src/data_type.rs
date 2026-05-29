use serde::{Deserialize, Serialize};
use std::fmt;

use crate::Field;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    Null,
    Boolean,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    Float32,
    Float64,
    Decimal(DecimalType),
    Date,
    DateTime,
    Varchar(usize),
    Char(usize),
    String,
    Binary,
    Array(Box<DataType>),
    Map(Box<DataType>, Box<DataType>),
    Struct(Vec<Field>),
    Json,
    /// Float32 vector with dimension
    Float32Vector(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecimalType {
    pub precision: u8,
    pub scale: u8,
}

impl DataType {
    pub fn size(&self) -> usize {
        match self {
            Self::Null => 0,
            Self::Boolean => 1,
            Self::Int8 => 1,
            Self::Int16 => 2,
            Self::Int32 => 4,
            Self::Int64 => 8,
            Self::Int128 => 16,
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::Date => 4,
            Self::DateTime => 8,
            Self::Varchar(_) | Self::Char(_) | Self::String | Self::Binary => 16, // offset + length
            Self::Decimal(_) => 16,
            Self::Json => 32,
            Self::Float32Vector(dim) => dim * 4,
            Self::Array(_) | Self::Map(_, _) | Self::Struct(_) => 0,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Int128
                | Self::Float32
                | Self::Float64
                | Self::Decimal(_)
        )
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "NULL"),
            Self::Boolean => write!(f, "BOOLEAN"),
            Self::Int8 => write!(f, "INT8"),
            Self::Int16 => write!(f, "INT16"),
            Self::Int32 => write!(f, "INT32"),
            Self::Int64 => write!(f, "INT64"),
            Self::Int128 => write!(f, "INT128"),
            Self::Float32 => write!(f, "FLOAT32"),
            Self::Float64 => write!(f, "FLOAT64"),
            Self::Decimal(d) => write!(f, "DECIMAL({}, {})", d.precision, d.scale),
            Self::Date => write!(f, "DATE"),
            Self::DateTime => write!(f, "DATETIME"),
            Self::Varchar(n) => write!(f, "VARCHAR({})", n),
            Self::Char(n) => write!(f, "CHAR({})", n),
            Self::String => write!(f, "STRING"),
            Self::Binary => write!(f, "BINARY"),
            Self::Array(inner) => write!(f, "ARRAY({})", inner),
            Self::Map(k, v) => write!(f, "MAP({}, {})", k, v),
            Self::Json => write!(f, "JSON"),
            Self::Float32Vector(dim) => write!(f, "FLOAT32_VECTOR({})", dim),
            Self::Struct(fields) => {
                write!(f, "STRUCT(")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field.name, field.data_type)?;
                }
                write!(f, ")")
            }
        }
    }
}
