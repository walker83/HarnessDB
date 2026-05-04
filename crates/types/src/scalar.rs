use crate::DataType;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarValue {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Float32(f32),
    Float64(f64),
    Date(i32),
    DateTime(i64),
    String(String),
    Binary(Vec<u8>),
    Array(Vec<ScalarValue>),
    Json(JsonValue),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl ScalarValue {
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Null => DataType::Null,
            Self::Boolean(_) => DataType::Boolean,
            Self::Int8(_) => DataType::Int8,
            Self::Int16(_) => DataType::Int16,
            Self::Int32(_) => DataType::Int32,
            Self::Int64(_) => DataType::Int64,
            Self::Int128(_) => DataType::Int128,
            Self::Float32(_) => DataType::Float32,
            Self::Float64(_) => DataType::Float64,
            Self::Date(_) => DataType::Date,
            Self::DateTime(_) => DataType::DateTime,
            Self::String(_) => DataType::String,
            Self::Binary(_) => DataType::Binary,
            Self::Array(v) => {
                let inner = v.first().map(|s| s.data_type()).unwrap_or(DataType::Null);
                DataType::Array(Box::new(inner))
            }
            Self::Json(_) => DataType::Json,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}
