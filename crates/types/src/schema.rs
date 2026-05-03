use crate::{DataType, Field};

#[derive(Debug, Clone)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn empty() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn field(&self, idx: usize) -> Option<&Field> {
        self.fields.get(idx)
    }

    pub fn num_fields(&self) -> usize {
        self.fields.len()
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    pub fn data_types(&self) -> Vec<&DataType> {
        self.fields.iter().map(|f| &f.data_type).collect()
    }

    pub fn project(&self, indices: &[usize]) -> Self {
        let fields: Vec<Field> = indices.iter()
            .filter_map(|&i| self.fields.get(i).cloned())
            .collect();
        Self { fields }
    }
}

impl FromIterator<Field> for Schema {
    fn from_iter<I: IntoIterator<Item = Field>>(iter: I) -> Self {
        Self { fields: iter.into_iter().collect() }
    }
}
