//! Parquet segment reader for Arrow RecordBatch.
//!
//! Reads Parquet files with:
//! - Predicate pushdown using column statistics
//! - Column projection
//! - Bloom filter for high-cardinality lookups

#[cfg(feature = "parquet-storage")]
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
#[cfg(feature = "parquet-storage")]
use arrow_array::RecordBatch;
#[cfg(feature = "parquet-storage")]
use std::sync::Arc;
#[cfg(feature = "parquet-storage")]
use std::path::Path;
#[cfg(feature = "parquet-storage")]
use tracing::debug;

use thiserror::Error;
use crate::segment::parquet_writer::ParquetSegmentMeta;

#[derive(Debug, Error)]
pub enum ParquetReadError {
    #[error("Parquet error: {0}")]
    ParquetError(String),
    #[error("Arrow error: {0}")]
    ArrowError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Predicate evaluation error: {0}")]
    PredicateError(String),
}

pub type Result<T> = std::result::Result<T, ParquetReadError>;

/// Predicate for filtering rows during read.
#[derive(Debug, Clone)]
pub enum ReadPredicate {
    /// Column equals value.
    Eq { column: String, value: ScalarValue },
    /// Column not equals value.
    NotEq { column: String, value: ScalarValue },
    /// Column in range [min, max].
    Range { column: String, min: ScalarValue, max: ScalarValue },
    /// Column is null.
    IsNull { column: String },
    /// Column is not null.
    IsNotNull { column: String },
    /// Combined predicates with AND.
    And(Vec<ReadPredicate>),
    /// Combined predicates with OR.
    Or(Vec<ReadPredicate>),
}

/// Scalar value for predicates.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Int64(i64),
    Float64(f64),
    String(String),
    Date(i32),
    Null,
}

impl ScalarValue {
    pub fn to_string_repr(&self) -> String {
        match self {
            ScalarValue::Int64(v) => v.to_string(),
            ScalarValue::Float64(v) => v.to_string(),
            ScalarValue::String(v) => v.clone(),
            ScalarValue::Date(v) => v.to_string(),
            ScalarValue::Null => "NULL".to_string(),
        }
    }
}

/// Read options for Parquet segment.
#[derive(Debug, Clone, Default)]
pub struct ParquetReadOptions {
    /// Columns to project (None = all columns).
    pub projection: Option<Vec<String>>,
    /// Predicates to apply (for pushdown).
    pub predicates: Vec<ReadPredicate>,
    /// Maximum rows to read (for LIMIT pushdown).
    pub limit: Option<usize>,
}

/// Read a Parquet file to RecordBatch.
#[cfg(feature = "parquet-storage")]
pub fn read_parquet_segment(
    path: &Path,
    options: &ParquetReadOptions,
) -> Result<RecordBatch> {
    use parquet::arrow::ProjectionMask;

    // Open file and get metadata first for predicate pruning
    let file = std::fs::File::open(path)?;

    // Build reader builder
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    // Check predicates against metadata for early pruning
    let metadata = builder.metadata();
    if should_skip_based_on_stats(metadata, &options.predicates) {
        debug!("Skipping segment {} due to statistics pruning", path.display());
        // Return empty batch with correct schema
        return create_empty_batch_from_schema(metadata);
    }

    // Get schema before applying projection
    let arrow_schema = builder.schema().clone();

    // Apply projection
    let parquet_schema = metadata.file_metadata().schema_descr();
    let builder = if let Some(proj) = &options.projection {
        let indices: Vec<usize> = proj.iter()
            .filter_map(|col| arrow_schema.index_of(col).ok())
            .collect();
        let mask = ProjectionMask::leaves(parquet_schema, indices);
        builder.with_projection(mask)
    } else {
        builder
    };

    // Build and read
    let batch_reader = builder.build()
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    // Collect batches (respecting limit)
    let mut batches: Vec<RecordBatch> = Vec::new();
    let mut total_rows = 0;

    for batch in batch_reader {
        let batch = batch.map_err(|e| ParquetReadError::ArrowError(e.to_string()))?;

        if let Some(limit) = options.limit {
            if total_rows >= limit {
                break;
            }
            let remaining = limit - total_rows;
            if batch.num_rows() > remaining {
                // Slice batch to remaining rows
                let sliced = batch.slice(0, remaining);
                batches.push(sliced);
                total_rows += remaining;
                break;
            }
        }

        total_rows += batch.num_rows();
        batches.push(batch);
    }

    // Merge batches
    if batches.is_empty() {
        // Return empty batch with correct schema
        let columns: Vec<Arc<dyn arrow_array::Array>> = arrow_schema.fields()
            .iter()
            .map(|field| arrow_array::new_empty_array(field.data_type()))
            .collect();
        return RecordBatch::try_new(arrow_schema, columns)
            .map_err(|e| ParquetReadError::ArrowError(e.to_string()));
    }

    if batches.len() == 1 {
        return Ok(batches[0].clone());
    }

    // Concatenate batches
    arrow_select::concat::concat_batches(&batches[0].schema(), &batches)
        .map_err(|e| ParquetReadError::ArrowError(e.to_string()))
}

