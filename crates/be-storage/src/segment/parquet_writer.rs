//! Parquet segment writer for Arrow RecordBatch.
//!
//! Writes RecordBatch to Parquet format with:
//! - ZSTD compression
//! - Column statistics (min/max/null_count) for predicate pushdown
//! - Bloom filters for high-cardinality columns

#[cfg(feature = "parquet-storage")]
use parquet::{
    file::properties::WriterProperties,
};
#[cfg(feature = "parquet-storage")]
use arrow_array::RecordBatch;
#[cfg(feature = "parquet-storage")]
use std::sync::Arc;
#[cfg(feature = "parquet-storage")]
use std::path::Path;
#[cfg(feature = "parquet-storage")]
use tracing::debug;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParquetWriteError {
    #[error("Parquet error: {0}")]
    ParquetError(String),
    #[error("Arrow error: {0}")]
    ArrowError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ParquetWriteError>;

/// Segment metadata returned after writing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParquetSegmentMeta {
    /// Segment file path (relative to tablet directory).
    pub path: String,
    /// Number of rows in this segment.
    pub num_rows: u64,
    /// File size in bytes.
    pub size: u64,
    /// Column statistics (for predicate pushdown).
    pub column_stats: Vec<ColumnStats>,
}

/// Column statistics for predicate pushdown.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColumnStats {
    pub column_name: String,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub null_count: u64,
    pub distinct_count: Option<u64>,
}

/// Parquet segment writer configuration.
#[derive(Debug, Clone)]
pub struct ParquetWriterConfig {
    /// Compression type (default: ZSTD).
    pub compression: Compression,
    /// Row group size (default: 64KB).
    pub row_group_size: usize,
    /// Enable bloom filters (default: true).
    pub enable_bloom_filter: bool,
    /// Bloom filter NDV threshold (default: 10000).
    pub bloom_filter_ndv_threshold: u64,
}

impl Default for ParquetWriterConfig {
    fn default() -> Self {
        Self {
            compression: Compression::ZSTD,
            row_group_size: 64 * 1024,
            enable_bloom_filter: true,
            bloom_filter_ndv_threshold: 10000,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Compression {
    Uncompressed,
    SNAPPY,
    GZIP,
    LZ4,
    ZSTD,
}

#[cfg(feature = "parquet-storage")]
impl From<Compression> for parquet::basic::Compression {
    fn from(c: Compression) -> Self {
        match c {
            Compression::Uncompressed => parquet::basic::Compression::UNCOMPRESSED,
            Compression::SNAPPY => parquet::basic::Compression::SNAPPY,
            Compression::GZIP => parquet::basic::Compression::GZIP(parquet::basic::GzipLevel::default()),
            Compression::LZ4 => parquet::basic::Compression::LZ4_RAW,
            Compression::ZSTD => parquet::basic::Compression::ZSTD(parquet::basic::ZstdLevel::default()),
        }
    }
}

/// Write a RecordBatch to a Parquet file.
#[cfg(feature = "parquet-storage")]
pub fn write_parquet_segment(
    path: &Path,
    batch: &RecordBatch,
    config: &ParquetWriterConfig,
) -> Result<ParquetSegmentMeta> {
    use parquet::arrow::ArrowWriter;
    use parquet::basic::Compression as ParquetCompression;

    // Configure writer properties
    let mut props_builder = WriterProperties::builder()
        .set_compression(ParquetCompression::from(config.compression));

    // Enable statistics - use the correct API
    use parquet::file::properties::EnabledStatistics;
    props_builder = props_builder.set_statistics_enabled(EnabledStatistics::Page);

    if config.enable_bloom_filter {
        props_builder = props_builder.set_bloom_filter_enabled(true);
    }

    let props = props_builder.build();

    // Create file
    let file = std::fs::File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema().clone(), Some(props))
        .map_err(|e| ParquetWriteError::ParquetError(e.to_string()))?;

    // Write batch
    writer.write(batch)
        .map_err(|e| ParquetWriteError::ParquetError(e.to_string()))?;

    // Close writer to get metadata
    let meta = writer.close()
        .map_err(|e| ParquetWriteError::ParquetError(e.to_string()))?;

    // Collect statistics
    let column_stats = collect_column_stats(batch);

    let result = ParquetSegmentMeta {
        path: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
        num_rows: batch.num_rows() as u64,
        size: std::fs::metadata(path)?.len(),
        column_stats,
    };

    debug!("Written Parquet segment: {} rows, {} bytes", result.num_rows, result.size);
    Ok(result)
}

#[cfg(feature = "parquet-storage")]
fn collect_column_stats(batch: &RecordBatch) -> Vec<ColumnStats> {
    use arrow_array::Array;
    use arrow_schema::Field;

    batch.schema().fields().iter().zip(batch.columns().iter())
        .map(|(field, array)| {
            ColumnStats {
                column_name: field.name().clone(),
                min_value: get_min_value(array, field),
                max_value: get_max_value(array, field),
                null_count: array.null_count() as u64,
                distinct_count: None, // Would require counting unique values
            }
        })
        .collect()
}

#[cfg(feature = "parquet-storage")]
fn get_min_value(array: &dyn arrow_array::Array, field: &arrow_schema::Field) -> Option<String> {
    use arrow_array::{Int64Array, Float64Array, StringArray, Date32Array};

    if array.is_empty() || array.null_count() == array.len() {
        return None;
    }

    // Get min based on type
    match array.data_type() {
        arrow_schema::DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            arr.iter().filter_map(|v| v).min().map(|v| v.to_string())
        }
        arrow_schema::DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            // Float64 doesn't implement Ord, use partial_cmp
            arr.iter()
                .filter_map(|v| v)
                .fold(None, |min: Option<f64>, v| {
                    match min {
                        None => Some(v),
                        Some(m) => if v < m { Some(v) } else { Some(m) }
                    }
                })
                .map(|v| v.to_string())
        }
        arrow_schema::DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            arr.iter().filter_map(|v| v).min().map(|v| v.to_string())
        }
        arrow_schema::DataType::Date32 => {
            let arr = array.as_any().downcast_ref::<Date32Array>().unwrap();
            arr.iter().filter_map(|v| v).min().map(|v| v.to_string())
        }
        _ => None,
    }
}

