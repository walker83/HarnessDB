pub mod catalog;
pub mod information_schema;
pub mod table_provider;

pub use catalog::{ParquetCatalogProvider, ParquetSchemaProvider};
pub use information_schema::InformationSchemaProvider;
pub use table_provider::ParquetTableProvider;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_array::{ArrayRef, new_null_array};
use arrow_schema::{Field, Schema as ArrowSchema};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parquet error: {0}")]
    Parquet(String),
    #[error("Arrow error: {0}")]
    Arrow(String),
    #[error("Table not found: {0}.{1}")]
    TableNotFound(String, String),
    #[error("{0}")]
    Other(String),
}

impl From<String> for StorageError {
    fn from(s: String) -> Self {
        StorageError::Other(s)
    }
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Lightweight Parquet storage backend.
///
/// Data layout: `{data_dir}/{database}/{table}/data.parquet`
pub struct ParquetStorage {
    data_dir: PathBuf,
}

impl ParquetStorage {
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;
        debug!("ParquetStorage opened at {}", data_dir.display());
        Ok(Self { data_dir })
    }

    pub fn table_dir(&self, db: &str, table: &str) -> PathBuf {
        self.data_dir.join(db).join(table)
    }

    fn parquet_path(&self, db: &str, table: &str) -> PathBuf {
        self.table_dir(db, table).join("data.parquet")
    }

    pub fn table_exists(&self, db: &str, table: &str) -> bool {
        self.parquet_path(db, table).exists()
    }

    /// Create a new table: creates directory and writes an empty schema-only Parquet file.
    pub fn create_table(&self, db: &str, table: &str, schema: Arc<ArrowSchema>) -> Result<()> {
        let dir = self.table_dir(db, table);
        std::fs::create_dir_all(&dir)?;

        let path = dir.join("data.parquet");
        if !path.exists() {
            // Write an empty RecordBatch to establish schema
            let empty = self::write::empty_batch(&schema);
            self::write::write_parquet_atomic(&path, &empty)?;
        }
        debug!("Created table {}.{}", db, table);
        Ok(())
    }

    /// Drop a table: removes the table directory.
    pub fn drop_table(&self, db: &str, table: &str) -> Result<()> {
        let dir = self.table_dir(db, table);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        debug!("Dropped table {}.{}", db, table);
        Ok(())
    }

    /// Insert rows: read existing data, concatenate, write back atomically.
    pub fn insert(&self, db: &str, table: &str, new_batch: RecordBatch) -> Result<()> {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Err(StorageError::TableNotFound(db.to_string(), table.to_string()));
        }

        let num_new_rows = new_batch.num_rows();
        let existing = self::read::read_parquet(&path)?;
        let combined = if existing.num_rows() == 0 {
            new_batch
        } else {
            arrow_select::concat::concat_batches(&existing.schema(), &[existing, new_batch])
                .map_err(|e| StorageError::Arrow(e.to_string()))?
        };

        self::write::write_parquet_atomic(&path, &combined)?;
        debug!("Inserted {} rows into {}.{}", num_new_rows, db, table);
        Ok(())
    }

    /// Read all data from a table as a single RecordBatch.
    pub fn read(&self, db: &str, table: &str) -> Result<RecordBatch> {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Err(StorageError::TableNotFound(db.to_string(), table.to_string()));
        }
        self::read::read_parquet(&path)
    }

    /// Read with optional projection and limit.
    pub fn read_with_options(
        &self,
        db: &str,
        table: &str,
        projection: Option<&Vec<usize>>,
        limit: Option<usize>,
    ) -> Result<RecordBatch> {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Err(StorageError::TableNotFound(db.to_string(), table.to_string()));
        }
        self::read::read_parquet_with_options(&path, projection, limit)
    }

    /// Update rows: read, apply update function, write back.
    pub fn update<F>(&self, db: &str, table: &str, update_fn: F) -> Result<usize>
    where
        F: FnOnce(RecordBatch) -> Result<(RecordBatch, usize)>,
    {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Err(StorageError::TableNotFound(db.to_string(), table.to_string()));
        }

        let existing = self::read::read_parquet(&path)?;
        let (updated, count) = update_fn(existing)?;
        self::write::write_parquet_atomic(&path, &updated)?;
        debug!("Updated {} rows in {}.{}", count, db, table);
        Ok(count)
    }

    /// Delete rows: read, filter, write back.
    pub fn delete<F>(&self, db: &str, table: &str, filter_fn: F) -> Result<usize>
    where
        F: FnOnce(RecordBatch) -> Result<(RecordBatch, usize)>,
    {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Err(StorageError::TableNotFound(db.to_string(), table.to_string()));
        }

        let existing = self::read::read_parquet(&path)?;
        let (kept, deleted_count) = filter_fn(existing)?;
        self::write::write_parquet_atomic(&path, &kept)?;
        debug!("Deleted {} rows from {}.{}", deleted_count, db, table);
        Ok(deleted_count)
    }

    /// Rewrite the Parquet file by dropping a column at `col_index`.
    pub fn rewrite_parquet_drop_column(&self, db: &str, table: &str, col_index: usize) -> Result<()> {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Ok(());
        }
        let existing = self::read::read_parquet(&path)?;
        if existing.num_rows() == 0 {
            return Ok(());
        }
        // Project out the column
        let mut indices: Vec<usize> = (0..existing.num_columns()).collect();
        if col_index >= indices.len() {
            return Err(StorageError::Other(format!(
                "Column index {} out of range (num_columns={})",
                col_index,
                indices.len()
            )));
        }
        indices.remove(col_index);
        let projected = existing
            .project(&indices)
            .map_err(|e| StorageError::Arrow(e.to_string()))?;
        self::write::write_parquet_atomic(&path, &projected)?;
        Ok(())
    }

    /// Rewrite the Parquet file by appending a NULL column for existing rows.
    pub fn rewrite_parquet_add_column(&self, db: &str, table: &str, field: &Field) -> Result<()> {
        let path = self.parquet_path(db, table);
        if !path.exists() {
            return Ok(());
        }
        let existing = self::read::read_parquet(&path)?;
        if existing.num_rows() == 0 {
            return Ok(());
        }
        // Create null array for new column
        let null_array = new_null_array(field.data_type(), existing.num_rows());
        let mut fields: Vec<Field> = existing.schema().fields().iter().map(|f| f.as_ref().clone()).collect();
        fields.push(field.clone());
        let mut columns: Vec<ArrayRef> = existing.columns().to_vec();
        columns.push(null_array);
        let new_batch = RecordBatch::try_new(Arc::new(ArrowSchema::new(fields)), columns)
            .map_err(|e| StorageError::Arrow(e.to_string()))?;
        self::write::write_parquet_atomic(&path, &new_batch)?;
        Ok(())
    }

    /// Truncate a table: delete data file and recreate empty.
    pub fn truncate(&self, db: &str, table: &str, schema: Arc<ArrowSchema>) -> Result<()> {
        let path = self.parquet_path(db, table);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let empty = self::write::empty_batch(&schema);
        self::write::write_parquet_atomic(&path, &empty)?;
        debug!("Truncated table {}.{}", db, table);
        Ok(())
    }
}