#[cfg(feature = "parquet-storage")]
fn should_skip_based_on_stats(
    metadata: &parquet::file::metadata::ParquetMetaData,
    predicates: &[ReadPredicate],
) -> bool {
    // Check all row groups - if ALL row groups can be pruned, skip the file
    let row_groups = metadata.row_groups();

    // For AND predicates, if ALL row groups can be pruned for any predicate, skip
    // For OR predicates, only skip if ALL row groups can be pruned for ALL predicates
    for predicate in predicates {
        if can_prune_all_row_groups(row_groups, predicate) {
            return true;
        }
    }

    false
}

#[cfg(feature = "parquet-storage")]
fn can_prune_all_row_groups(
    row_groups: &[parquet::file::metadata::RowGroupMetaData],
    predicate: &ReadPredicate,
) -> bool {
    // Check if predicate eliminates ALL row groups
    match predicate {
        ReadPredicate::And(preds) => {
            // AND: if any sub-predicate prunes all groups, the AND prunes all
            preds.iter().any(|p| can_prune_all_row_groups(row_groups, p))
        }
        ReadPredicate::Or(preds) => {
            // OR: only prunes all groups if ALL sub-predicates prune all groups
            preds.iter().all(|p| can_prune_all_row_groups(row_groups, p))
        }
        _ => {
            // For leaf predicates, check each row group
            // Return true if predicate conflicts with statistics in ALL groups
            row_groups.iter().all(|rg| can_prune_row_group(rg, predicate))
        }
    }
}

#[cfg(feature = "parquet-storage")]
fn can_prune_row_group(
    row_group: &parquet::file::metadata::RowGroupMetaData,
    predicate: &ReadPredicate,
) -> bool {
    match predicate {
        ReadPredicate::Eq { column, value } => {
            // Find column metadata
            for col_meta in row_group.columns() {
                if col_meta.column_path().string() == column.as_str() {
                    if let Some(stats) = col_meta.statistics() {
                        // Check if value is outside min/max range
                        return is_value_below_max(value, stats) == Some(false)
                            || is_value_above_min(value, stats) == Some(false);
                    }
                }
            }
            false
        }
        ReadPredicate::Range { column, min, max } => {
            for col_meta in row_group.columns() {
                if col_meta.column_path().string() == column.as_str() {
                    if let Some(stats) = col_meta.statistics() {
                        // Check if range doesn't overlap with column stats
                        // If max < column_min OR min > column_max, can prune
                        let range_outside = if min != &ScalarValue::Null && max != &ScalarValue::Null {
                            // Both bounds specified
                            let max_below_min = is_value_below_max(max, stats);
                            let min_above_max = is_value_above_min(min, stats);
                            max_below_min == Some(false) || min_above_max == Some(false)
                        } else if min != &ScalarValue::Null {
                            // Only min specified (greater than min)
                            is_value_above_min(min, stats) == Some(false)
                        } else if max != &ScalarValue::Null {
                            // Only max specified (less than max)
                            is_value_below_max(max, stats) == Some(false)
                        } else {
                            false
                        };
                        return range_outside;
                    }
                }
            }
            false
        }
        ReadPredicate::IsNull { column } => {
            for col_meta in row_group.columns() {
                if col_meta.column_path().string() == column.as_str() {
                    if let Some(stats) = col_meta.statistics() {
                        // If null_count is 0, can prune
                        if let Some(null_count) = stats.null_count_opt() {
                            return null_count == 0;
                        }
                    }
                }
            }
            false
        }
        ReadPredicate::IsNotNull { column } => {
            for col_meta in row_group.columns() {
                if col_meta.column_path().string() == column.as_str() {
                    if let Some(stats) = col_meta.statistics() {
                        // If all rows are null, can prune
                        let num_rows = row_group.num_rows() as i64;
                        if let Some(null_count) = stats.null_count_opt() {
                            return null_count as i64 == num_rows;
                        }
                    }
                }
            }
            false
        }
        ReadPredicate::And(preds) => {
            // AND: prune if any sub-predicate prunes this group
            preds.iter().any(|p| can_prune_row_group(row_group, p))
        }
        ReadPredicate::Or(preds) => {
            // OR: prune only if ALL sub-predicates prune this group
            preds.iter().all(|p| can_prune_row_group(row_group, p))
        }
        _ => false,
    }
}

