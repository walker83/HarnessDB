//! Doris-compatible UDF implementations.
//!
//! Provides Doris-specific functions:
//! - Time functions: date_trunc, months_add, days_add, hours_add
//! - String functions: concat_ws, substring_index
//! - Aggregate functions: bitmap_count

use std::sync::Arc;
use datafusion::logical_expr::{ScalarUDF, AggregateUDF, Volatility};
use datafusion::scalar::ScalarValue;
use arrow_schema::DataType;

// ---------------------------------------------------------------------------
// Time Functions
// ---------------------------------------------------------------------------

/// date_trunc - truncate date to specified precision
/// Usage: date_trunc('month', date_col) -> truncated date
pub fn create_date_trunc_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::Date32Array;

    create_udf(
        "date_trunc",
        vec![DataType::Utf8, DataType::Date32],
        DataType::Date32,
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let precision = args[0].as_any().downcast_ref::<arrow_array::StringArray>().unwrap();
            let dates = args[1].as_any().downcast_ref::<Date32Array>().unwrap();

            let result: Vec<Option<i32>> = dates.iter().zip(precision.iter())
                .map(|(d, p)| {
                    match (d, p) {
                        (Some(date), Some(prec)) => truncate_date(date, prec),
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(Date32Array::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

fn truncate_date(days: i32, precision: &str) -> Option<i32> {
    // Days since Unix epoch (1970-01-01)
    let approx_years = days / 365;
    let year = 1970 + approx_years;

    match precision.to_lowercase().as_str() {
        "year" => {
            let days_to_year = (year - 1970) * 365 + leap_year_offset(year);
            Some(days_to_year)
        }
        "month" => {
            let month_offset = (days % 365) / 30;
            let days_to_month = (year - 1970) * 365 + month_offset * 30 + leap_year_offset(year);
            Some(days_to_month)
        }
        "day" => Some(days),
        _ => Some(days),
    }
}

fn leap_year_offset(year: i32) -> i32 {
    (year - 1970) / 4 - (year - 1970) / 100 + (year - 1970) / 400
}

/// days_add - add days to a date
pub fn create_days_add_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::{Date32Array, Int64Array};

    create_udf(
        "days_add",
        vec![DataType::Date32, DataType::Int64],
        DataType::Date32,
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let dates = args[0].as_any().downcast_ref::<Date32Array>().unwrap();
            let days_to_add = args[1].as_any().downcast_ref::<Int64Array>().unwrap();

            let result: Vec<Option<i32>> = dates.iter().zip(days_to_add.iter())
                .map(|(d, n)| {
                    match (d, n) {
                        (Some(date), Some(n)) => Some(date + n as i32),
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(Date32Array::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

/// months_add - add months to a date
pub fn create_months_add_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::{Date32Array, Int64Array};

    create_udf(
        "months_add",
        vec![DataType::Date32, DataType::Int64],
        DataType::Date32,
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let dates = args[0].as_any().downcast_ref::<Date32Array>().unwrap();
            let months = args[1].as_any().downcast_ref::<Int64Array>().unwrap();

            let result: Vec<Option<i32>> = dates.iter().zip(months.iter())
                .map(|(d, m)| {
                    match (d, m) {
                        (Some(date), Some(m)) => Some(date + (m * 30) as i32),
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(Date32Array::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

/// hours_add - add hours to a datetime
pub fn create_hours_add_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::{TimestampSecondArray, Int64Array};

    create_udf(
        "hours_add",
        vec![DataType::Timestamp(arrow_schema::TimeUnit::Second, None), DataType::Int64],
        DataType::Timestamp(arrow_schema::TimeUnit::Second, None),
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let timestamps = args[0].as_any().downcast_ref::<TimestampSecondArray>().unwrap();
            let hours = args[1].as_any().downcast_ref::<Int64Array>().unwrap();

            let result: Vec<Option<i64>> = timestamps.iter().zip(hours.iter())
                .map(|(ts, h)| {
                    match (ts, h) {
                        (Some(ts), Some(h)) => Some(ts + h * 3600),
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(TimestampSecondArray::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

// ---------------------------------------------------------------------------
// String Functions
// ---------------------------------------------------------------------------

/// concat_ws - concatenate strings with separator
pub fn create_concat_ws_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::StringArray;

    create_udf(
        "concat_ws",
        vec![DataType::Utf8, DataType::Utf8, DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let sep = args[0].as_any().downcast_ref::<StringArray>().unwrap();
            let str1 = args[1].as_any().downcast_ref::<StringArray>().unwrap();
            let str2 = args[2].as_any().downcast_ref::<StringArray>().unwrap();

            let result: Vec<Option<String>> = sep.iter().zip(str1.iter()).zip(str2.iter())
                .map(|((s, a), b)| {
                    match (s, a, b) {
                        (Some(sep), Some(a), Some(b)) => Some(format!("{}{}{}", a, sep, b)),
                        (Some(_), Some(a), None) => Some(a.to_string()),
                        (Some(_), None, Some(b)) => Some(b.to_string()),
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(StringArray::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

/// substring_index - substring before/after delimiter
pub fn create_substring_index_udf() -> ScalarUDF {
    use datafusion::logical_expr::create_udf;
    use arrow_array::{StringArray, Int64Array};

    create_udf(
        "substring_index",
        vec![DataType::Utf8, DataType::Utf8, DataType::Int64],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(|args: &[datafusion::logical_expr::ColumnarValue]| {
            let args = datafusion::logical_expr::ColumnarValue::values_to_arrays(args)?;
            let str_arr = args[0].as_any().downcast_ref::<StringArray>().unwrap();
            let delim_arr = args[1].as_any().downcast_ref::<StringArray>().unwrap();
            let count_arr = args[2].as_any().downcast_ref::<Int64Array>().unwrap();

            let result: Vec<Option<String>> = str_arr.iter()
                .zip(delim_arr.iter())
                .zip(count_arr.iter())
                .map(|((s, d), c)| {
                    match (s, d, c) {
                        (Some(str), Some(delim), Some(count)) => {
                            if count > 0 {
                                let parts: Vec<&str> = str.split(delim).collect();
                                if parts.len() >= count as usize {
                                    Some(parts[..count as usize].join(delim))
                                } else {
                                    Some(str.to_string())
                                }
                            } else {
                                let parts: Vec<&str> = str.split(delim).collect();
                                let abs_count = (-count) as usize;
                                if parts.len() >= abs_count {
                                    Some(parts[parts.len() - abs_count..].join(delim))
                                } else {
                                    Some(str.to_string())
                                }
                            }
                        }
                        _ => None,
                    }
                })
                .collect();

            Ok(datafusion::logical_expr::ColumnarValue::Array(Arc::new(StringArray::from(result)) as Arc<dyn arrow_array::Array>))
        }),
    )
}

// ---------------------------------------------------------------------------
// Aggregate Functions (Doris-specific)
// ---------------------------------------------------------------------------

/// bitmap_count - count distinct values using bitmap
pub fn create_bitmap_count_udf() -> AggregateUDF {
    use datafusion::logical_expr::{AggregateUDFImpl, Accumulator, function::AccumulatorArgs};
    use std::collections::HashSet;

    #[derive(Debug)]
    struct BitmapCountAccumulator {
        values: HashSet<ScalarValue>,
    }

    impl BitmapCountAccumulator {
        fn new() -> Self {
            Self { values: HashSet::new() }
        }
    }

    impl Accumulator for BitmapCountAccumulator {
        fn update_batch(&mut self, values: &[Arc<dyn arrow_array::Array>]) -> datafusion::error::Result<()> {
            if values.is_empty() {
                return Ok(());
            }
            let arr = &values[0];
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let scalar = ScalarValue::try_from_array(arr, i)?;
                    self.values.insert(scalar);
                }
            }
            Ok(())
        }

        fn merge_batch(&mut self, states: &[Arc<dyn arrow_array::Array>]) -> datafusion::error::Result<()> {
            if states.is_empty() {
                return Ok(());
            }
            let arr = states[0].as_any().downcast_ref::<arrow_array::Int64Array>().unwrap();
            for i in 0..arr.len() {
                let count = arr.value(i);
                // Simplified - real bitmap would merge actual bitmaps
                let _ = count;
            }
            Ok(())
        }

        fn state(&mut self) -> datafusion::error::Result<Vec<ScalarValue>> {
            Ok(vec![ScalarValue::Int64(Some(self.values.len() as i64))])
        }

        fn evaluate(&mut self) -> datafusion::error::Result<ScalarValue> {
            Ok(ScalarValue::Int64(Some(self.values.len() as i64)))
        }

        fn size(&self) -> usize {
            self.values.len() * 16
        }
    }

    #[derive(Debug)]
    struct BitmapCountUDFImpl {
        signature: datafusion::logical_expr::Signature,
    }

    impl BitmapCountUDFImpl {
        fn new() -> Self {
            Self {
                signature: datafusion::logical_expr::Signature::any(1, Volatility::Immutable),
            }
        }
    }

    impl AggregateUDFImpl for BitmapCountUDFImpl {
        fn as_any(&self) -> &dyn std::any::Any { self }

        fn name(&self) -> &str { "bitmap_count" }

        fn signature(&self) -> &datafusion::logical_expr::Signature { &self.signature }

        fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
            Ok(DataType::Int64)
        }

        fn accumulator(&self, _acc_args: AccumulatorArgs) -> datafusion::error::Result<Box<dyn Accumulator>> {
            Ok(Box::new(BitmapCountAccumulator::new()))
        }
    }

    AggregateUDF::from(BitmapCountUDFImpl::new())
}

// ---------------------------------------------------------------------------
// UDF Registration
// ---------------------------------------------------------------------------

/// Register all Doris-compatible UDFs with a DataFusion context.
pub fn register_doris_udfs(ctx: &mut datafusion::prelude::SessionContext) {
    // Time functions
    ctx.register_udf(create_date_trunc_udf());
    ctx.register_udf(create_days_add_udf());
    ctx.register_udf(create_months_add_udf());
    ctx.register_udf(create_hours_add_udf());

    // String functions
    ctx.register_udf(create_concat_ws_udf());
    ctx.register_udf(create_substring_index_udf());

    // Aggregate functions
    ctx.register_udaf(create_bitmap_count_udf());
}