mod write {
    use super::*;

    /// Create an empty RecordBatch with the given schema.
    pub fn empty_batch(schema: &Arc<ArrowSchema>) -> RecordBatch {
        let cols: Vec<Arc<dyn arrow_array::Array>> = schema
            .fields()
            .iter()
            .map(|f| arrow_array::new_empty_array(f.data_type()))
            .collect();
        RecordBatch::try_new(schema.clone(), cols)
            .expect("schema should be valid for empty batch (need at least one column)")
    }

    /// Write a RecordBatch to a Parquet file atomically (write temp + rename).
    pub fn write_parquet_atomic(path: &Path, batch: &RecordBatch) -> Result<()> {
        use parquet::arrow::ArrowWriter;
        use parquet::basic::Compression;
        use parquet::file::properties::{EnabledStatistics, WriterProperties};

        let dir = path.parent().ok_or_else(|| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path has no parent",
            ))
        })?;

        let temp_path = dir.join(format!(
            ".tmp_{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(parquet::basic::ZstdLevel::default()))
            .set_statistics_enabled(EnabledStatistics::Page)
            .build();

        let file = std::fs::File::create(&temp_path)?;
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
            .map_err(|e| StorageError::Parquet(e.to_string()))?;

        writer
            .write(batch)
            .map_err(|e| StorageError::Parquet(e.to_string()))?;
        writer
            .close()
            .map_err(|e| StorageError::Parquet(e.to_string()))?;

        // fsync before rename for crash safety
        std::fs::File::open(&temp_path)?.sync_all()?;
        std::fs::rename(&temp_path, path)?;
        debug!("Written {} rows to {}", batch.num_rows(), path.display());
        Ok(())
    }
}

mod read {
    use super::*;

    /// Read a Parquet file into a single RecordBatch.
    pub fn read_parquet(path: &Path) -> Result<RecordBatch> {
        read_parquet_with_options(path, None, None)
    }

    /// Read with optional column projection (by index) and row limit.
    pub fn read_parquet_with_options(
        path: &Path,
        projection: Option<&Vec<usize>>,
        limit: Option<usize>,
    ) -> Result<RecordBatch> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use parquet::arrow::ProjectionMask;

        let file = std::fs::File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| StorageError::Parquet(e.to_string()))?;

        let schema = builder.schema().clone();
        let parquet_schema = builder.metadata().file_metadata().schema_descr();

        let builder = if let Some(indices) = projection {
            let mask = ProjectionMask::leaves(parquet_schema, indices.iter().copied());
            builder.with_projection(mask)
        } else {
            builder
        };

        let reader = builder
            .build()
            .map_err(|e| StorageError::Parquet(e.to_string()))?;

        let mut batches = Vec::new();
        let mut total_rows = 0;

        for batch_result in reader {
            let batch = batch_result.map_err(|e| StorageError::Arrow(e.to_string()))?;
            if let Some(limit) = limit {
                if total_rows >= limit {
                    break;
                }
                let remaining = limit - total_rows;
                if batch.num_rows() > remaining {
                    batches.push(batch.slice(0, remaining));
                    break;
                }
            }
            total_rows += batch.num_rows();
            batches.push(batch);
        }

        if batches.is_empty() {
            let cols: Vec<Arc<dyn arrow_array::Array>> = schema
                .fields()
                .iter()
                .map(|f| arrow_array::new_empty_array(f.data_type()))
                .collect();
            return RecordBatch::try_new(schema, cols)
                .map_err(|e| StorageError::Arrow(e.to_string()));
        }

        if batches.len() == 1 {
            return Ok(batches.into_iter().next().unwrap());
        }

        arrow_select::concat::concat_batches(&batches[0].schema(), &batches)
            .map_err(|e| StorageError::Arrow(e.to_string()))
    }
}