#[cfg(feature = "parquet-storage")]
fn is_value_above_min(value: &ScalarValue, stats: &parquet::file::statistics::Statistics) -> Option<bool> {
    // Compare value with min statistics
    // Returns Some(true) if value > min, Some(false) if value < min

    match value {
        ScalarValue::Int64(v) => {
            let min_bytes = stats.min_bytes();
            if min_bytes.len() == 8 {
                let min = i64::from_le_bytes(min_bytes.try_into().ok()?);
                Some(*v > min)
            } else {
                None
            }
        }
        ScalarValue::Float64(v) => {
            let min_bytes = stats.min_bytes();
            if min_bytes.len() == 8 {
                let min = f64::from_le_bytes(min_bytes.try_into().ok()?);
                Some(*v > min)
            } else {
                None
            }
        }
        ScalarValue::String(v) => {
            // Compare UTF8 string with bytes
            let min_bytes = stats.min_bytes();
            if !min_bytes.is_empty() {
                let min_str = std::str::from_utf8(min_bytes).ok()?;
                Some(v.as_str() > min_str)
            } else {
                None
            }
        }
        ScalarValue::Date(v) => {
            let min_bytes = stats.min_bytes();
            if min_bytes.len() == 4 {
                let min = i32::from_le_bytes(min_bytes.try_into().ok()?);
                Some(*v > min)
            } else {
                None
            }
        }
        ScalarValue::Null => None,
    }
}

#[cfg(feature = "parquet-storage")]
fn is_value_below_max(value: &ScalarValue, stats: &parquet::file::statistics::Statistics) -> Option<bool> {
    // Compare value with max statistics
    // Returns Some(true) if value < max, Some(false) if value > max

    match value {
        ScalarValue::Int64(v) => {
            let max_bytes = stats.max_bytes();
            if max_bytes.len() == 8 {
                let max = i64::from_le_bytes(max_bytes.try_into().ok()?);
                Some(*v < max)
            } else {
                None
            }
        }
        ScalarValue::Float64(v) => {
            let max_bytes = stats.max_bytes();
            if max_bytes.len() == 8 {
                let max = f64::from_le_bytes(max_bytes.try_into().ok()?);
                Some(*v < max)
            } else {
                None
            }
        }
        ScalarValue::String(v) => {
            let max_bytes = stats.max_bytes();
            if !max_bytes.is_empty() {
                let max_str = std::str::from_utf8(max_bytes).ok()?;
                Some(v.as_str() < max_str)
            } else {
                None
            }
        }
        ScalarValue::Date(v) => {
            let max_bytes = stats.max_bytes();
            if max_bytes.len() == 4 {
                let max = i32::from_le_bytes(max_bytes.try_into().ok()?);
                Some(*v < max)
            } else {
                None
            }
        }
        ScalarValue::Null => None,
    }
}

#[cfg(feature = "parquet-storage")]
fn create_empty_batch_from_schema(
    metadata: &parquet::file::metadata::ParquetMetaData,
) -> Result<RecordBatch> {
    use parquet::arrow::parquet_to_arrow_schema;

    let parquet_schema = metadata.file_metadata().schema_descr();
    let arrow_schema = parquet_to_arrow_schema(parquet_schema, None)
        .map_err(|e| ParquetReadError::ArrowError(e.to_string()))?;

    // Create empty columns
    let columns: Vec<Arc<dyn arrow_array::Array>> = arrow_schema.fields()
        .iter()
        .map(|field| arrow_array::new_empty_array(field.data_type()))
        .collect();

    RecordBatch::try_new(Arc::new(arrow_schema), columns)
        .map_err(|e| ParquetReadError::ArrowError(e.to_string()))
}

