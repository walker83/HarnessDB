use types::DataType as RorisDataType;

pub fn to_arrow_data_type(dt: &RorisDataType) -> arrow_schema::DataType {
    match dt {
        RorisDataType::Null => arrow_schema::DataType::Null,
        RorisDataType::Boolean => arrow_schema::DataType::Boolean,
        RorisDataType::Int8 => arrow_schema::DataType::Int8,
        RorisDataType::Int16 => arrow_schema::DataType::Int16,
        RorisDataType::Int32 => arrow_schema::DataType::Int32,
        RorisDataType::Int64 => arrow_schema::DataType::Int64,
        RorisDataType::Int128 => arrow_schema::DataType::Decimal128(38, 0),
        RorisDataType::Float32 => arrow_schema::DataType::Float32,
        RorisDataType::Float64 => arrow_schema::DataType::Float64,
        RorisDataType::Date => arrow_schema::DataType::Date32,
        RorisDataType::DateTime => arrow_schema::DataType::Timestamp(
            arrow_schema::TimeUnit::Second,
            None,
        ),
        RorisDataType::String
        | RorisDataType::Varchar(_)
        | RorisDataType::Char(_) => arrow_schema::DataType::Utf8,
        RorisDataType::Binary => arrow_schema::DataType::Binary,
        _ => arrow_schema::DataType::Utf8,
    }
}
