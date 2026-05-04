use crate::{Schema, Vector, ScalarValue, Field};

#[derive(Debug, Clone)]
pub struct Block {
    schema: Schema,
    columns: Vec<Vector>,
}

impl Block {
    pub fn new(schema: Schema, columns: Vec<Vector>) -> Self {
        Self { schema, columns }
    }

    pub fn empty(schema: Schema) -> Self {
        let columns = schema.fields().iter().map(|f| empty_vector(&f.data_type)).collect();
        Self { schema, columns }
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn columns(&self) -> &[Vector] {
        &self.columns
    }

    pub fn column(&self, idx: usize) -> Option<&Vector> {
        self.columns.get(idx)
    }

    pub fn column_by_name(&self, name: &str) -> Option<(usize, &Vector)> {
        let idx = self.schema.index_of(name)?;
        Some((idx, &self.columns[idx]))
    }

    pub fn num_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.num_rows() == 0
    }

    pub fn row(&self, idx: usize) -> Vec<ScalarValue> {
        self.columns.iter().map(|c| c.scalar_at(idx)).collect()
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        let columns = self.columns.iter()
            .map(|c| c.slice(start, len))
            .collect();
        Self { schema: self.schema.clone(), columns }
    }

    pub fn project(&self, indices: &[usize]) -> Self {
        let schema = self.schema.project(indices);
        let columns: Vec<Vector> = indices.iter()
            .map(|&i| self.columns[i].clone())
            .collect();
        Self { schema, columns }
    }

    pub fn filter(&self, selection: &crate::Bitmap) -> Self {
        // Pre-count selected rows for preallocation
        let num_selected = selection.set_count();
        let num_cols = self.columns.len();

        // Preallocate all columns
        let columns: Vec<Vector> = self.columns.iter()
            .map(|c| c.filter(selection))
            .collect();

        Self { schema: self.schema.clone(), columns }
    }

    pub fn append_block(&mut self, other: &Block) {
        for (i, col) in self.columns.iter_mut().enumerate() {
            if i < other.columns.len() {
                col.append_vector(&other.columns[i]);
            }
        }
    }

    pub fn concat(blocks: &[Block]) -> Option<Block> {
        if blocks.is_empty() { return None; }
        let first = &blocks[0];
        let mut result = first.clone();
        for block in &blocks[1..] {
            result.append_block(block);
        }
        Some(result)
    }
}

fn empty_vector(dt: &crate::DataType) -> Vector {
    match dt {
        crate::DataType::Boolean => Vector::Boolean(crate::vector::BooleanVector::new()),
        crate::DataType::Int8 => Vector::Int8(crate::vector::Int8Vector::new()),
        crate::DataType::Int16 => Vector::Int16(crate::vector::Int16Vector::new()),
        crate::DataType::Int32 => Vector::Int32(crate::vector::Int32Vector::new()),
        crate::DataType::Int64 => Vector::Int64(crate::vector::Int64Vector::new()),
        crate::DataType::Int128 => Vector::Int128(crate::vector::Int128Vector::new()),
        crate::DataType::Float32 => Vector::Float32(crate::vector::Float32Vector::new()),
        crate::DataType::Float64 => Vector::Float64(crate::vector::Float64Vector::new()),
        crate::DataType::String => Vector::String(crate::vector::StringVector::new()),
        crate::DataType::Date => Vector::Date(crate::vector::DateVector::new()),
        crate::DataType::DateTime => Vector::DateTime(crate::vector::DateTimeVector::new()),
        _ => Vector::Null(crate::vector::NullVector::new(0)),
    }
}
