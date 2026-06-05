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
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Decimal(DecimalType),
    Date,
    DateTime,
    Time,
    DateTimeOffset,
    Varchar(usize),
    Char(usize),
    String,
    Binary,
    FixedSizeBinary(usize),
    Array(Box<DataType>),
    Map(Box<DataType>, Box<DataType>),
    Struct(Vec<Field>),
    Json,
    /// T-SQL MONEY type — fixed precision decimal(19,4)
    Money,
    /// T-SQL SMALLMONEY type — fixed precision decimal(10,4)
    SmallMoney,
    /// T-SQL UNIQUEIDENTIFIER — UUID/GUID stored as 16-byte fixed binary
    UniqueIdentifier,
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
            Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 => 4,
            Self::Int64 | Self::UInt64 => 8,
            Self::Int128 => 16,
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::Date => 4,
            Self::DateTime => 8,
            Self::Time => 8,
            Self::DateTimeOffset => 12,
            Self::Varchar(_) | Self::Char(_) | Self::String | Self::Binary => 16, // offset + length
            Self::FixedSizeBinary(n) => *n,
            Self::Decimal(_) => 16,
            Self::Money => 16,    // decimal(19,4)
            Self::SmallMoney => 8, // decimal(10,4)
            Self::UniqueIdentifier => 16, // UUID = 16 bytes
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
                | Self::UInt8
                | Self::UInt16
                | Self::UInt32
                | Self::UInt64
                | Self::Float32
                | Self::Float64
                | Self::Decimal(_)
                | Self::Money
                | Self::SmallMoney
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
            Self::UInt8 => write!(f, "UINT8"),
            Self::UInt16 => write!(f, "UINT16"),
            Self::UInt32 => write!(f, "UINT32"),
            Self::UInt64 => write!(f, "UINT64"),
            Self::Float32 => write!(f, "FLOAT32"),
            Self::Float64 => write!(f, "FLOAT64"),
            Self::Decimal(d) => write!(f, "DECIMAL({}, {})", d.precision, d.scale),
            Self::Date => write!(f, "DATE"),
            Self::DateTime => write!(f, "DATETIME"),
            Self::Time => write!(f, "TIME"),
            Self::DateTimeOffset => write!(f, "DATETIMEOFFSET"),
            Self::Varchar(n) => write!(f, "VARCHAR({})", n),
            Self::Char(n) => write!(f, "CHAR({})", n),
            Self::String => write!(f, "STRING"),
            Self::Binary => write!(f, "BINARY"),
            Self::FixedSizeBinary(n) => write!(f, "BINARY({})", n),
            Self::Array(inner) => write!(f, "ARRAY({})", inner),
            Self::Map(k, v) => write!(f, "MAP({}, {})", k, v),
            Self::Json => write!(f, "JSON"),
            Self::Money => write!(f, "MONEY"),
            Self::SmallMoney => write!(f, "SMALLMONEY"),
            Self::UniqueIdentifier => write!(f, "UNIQUEIDENTIFIER"),
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
