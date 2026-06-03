use std::sync::Arc;
use types::DataType as HarnessDataType;

pub fn to_arrow_data_type(dt: &HarnessDataType) -> arrow_schema::DataType {
    match dt {
        HarnessDataType::Null => arrow_schema::DataType::Null,
        HarnessDataType::Boolean => arrow_schema::DataType::Boolean,
        HarnessDataType::Int8 => arrow_schema::DataType::Int8,
        HarnessDataType::Int16 => arrow_schema::DataType::Int16,
        HarnessDataType::Int32 => arrow_schema::DataType::Int32,
        HarnessDataType::Int64 => arrow_schema::DataType::Int64,
        // Int128 stored as Decimal128(38, 0) for wide compatibility
        HarnessDataType::Int128 => arrow_schema::DataType::Decimal128(38, 0),
        HarnessDataType::Float32 => arrow_schema::DataType::Float32,
        HarnessDataType::Float64 => arrow_schema::DataType::Float64,
        HarnessDataType::Decimal(d) => {
            // Safe conversion: u8 to i8 with bounds check.
            // Arrow's Decimal128 scale is i8, but Harness stores it as u8.
            // Clamp to i8::MAX if the scale exceeds the i8 range.
            let scale = if d.scale > i8::MAX as u8 {
                tracing::warn!(
                    "Decimal scale {} exceeds i8 range, clamping to {}",
                    d.scale,
                    i8::MAX
                );
                i8::MAX
            } else {
                d.scale as i8
            };
            arrow_schema::DataType::Decimal128(d.precision, scale)
        }
        HarnessDataType::Date => arrow_schema::DataType::Date32,
        HarnessDataType::DateTime => {
            arrow_schema::DataType::Timestamp(arrow_schema::TimeUnit::Second, None)
        }
        HarnessDataType::String | HarnessDataType::Varchar(_) | HarnessDataType::Char(_) => {
            arrow_schema::DataType::Utf8
        }
        HarnessDataType::Binary => arrow_schema::DataType::Binary,
        // JSON stored as UTF-8 string
        HarnessDataType::Json => arrow_schema::DataType::Utf8,
        HarnessDataType::Array(inner) => arrow_schema::DataType::List(Arc::new(
            arrow_schema::Field::new("item", to_arrow_data_type(inner), true),
        )),
        HarnessDataType::Map(key, value) => {
            let key_field = arrow_schema::Field::new("key", to_arrow_data_type(key), false);
            let value_field = arrow_schema::Field::new("value", to_arrow_data_type(value), true);
            let entries = arrow_schema::DataType::Struct(arrow_schema::Fields::from(vec![
                key_field,
                value_field,
            ]));
            arrow_schema::DataType::Map(
                Arc::new(arrow_schema::Field::new("entries", entries, false)),
                false,
            )
        }
        HarnessDataType::Struct(fields) => {
            let arrow_fields: Vec<arrow_schema::Field> = fields
                .iter()
                .map(|f| {
                    arrow_schema::Field::new(&f.name, to_arrow_data_type(&f.data_type), f.nullable)
                })
                .collect();
            arrow_schema::DataType::Struct(arrow_schema::Fields::from(arrow_fields))
        }
        HarnessDataType::Float32Vector(dim) => arrow_schema::DataType::FixedSizeList(
            Arc::new(arrow_schema::Field::new(
                "item",
                arrow_schema::DataType::Float32,
                false,
            )),
            *dim as i32,
        ),
        #[allow(unreachable_patterns)]
        _ => {
            tracing::warn!("Unknown Harness data type: {:?}, falling back to Utf8", dt);
            arrow_schema::DataType::Utf8
        }
    }
}

pub fn from_arrow_data_type(dt: &arrow_schema::DataType) -> HarnessDataType {
    match dt {
        arrow_schema::DataType::Null => HarnessDataType::Null,
        arrow_schema::DataType::Boolean => HarnessDataType::Boolean,
        arrow_schema::DataType::Int8 => HarnessDataType::Int8,
        arrow_schema::DataType::Int16 => HarnessDataType::Int16,
        arrow_schema::DataType::Int32 => HarnessDataType::Int32,
        arrow_schema::DataType::Int64 => HarnessDataType::Int64,
        arrow_schema::DataType::Float32 => HarnessDataType::Float32,
        arrow_schema::DataType::Float64 => HarnessDataType::Float64,
        arrow_schema::DataType::Date32 => HarnessDataType::Date,
        arrow_schema::DataType::Timestamp(_, _) => HarnessDataType::DateTime,
        arrow_schema::DataType::Utf8 => HarnessDataType::String,
        arrow_schema::DataType::Binary => HarnessDataType::Binary,
        arrow_schema::DataType::Decimal128(p, s) => {
            // Safe conversion: i8 to u8 with bounds check.
            // Arrow stores scale as i8, but negative scale is unusual for Harness.
            // Clamp to 0 if the scale is negative, warn if out of range.
            let scale = if *s < 0 {
                tracing::warn!("Arrow Decimal128 scale {} is negative, clamping to 0", s);
                0
            } else {
                *s as u8
            };
            HarnessDataType::Decimal(types::data_type::DecimalType {
                precision: *p,
                scale,
            })
        }
        arrow_schema::DataType::List(field) => {
            HarnessDataType::Array(Box::new(from_arrow_data_type(field.data_type())))
        }
        arrow_schema::DataType::Map(entries_field, _) => {
            if let arrow_schema::DataType::Struct(struct_fields) = entries_field.data_type() {
                let key_field = &struct_fields[0];
                let value_field = &struct_fields[1];
                HarnessDataType::Map(
                    Box::new(from_arrow_data_type(key_field.data_type())),
                    Box::new(from_arrow_data_type(value_field.data_type())),
                )
            } else {
                tracing::warn!(
                    "Unexpected Map entries type: {:?}",
                    entries_field.data_type()
                );
                HarnessDataType::String
            }
        }
        arrow_schema::DataType::Struct(arrow_fields) => {
            let fields: Vec<types::Field> = arrow_fields
                .iter()
                .map(|f| types::Field {
                    name: f.name().to_string(),
                    data_type: from_arrow_data_type(f.data_type()),
                    nullable: f.is_nullable(),
                })
                .collect();
            HarnessDataType::Struct(fields)
        }
        other => {
            tracing::warn!(
                "Unknown Arrow data type: {:?}, falling back to String",
                other
            );
            HarnessDataType::String
        }
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