#[cfg(feature = "parquet-storage")]
fn get_max_value(array: &dyn arrow_array::Array, field: &arrow_schema::Field) -> Option<String> {
    use arrow_array::{Int64Array, Float64Array, StringArray, Date32Array};

    if array.is_empty() || array.null_count() == array.len() {
        return None;
    }

    match array.data_type() {
        arrow_schema::DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            arr.iter().filter_map(|v| v).max().map(|v| v.to_string())
        }
        arrow_schema::DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            // Float64 doesn't implement Ord, use partial_cmp
            arr.iter()
                .filter_map(|v| v)
                .fold(None, |max: Option<f64>, v| {
                    match max {
                        None => Some(v),
                        Some(m) => if v > m { Some(v) } else { Some(m) }
                    }
                })
                .map(|v| v.to_string())
        }
        arrow_schema::DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            arr.iter().filter_map(|v| v).max().map(|v| v.to_string())
        }
        arrow_schema::DataType::Date32 => {
            let arr = array.as_any().downcast_ref::<Date32Array>().unwrap();
            arr.iter().filter_map(|v| v).max().map(|v| v.to_string())
        }
        _ => None,
    }
}

/// Stub implementation when parquet-storage feature is disabled.
#[cfg(not(feature = "parquet-storage"))]
pub fn write_parquet_segment(
    _path: &Path,
    _batch: &[u8],
    _config: &ParquetWriterConfig,
) -> Result<ParquetSegmentMeta> {
    Err(ParquetWriteError::ParquetError("parquet-storage feature not enabled".to_string()))
}

#[cfg(test)]
#[cfg(feature = "parquet-storage")]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use arrow_array::{Int64Array, Float64Array, StringArray, RecordBatch};
    use arrow_schema::{Schema, Field, DataType};

    #[test]
    fn test_write_parquet_segment() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        // Create a simple RecordBatch
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
        let meta = write_parquet_segment(&path, &batch, &config).unwrap();

        assert_eq!(meta.num_rows, 5);
        assert!(meta.size > 0);
        assert_eq!(meta.column_stats.len(), 3);

        // Check id column stats
        let id_stats = &meta.column_stats[0];
        assert_eq!(id_stats.column_name, "id");
        assert_eq!(id_stats.min_value, Some("1".to_string()));
        assert_eq!(id_stats.max_value, Some("5".to_string()));
        assert_eq!(id_stats.null_count, 0);

        // Check value column stats (has nulls)
        let value_stats = &meta.column_stats[1];
        assert_eq!(value_stats.null_count, 1);
    }
}