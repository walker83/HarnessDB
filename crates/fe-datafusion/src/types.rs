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

pub fn from_arrow_data_type(dt: &arrow_schema::DataType) -> RorisDataType {
    match dt {
        arrow_schema::DataType::Null => RorisDataType::Null,
        arrow_schema::DataType::Boolean => RorisDataType::Boolean,
        arrow_schema::DataType::Int8 => RorisDataType::Int8,
        arrow_schema::DataType::Int16 => RorisDataType::Int16,
        arrow_schema::DataType::Int32 => RorisDataType::Int32,
        arrow_schema::DataType::Int64 => RorisDataType::Int64,
        arrow_schema::DataType::Float32 => RorisDataType::Float32,
        arrow_schema::DataType::Float64 => RorisDataType::Float64,
        arrow_schema::DataType::Date32 => RorisDataType::Date,
        arrow_schema::DataType::Timestamp(_, _) => RorisDataType::DateTime,
        arrow_schema::DataType::Utf8 => RorisDataType::String,
        arrow_schema::DataType::Binary => RorisDataType::Binary,
        arrow_schema::DataType::Decimal128(_, _) => RorisDataType::Int128,
        _ => RorisDataType::String,
    }
}

pub fn to_arrow_field(field: &types::Field) -> arrow_schema::Field {
    arrow_schema::Field::new(
        &field.name,
        to_arrow_data_type(&field.data_type),
        field.nullable,
    )
}

pub fn to_arrow_schema(schema: &types::Schema) -> arrow_schema::Schema {
    let fields: Vec<arrow_schema::Field> = schema.fields().iter().map(to_arrow_field).collect();
    arrow_schema::Schema::new(fields)
}
