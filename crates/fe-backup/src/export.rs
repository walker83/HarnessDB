//! Table export and import functionality

use std::path::Path;
use fe_storage::ParquetStorage;

/// Export table data to a file
pub fn export_table(
    storage: &ParquetStorage,
    database: &str,
    table: &str,
    path: &str,
    format: &str,
) -> Result<String, String> {
    let export_path = Path::new(path);

    // Create parent directory if needed
    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
    }

    match format.to_lowercase().as_str() {
        "parquet" => {
            // Copy the parquet file directly
            let src = storage.table_dir(database, table).join("data.parquet");
            if !src.exists() {
                return Err(format!("Table {}.{} has no data file", database, table));
            }
            std::fs::copy(&src, export_path)
                .map_err(|e| format!("Failed to export table: {}", e))?;

            let metadata = std::fs::metadata(export_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;

            // Read Parquet metadata to get the actual row count
            let file = std::fs::File::open(export_path)
                .map_err(|e| format!("Failed to open exported parquet: {}", e))?;
            let parquet_reader = parquet::file::reader::SerializedFileReader::new(file)
                .map_err(|e| format!("Failed to read parquet metadata: {}", e))?;
            let parquet_meta = parquet_reader.metadata();
            let num_rows: i64 = parquet_meta.file_metadata().num_rows();

            Ok(format!(
                "EXPORT TABLE `{}.{}` TO '{}' completed (parquet, {} rows, {} bytes)",
                database, table, path, num_rows, metadata.len()
            ))
        }
        "csv" => {
            // Read parquet and write as CSV
            let batch = storage.read(database, table)
                .map_err(|e| format!("Failed to read table: {}", e))?;

            let file = std::fs::File::create(export_path)
                .map_err(|e| format!("Failed to create CSV file: {}", e))?;

            let mut writer = arrow::csv::WriterBuilder::new()
                .with_header(true)
                .build(file);

            writer.write(&batch)
                .map_err(|e| format!("Failed to write CSV: {}", e))?;

            // Flush the writer to ensure all data is on disk before checking file size
            writer.finish()
                .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;

            let metadata = std::fs::metadata(export_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;

            Ok(format!(
                "EXPORT TABLE `{}.{}` TO '{}' completed (csv, {} rows, {} bytes)",
                database, table, path, batch.num_rows(), metadata.len()
            ))
        }
        _ => Err(format!("Unsupported export format: '{}'. Supported: parquet, csv", format)),
    }
}

/// Import table data from a file
pub fn import_table(
    storage: &ParquetStorage,
    database: &str,
    table: &str,
    path: &str,
    format: &str,
) -> Result<String, String> {
    let import_path = Path::new(path);

    if !import_path.exists() {
        return Err(format!("Import file '{}' not found", path));
    }

    match format.to_lowercase().as_str() {
        "parquet" => {
            // Read parquet and insert
            let file = std::fs::File::open(import_path)
                .map_err(|e| format!("Failed to open import file: {}", e))?;

            let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
                .map_err(|e| format!("Failed to read parquet file: {}", e))?
                .build()
                .map_err(|e| format!("Failed to build parquet reader: {}", e))?;

            let mut total_rows = 0;
            for batch_result in reader {
                let batch = batch_result
                    .map_err(|e| format!("Failed to read batch: {}", e))?;
                total_rows += batch.num_rows();
                storage.insert(database, table, batch)
                    .map_err(|e| format!("Failed to insert data: {}", e))?;
            }

            Ok(format!(
                "IMPORT TABLE `{}.{}` FROM '{}' completed (parquet, {} rows)",
                database, table, path, total_rows
            ))
        }
        "csv" => {
            // Read CSV and insert
            let file = std::fs::File::open(import_path)
                .map_err(|e| format!("Failed to open import file: {}", e))?;

            // We need the schema to read CSV - read existing table schema first
            let existing = storage.read(database, table);
            let schema = match existing {
                Ok(batch) => batch.schema(),
                Err(_) => {
                    return Err(format!(
                        "Table {}.{} must exist before importing CSV data",
                        database, table
                    ));
                }
            };

            let reader = arrow::csv::ReaderBuilder::new(schema)
                .with_header(true)
                .build(file)
                .map_err(|e| format!("Failed to build CSV reader: {}", e))?;

            let mut total_rows = 0;
            for batch_result in reader {
                let batch = batch_result
                    .map_err(|e| format!("Failed to read CSV batch: {}", e))?;
                total_rows += batch.num_rows();
                storage.insert(database, table, batch)
                    .map_err(|e| format!("Failed to insert data: {}", e))?;
            }

            Ok(format!(
                "IMPORT TABLE `{}.{}` FROM '{}' completed (csv, {} rows)",
                database, table, path, total_rows
            ))
        }
        _ => Err(format!("Unsupported import format: '{}'. Supported: parquet, csv", format)),
    }
}
