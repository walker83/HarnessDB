use types::DataType as RorisDataType;

use arrow_schema as arrow_dt;

pub fn to_arrow_data_type(dt: &RorisDataType) -> arrow_dt::DataType {
    match dt {
        RorisDataType::Null => arrow_dt::DataType::Null,
        RorisDataType::Boolean => arrow_dt::DataType::Boolean,
        RorisDataType::Int8 => arrow_dt::DataType::Int8,
        RorisDataType::Int16 => arrow_dt::DataType::Int16,
        RorisDataType::Int32 => arrow_dt::DataType::Int32,
        RorisDataType::Int64 => arrow_dt::DataType::Int64,
        RorisDataType::Int128 => arrow_dt::DataType::Decimal128(38, 0),
        RorisDataType::Float32 => arrow_dt::DataType::Float32,
        RorisDataType::Float64 => arrow_dt::DataType::Float64,
        RorisDataType::Date => arrow_dt::DataType::Date32,
        RorisDataType::DateTime => arrow_dt::DataType::Timestamp(
            arrow_dt::TimeUnit::Second,
            None,
        ),
        RorisDataType::String
        | RorisDataType::Varchar(_)
        | RorisDataType::Char(_) => arrow_dt::DataType::Utf8,
        RorisDataType::Binary => arrow_dt::DataType::Binary,
        _ => arrow_dt::DataType::Utf8,
    }
}

pub fn from_arrow_data_type(dt: &arrow_dt::DataType) -> RorisDataType {
    match dt {
        arrow_dt::DataType::Null => RorisDataType::Null,
        arrow_dt::DataType::Boolean => RorisDataType::Boolean,
        arrow_dt::DataType::Int8 => RorisDataType::Int8,
        arrow_dt::DataType::Int16 => RorisDataType::Int16,
        arrow_dt::DataType::Int32 => RorisDataType::Int32,
        arrow_dt::DataType::Int64 => RorisDataType::Int64,
        arrow_dt::DataType::Float32 => RorisDataType::Float32,
        arrow_dt::DataType::Float64 => RorisDataType::Float64,
        arrow_dt::DataType::Date32 => RorisDataType::Date,
        arrow_dt::DataType::Timestamp(_, _) => RorisDataType::DateTime,
        arrow_dt::DataType::Utf8 => RorisDataType::String,
        arrow_dt::DataType::Binary => RorisDataType::Binary,
        arrow_dt::DataType::Decimal128(_, _) => RorisDataType::Int128,
        _ => RorisDataType::String,
    }
}

pub fn to_arrow_field(field: &types::Field) -> arrow_dt::Field {
    arrow_dt::Field::new(
        &field.name,
        to_arrow_data_type(&field.data_type),
        field.nullable,
    )
}

pub fn to_arrow_schema(schema: &types::Schema) -> arrow_dt::Schema {
    let fields: Vec<arrow_dt::Field> = schema.fields().iter().map(to_arrow_field).collect();
    arrow_dt::Schema::new(fields)
}
