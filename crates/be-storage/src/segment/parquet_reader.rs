//! Parquet segment reader for Arrow RecordBatch.
//!
//! Reads Parquet files with:
//! - Predicate pushdown using column statistics
//! - Column projection
//! - Bloom filter for high-cardinality lookups

#[cfg(feature = "parquet-storage")]
use parquet::{
    file::reader::SerializedFileReader,
    arrow::ArrowReader,
    arrow::ParquetRecordBatchReaderBuilder,
};
#[cfg(feature = "parquet-storage")]
use arrow_array::RecordBatch;
#[cfg(feature = "parquet-storage")]
use arrow_schema::Schema as ArrowSchema;
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
#[derive(Debug, Clone)]
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
    use parquet::file::metadata::ParquetMetaData;

    // Open file
    let file = std::fs::File::open(path)?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    // Check predicates against metadata for early pruning
    let metadata = reader.metadata();
    if should_skip_based_on_stats(metadata, &options.predicates) {
        debug!("Skipping segment {} due to statistics pruning", path.display());
        // Return empty batch with correct schema
        return create_empty_batch_from_schema(metadata);
    }

    // Build reader with projection
    let builder = ParquetRecordBatchReaderBuilder::try_new(reader)
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    // Apply projection
    let builder = if let Some(proj) = &options.projection {
        let schema = builder.schema();
        let indices: Vec<usize> = proj.iter()
            .filter_map(|col| schema.index_of(col).ok())
            .collect();
        builder.with_projection(indices)
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
        return create_empty_batch_from_schema(metadata);
    }

    if batches.len() == 1 {
        return Ok(batches[0].clone());
    }

    // Concatenate batches
    arrow_array::compute::concat_batches(&batches[0].schema(), &batches)
        .map_err(|e| ParquetReadError::ArrowError(e.to_string()))
}

#[cfg(feature = "parquet-storage")]
fn should_skip_based_on_stats(
    metadata: &parquet::file::metadata::ParquetMetaData,
    predicates: &[ReadPredicate],
) -> bool {
    // Get row group metadata
    let row_groups = metadata.row_groups();

    for predicate in predicates {
        if can_prune_with_predicate(row_groups, predicate) {
            return true;
        }
    }

    false
}

#[cfg(feature = "parquet-storage")]
fn can_prune_with_predicate(
    row_groups: &[parquet::file::metadata::RowGroupMetaData],
    predicate: &ReadPredicate,
) -> bool {
    match predicate {
        ReadPredicate::Eq { column, value } => {
            // Check if value is outside min/max range in any row group
            for rg in row_groups {
                for col_meta in rg.columns() {
                    if col_meta.column_name() == column {
                        if let Some(stats) = col_meta.statistics() {
                            let min = stats.min_string();
                            let max = stats.max_string();
                            let val = value.to_string_repr();

                            // If value < min or value > max, can prune
                            if val < min || val > max {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        ReadPredicate::Range { column, min, max } => {
            // Check if range doesn't overlap with column min/max
            for rg in row_groups {
                for col_meta in rg.columns() {
                    if col_meta.column_name() == column {
                        if let Some(stats) = col_meta.statistics() {
                            let col_min = stats.min_string();
                            let col_max = stats.max_string();
                            let req_min = min.to_string_repr();
                            let req_max = max.to_string_repr();

                            // If requested range is entirely outside column range
                            if req_max < col_min || req_min > col_max {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        ReadPredicate::And(predicates) => {
            // If any sub-predicate can prune, we can prune
            predicates.iter().any(|p| can_prune_with_predicate(row_groups, p))
        }
        ReadPredicate::Or(predicates) => {
            // Only prune if ALL sub-predicates can prune
            predicates.iter().all(|p| can_prune_with_predicate(row_groups, p))
        }
        _ => false, // Other predicates don't support pruning
    }
}

#[cfg(feature = "parquet-storage")]
fn create_empty_batch_from_schema(
    metadata: &parquet::file::metadata::ParquetMetaData,
) -> Result<RecordBatch> {
    use parquet::arrow::parquet_to_arrow_schema;

    let parquet_schema = metadata.file_metadata().schema();
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
    let file = std::fs::File::open(path)?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| ParquetReadError::ParquetError(e.to_string()))?;

    let file_meta = reader.metadata().file_metadata();

    // Extract column statistics
    let column_stats: Vec<crate::segment::parquet_writer::ColumnStats> = reader
        .metadata()
        .row_groups()
        .first()
        .map(|rg| {
            rg.columns()
                .iter()
                .map(|col_meta| {
                    let stats = col_meta.statistics();
                    crate::segment::parquet_writer::ColumnStats {
                        column_name: col_meta.column_name().to_string(),
                        min_value: stats.map(|s| s.min_string()),
                        max_value: stats.map(|s| s.max_string()),
                        null_count: col_meta.null_count() as u64,
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

    let file = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 4];
    use std::io::Read;
    file.read_exact(&mut header).ok()?;

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