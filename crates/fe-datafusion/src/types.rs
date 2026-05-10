use types::DataType as RorisDataType;

use arrow_schema as arrow_dt;
use datafusion::prelude::Expr;
use datafusion::scalar::ScalarValue as DFScalarValue;

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

/// Convert DataFusion Expr to storage ReadPredicate.
/// Supports basic comparison predicates for pushdown.
pub fn expr_to_predicate(
    expr: &Expr,
    schema: &arrow_dt::Schema,
) -> Option<be_storage::segment::ReadPredicate> {
    use be_storage::segment::{ReadPredicate, ScalarValue};
    use datafusion::logical_expr::Operator;

    match expr {
        // Binary expressions (comparisons and logical operators)
        Expr::BinaryExpr(binary) => {
            let (left, op, right) = (binary.left.as_ref(), binary.op, binary.right.as_ref());

            // Handle logical operators (AND, OR)
            match op {
                Operator::And => {
                    let left_pred = expr_to_predicate(left, schema)?;
                    let right_pred = expr_to_predicate(right, schema)?;
                    Some(ReadPredicate::And(vec![left_pred, right_pred]))
                }
                Operator::Or => {
                    let left_pred = expr_to_predicate(left, schema)?;
                    let right_pred = expr_to_predicate(right, schema)?;
                    Some(ReadPredicate::Or(vec![left_pred, right_pred]))
                }
                _ => {
                    // Extract column name and value for comparison operators
                    let (col_name, value) = extract_column_and_value(left, right, schema)?;

                    match op {
                        Operator::Eq => {
                            Some(ReadPredicate::Eq { column: col_name, value })
                        }
                        Operator::NotEq => {
                            Some(ReadPredicate::NotEq { column: col_name, value })
                        }
                        Operator::Lt => {
                            Some(ReadPredicate::Range {
                                column: col_name,
                                min: ScalarValue::Null,
                                max: value,
                            })
                        }
                        Operator::LtEq => {
                            Some(ReadPredicate::Range {
                                column: col_name,
                                min: ScalarValue::Null,
                                max: value,
                            })
                        }
                        Operator::Gt => {
                            Some(ReadPredicate::Range {
                                column: col_name,
                                min: value,
                                max: ScalarValue::Null,
                            })
                        }
                        Operator::GtEq => {
                            Some(ReadPredicate::Range {
                                column: col_name,
                                min: value,
                                max: ScalarValue::Null,
                            })
                        }
                        _ => None,
                    }
                }
            }
        }

        // IS NULL
        Expr::IsNull(expr) => {
            let col_name = extract_column_name(expr, schema)?;
            Some(ReadPredicate::IsNull { column: col_name })
        }

        // IS NOT NULL
        Expr::IsNotNull(expr) => {
            let col_name = extract_column_name(expr, schema)?;
            Some(ReadPredicate::IsNotNull { column: col_name })
        }

        _ => None, // Unsupported predicate type
    }
}

/// Extract column name from an expression.
fn extract_column_name(expr: &Expr, schema: &arrow_dt::Schema) -> Option<String> {
    match expr {
        Expr::Column(col) => {
            // Handle qualified column names (table.column) or unqualified
            Some(col.name.clone())
        }
        Expr::Alias(alias) => {
            // Alias struct has public fields: expr and name
            // Check if alias name matches a column in schema
            if schema.fields().iter().any(|f| f.name() == &alias.name) {
                Some(alias.name.clone())
            } else {
                extract_column_name(&alias.expr, schema)
            }
        }
        _ => None,
    }
}

/// Extract column name and scalar value from binary expression operands.
fn extract_column_and_value(
    left: &Expr,
    right: &Expr,
    schema: &arrow_dt::Schema,
) -> Option<(String, be_storage::segment::ScalarValue)> {
    use be_storage::segment::ScalarValue;

    // Try left = column, right = value
    if let Some(col_name) = extract_column_name(left, schema) {
        if let Some(value) = extract_scalar_value(right) {
            return Some((col_name, value));
        }
    }

    // Try right = column, left = value (reversed comparison)
    if let Some(col_name) = extract_column_name(right, schema) {
        if let Some(value) = extract_scalar_value(left) {
            return Some((col_name, value));
        }
    }

    None
}

/// Extract scalar value from a literal expression.
fn extract_scalar_value(expr: &Expr) -> Option<be_storage::segment::ScalarValue> {
    use be_storage::segment::ScalarValue;

    match expr {
        Expr::Literal(lit) => {
            match lit {
                DFScalarValue::Int64(Some(v)) => Some(ScalarValue::Int64(*v)),
                DFScalarValue::Int32(Some(v)) => Some(ScalarValue::Int64(*v as i64)),
                DFScalarValue::Int16(Some(v)) => Some(ScalarValue::Int64(*v as i64)),
                DFScalarValue::Int8(Some(v)) => Some(ScalarValue::Int64(*v as i64)),
                DFScalarValue::Float64(Some(v)) => Some(ScalarValue::Float64(*v)),
                DFScalarValue::Float32(Some(v)) => Some(ScalarValue::Float64(*v as f64)),
                DFScalarValue::Utf8(Some(v)) => Some(ScalarValue::String(v.clone())),
                DFScalarValue::LargeUtf8(Some(v)) => Some(ScalarValue::String(v.clone())),
                DFScalarValue::Date32(Some(v)) => Some(ScalarValue::Date(*v)),
                DFScalarValue::Null => Some(ScalarValue::Null),
                _ => None,
            }
        }
        _ => None,
    }
}
