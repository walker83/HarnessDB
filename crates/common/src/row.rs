/// A row of data represented as a vector of values.
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub values: Vec<bytes::Bytes>,
}
