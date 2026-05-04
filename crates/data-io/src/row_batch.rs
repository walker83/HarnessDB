use types::{Block, Schema, ScalarValue, DataType, vector::*};

pub struct RowBatchBuilder {
    schema: Schema,
    columns: Vec<Vector>,
}

impl RowBatchBuilder {
    pub fn new(schema: Schema) -> Self {
        let columns = schema.fields().iter().map(|f| empty_vector_for_type(&f.data_type)).collect();
        Self { schema, columns }
    }

    pub fn with_columns(schema: Schema, columns: Vec<Vector>) -> Self {
        Self { schema, columns }
    }

    pub fn append_row(&mut self, values: &[ScalarValue]) {
        for (col_idx, value) in values.iter().enumerate() {
            if col_idx >= self.columns.len() {
                break;
            }
            push_scalar_to_column(&mut self.columns[col_idx], value);
        }
    }

    pub fn append_null(&mut self) {
        for col in &mut self.columns {
            push_null_to_column(col);
        }
    }

    pub fn finish(self) -> Block {
        Block::new(self.schema, self.columns)
    }

    pub fn num_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }
}

fn empty_vector_for_type(data_type: &DataType) -> Vector {
    match data_type {
        DataType::Boolean => Vector::Boolean(BooleanVector::new()),
        DataType::Int8 => Vector::Int8(Int8Vector::new()),
        DataType::Int16 => Vector::Int16(Int16Vector::new()),
        DataType::Int32 => Vector::Int32(Int32Vector::new()),
        DataType::Int64 => Vector::Int64(Int64Vector::new()),
        DataType::Int128 => Vector::Int128(Int128Vector::new()),
        DataType::Float32 => Vector::Float32(Float32Vector::new()),
        DataType::Float64 => Vector::Float64(Float64Vector::new()),
        DataType::Date => Vector::Date(DateVector::new()),
        DataType::DateTime => Vector::DateTime(DateTimeVector::new()),
        _ => Vector::String(StringVector::new()),
    }
}

fn push_scalar_to_column(column: &mut Vector, value: &ScalarValue) {
    use ScalarValue::*;

    match value {
        Boolean(b) => {
            if let Vector::Boolean(v) = column {
                v.push(Some(*b));
            }
        }
        Int8(n) => {
            if let Vector::Int8(v) = column {
                v.push(Some(*n));
            }
        }
        Int16(n) => {
            if let Vector::Int16(v) = column {
                v.push(Some(*n));
            }
        }
        Int32(n) => {
            if let Vector::Int32(v) = column {
                v.push(Some(*n));
            }
        }
        Int64(n) => {
            if let Vector::Int64(v) = column {
                v.push(Some(*n));
            }
        }
        Int128(n) => {
            if let Vector::Int128(v) = column {
                v.push(Some(*n));
            }
        }
        Float32(f) => {
            if let Vector::Float32(v) = column {
                v.push(Some(*f));
            }
        }
        Float64(f) => {
            if let Vector::Float64(v) = column {
                v.push(Some(*f));
            }
        }
        Date(d) => {
            if let Vector::Date(v) = column {
                v.push(Some(*d));
            }
        }
        DateTime(d) => {
            if let Vector::DateTime(v) = column {
                v.push(Some(*d));
            }
        }
        String(s) => {
            if let Vector::String(v) = column {
                v.push(Some(s.as_str()));
            }
        }
        Json(j) => {
            if let Vector::Json(v) = column {
                v.push(Some(ScalarValue::Json(j.clone())));
            }
        }
        Null | Binary(_) | Array(_) | Float32Array(_) => {
            push_null_to_column(column);
        }
    }
}

fn push_null_to_column(column: &mut Vector) {
    match column {
        Vector::Boolean(v) => v.push(None),
        Vector::Int8(v) => v.push(None),
        Vector::Int16(v) => v.push(None),
        Vector::Int32(v) => v.push(None),
        Vector::Int64(v) => v.push(None),
        Vector::Int128(v) => v.push(None),
        Vector::Float32(v) => v.push(None),
        Vector::Float64(v) => v.push(None),
        Vector::Date(v) => v.push(None),
        Vector::DateTime(v) => v.push(None),
        Vector::String(v) => v.push(None),
        Vector::Json(v) => v.push(None),
        Vector::Null(_) => {}
        Vector::Float32Array(v) => v.push(None),
    }
}

/// Convert a block back to rows
pub fn block_to_rows(block: &Block) -> Vec<Vec<ScalarValue>> {
    let num_rows = block.num_rows();
    let num_cols = block.num_columns();

    (0..num_rows).map(|row_idx| {
        (0..num_cols).map(|col_idx| {
            block.column(col_idx).unwrap().scalar_at(row_idx)
        }).collect()
    }).collect()
}

/// Create a RowBatchBuilder from an existing block
pub fn block_to_builder(block: &Block) -> RowBatchBuilder {
    RowBatchBuilder::with_columns(block.schema().clone(), block.columns().to_vec())
}

/// A row batch that can be iterated
pub struct RowBatch<'a> {
    block: &'a Block,
    current_row: usize,
}

impl<'a> RowBatch<'a> {
    pub fn new(block: &'a Block) -> Self {
        Self { block, current_row: 0 }
    }

    pub fn next_row(&mut self) -> Option<Vec<ScalarValue>> {
        if self.current_row >= self.block.num_rows() {
            return None;
        }
        let row = self.block.row(self.current_row);
        self.current_row += 1;
        Some(row)
    }

    pub fn reset(&mut self) {
        self.current_row = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::Field;

    #[test]
    fn test_row_batch_builder() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::String, true),
            Field::new("score", DataType::Float64, true),
        ]);

        let mut builder = RowBatchBuilder::new(schema);
        builder.append_row(&[
            ScalarValue::Int64(1),
            ScalarValue::String("Alice".to_string()),
            ScalarValue::Float64(95.5),
        ]);
        builder.append_row(&[
            ScalarValue::Int64(2),
            ScalarValue::String("Bob".to_string()),
            ScalarValue::Float64(87.3),
        ]);
        builder.append_null();

        let block = builder.finish();
        assert_eq!(block.num_rows(), 3);
        assert_eq!(block.num_columns(), 3);
    }

    #[test]
    fn test_block_to_rows() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::String, true),
        ]);

        let id_vec = Int64Vector::from_vec(vec![1, 2, 3]);
        let name_vec = StringVector::from_vec(vec!["Alice", "Bob", "Charlie"]);
        let block = Block::new(schema, vec![Vector::Int64(id_vec), Vector::String(name_vec)]);

        let rows = block_to_rows(&block);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![ScalarValue::Int64(1), ScalarValue::String("Alice".to_string())]);
    }
}