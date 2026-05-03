use crate::DataType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl Field {
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Self { name: name.into(), data_type, nullable }
    }

    pub fn not_null(name: impl Into<String>, data_type: DataType) -> Self {
        Self::new(name, data_type, false)
    }
}
