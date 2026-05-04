use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use std::fs::File;
use std::path::Path;
use types::{Block, Schema, DataType, Field, Vector};
use types::vector::{Int64Vector, Int32Vector, Float32Vector, Float64Vector, StringVector, BooleanVector};

use common::Result;

pub struct ParquetReader {
    reader: SerializedFileReader<File>,
    schema: Schema,
    batch_size: usize,
    current_row: usize,
    rows_buffer: Vec<parquet::record::Row>,
}

impl ParquetReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(|e| common::DrorisError::Internal(format!("IO error: {}", e)))?;
        let reader = SerializedFileReader::new(file).map_err(|e| common::DrorisError::Internal(format!("Parquet error: {}", e)))?;
        let schema = Self::parquet_schema_to_schema(reader.metadata().file_metadata().schema())?;
        Ok(Self {
            reader,
            schema,
            batch_size: 1024,
            current_row: 0,
            rows_buffer: Vec::new(),
        })
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    fn parquet_schema_to_schema(parquet_schema: &parquet::schema::types::Type) -> Result<Schema> {
        use parquet::basic::Type as PhysicalType;

        let fields: Vec<Field> = parquet_schema
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name().to_string();
                let data_type = match field.get_physical_type() {
                    PhysicalType::BOOLEAN => DataType::Boolean,
                    PhysicalType::INT32 => DataType::Int32,
                    PhysicalType::INT64 => DataType::Int64,
                    PhysicalType::FLOAT => DataType::Float32,
                    PhysicalType::DOUBLE => DataType::Float64,
                    PhysicalType::BYTE_ARRAY | PhysicalType::FIXED_LEN_BYTE_ARRAY => DataType::String,
                    _ => DataType::String,
                };
                Field::new(name, data_type, true)
            })
            .collect();

        Ok(Schema::new(fields))
    }

    pub fn next_batch(&mut self) -> Result<Option<Block>> {
        // Get row iterator for remaining rows
        let row_iter = self.reader.get_row_iter(None).map_err(|e| common::DrorisError::Internal(format!("Parquet error: {:?}", e)))?;

        self.rows_buffer.clear();

        for row_result in row_iter {
            match row_result {
                Ok(row) => {
                    self.rows_buffer.push(row);
                    if self.rows_buffer.len() >= self.batch_size {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if self.rows_buffer.is_empty() {
            return Ok(None);
        }

        let num_cols = self.schema.num_fields();
        let mut columns: Vec<Vector> = Vec::with_capacity(num_cols);

        for col_idx in 0..num_cols {
            let data_type = self.schema.field(col_idx).map(|f| f.data_type.clone()).unwrap_or(DataType::String);

            let vector = self.build_column(&self.rows_buffer, col_idx, &data_type);
            columns.push(vector);
        }

        self.current_row += self.rows_buffer.len();
        Ok(Some(Block::new(self.schema.clone(), columns)))
    }

    fn build_column(&self, rows: &[parquet::record::Row], col_idx: usize, data_type: &DataType) -> Vector {
        match data_type {
            DataType::Boolean => {
                let mut vec = BooleanVector::new();
                for row in rows {
                    match row.get_bool(col_idx) {
                        Ok(v) => vec.push(Some(v)),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::Boolean(vec)
            }
            DataType::Int32 => {
                let mut vec = Int32Vector::new();
                for row in rows {
                    match row.get_int(col_idx) {
                        Ok(v) => vec.push(Some(v)),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::Int32(vec)
            }
            DataType::Int64 => {
                let mut vec = Int64Vector::new();
                for row in rows {
                    match row.get_long(col_idx) {
                        Ok(v) => vec.push(Some(v)),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::Int64(vec)
            }
            DataType::Float32 => {
                let mut vec = Float32Vector::new();
                for row in rows {
                    match row.get_float(col_idx) {
                        Ok(v) => vec.push(Some(v)),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::Float32(vec)
            }
            DataType::Float64 => {
                let mut vec = Float64Vector::new();
                for row in rows {
                    match row.get_double(col_idx) {
                        Ok(v) => vec.push(Some(v)),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::Float64(vec)
            }
            DataType::String => {
                let mut vec = StringVector::new();
                for row in rows {
                    match row.get_string(col_idx) {
                        Ok(v) => vec.push(Some(v.as_str())),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::String(vec)
            }
            _ => {
                let mut vec = StringVector::new();
                for row in rows {
                    match row.get_string(col_idx) {
                        Ok(v) => vec.push(Some(v.as_str())),
                        Err(_) => vec.push(None),
                    }
                }
                Vector::String(vec)
            }
        }
    }

    pub fn num_rows(&self) -> usize {
        self.reader.metadata().file_metadata().num_rows() as usize
    }

    pub fn num_columns(&self) -> usize {
        self.schema.fields().len()
    }

    pub fn read_all(&mut self) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        while let Some(block) = self.next_batch()? {
            blocks.push(block);
        }
        Ok(blocks)
    }
}