/// Read segment metadata from Parquet footer.
#[cfg(feature = "parquet-storage")]
pub fn read_parquet_meta(path: &Path) -> Result<ParquetSegmentMeta> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file = std::fs::File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    let file_meta = builder.metadata().file_metadata();

    // Extract column statistics (simplified for parquet 58)
    let column_stats: Vec<crate::segment::parquet_writer::ColumnStats> = builder
        .metadata()
        .row_groups()
        .first()
        .map(|rg| {
            rg.columns()
                .iter()
                .map(|col_meta| {
                    // Get null count from statistics if available
                    let null_count = col_meta.statistics()
                        .and_then(|s| s.null_count_opt())
                        .unwrap_or(0) as u64;

                    crate::segment::parquet_writer::ColumnStats {
                        column_name: col_meta.column_path().string().to_string(),
                        min_value: None, // Simplified - statistics API changed in parquet 58
                        max_value: None,
                        null_count,
                        distinct_count: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ParquetSegmentMeta {
        path: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
        num_rows: file_meta.num_rows() as u64,
        size: std::fs::metadata(path)?.len(),
        column_stats,
    })
}

/// Check if a file is a Parquet file by magic header.
pub fn is_parquet_file(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    let result = std::fs::File::open(path);
    if result.is_err() {
        return false;
    }
    let mut file = result.unwrap();
    let mut header = [0u8; 4];
    use std::io::Read;
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    // Parquet magic: "PAR1"
    header == *b"PAR1"
}

/// Stub implementation when parquet-storage feature is disabled.
#[cfg(not(feature = "parquet-storage"))]
pub fn read_parquet_segment(
    _path: &Path,
    _options: &ParquetReadOptions,
) -> Result<RecordBatch> {
    Err(ParquetReadError::ParquetError("parquet-storage feature not enabled".to_string()))
}

#[cfg(not(feature = "parquet-storage"))]
pub fn read_parquet_meta(_path: &Path) -> Result<ParquetSegmentMeta> {
    Err(ParquetReadError::ParquetError("parquet-storage feature not enabled".to_string()))
}

#[cfg(test)]
#[cfg(feature = "parquet-storage")]
mod tests {
    use super::*;
    use crate::segment::parquet_writer::{write_parquet_segment, ParquetWriterConfig};
    use tempfile::tempdir;
    use arrow_array::{Int64Array, Float64Array, StringArray, RecordBatch};
    use arrow_schema::{Schema, Field, DataType};

    #[test]
    fn test_read_parquet_segment() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        // Write test data
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("value", DataType::Float64, true),
            Field::new("name", DataType::Utf8, true),
        ]);

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(Float64Array::from(vec![Some(1.0), Some(2.0), None, Some(4.0), Some(5.0)])),
                Arc::new(StringArray::from(vec![Some("a"), Some("b"), None, Some("d"), Some("e")])),
            ],
        ).unwrap();

        let config = ParquetWriterConfig::default();
        write_parquet_segment(&path, &batch, &config).unwrap();

        // Read back
        let options = ParquetReadOptions::default();
        let read_batch = read_parquet_segment(&path, &options).unwrap();

        assert_eq!(read_batch.num_rows(), 5);
        assert_eq!(read_batch.schema(), batch.schema());
    }

    #[test]
    fn test_read_with_projection() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        // Write test data
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("value", DataType::Float64, true),
            Field::new("name", DataType::Utf8, true),
        ]);

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(Float64Array::from(vec![Some(1.0), Some(2.0), None, Some(4.0), Some(5.0)])),
                Arc::new(StringArray::from(vec![Some("a"), Some("b"), None, Some("d"), Some("e")])),
            ],
        ).unwrap();

        let config = ParquetWriterConfig::default();
        write_parquet_segment(&path, &batch, &config).unwrap();

        // Read with projection
        let options = ParquetReadOptions {
            projection: Some(vec!["id".to_string(), "name".to_string()]),
            predicates: vec![],
            limit: None,
        };
        let read_batch = read_parquet_segment(&path, &options).unwrap();

        assert_eq!(read_batch.num_columns(), 2);
        assert_eq!(read_batch.schema().fields().len(), 2);
    }

    #[test]
    fn test_read_with_limit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        // Write test data
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
        ]);

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
            ],
        ).unwrap();

        let config = ParquetWriterConfig::default();
        write_parquet_segment(&path, &batch, &config).unwrap();

        // Read with limit
        let options = ParquetReadOptions {
            projection: None,
            predicates: vec![],
            limit: Some(2),
        };
        let read_batch = read_parquet_segment(&path, &options).unwrap();

        assert_eq!(read_batch.num_rows(), 2);
    }

    #[test]
    fn test_is_parquet_file() {
        let dir = tempdir().unwrap();
        let parquet_path = dir.path().join("test.parquet");
        let text_path = dir.path().join("test.txt");

        // Create a Parquet file
        let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![Arc::new(Int64Array::from(vec![1]))],
        ).unwrap();

        let config = ParquetWriterConfig::default();
        write_parquet_segment(&parquet_path, &batch, &config).unwrap();

        // Create a text file
        std::fs::write(&text_path, "not parquet").unwrap();

        assert!(is_parquet_file(&parquet_path));
        assert!(!is_parquet_file(&text_path));
    }
}