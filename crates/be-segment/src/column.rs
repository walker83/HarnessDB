use types::DataType;

pub struct ColumnReader {
    pub column_name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl ColumnReader {
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Self {
            column_name: name.into(),
            data_type,
            nullable,
        }
    }

    pub fn read_page(&self, _page_idx: usize) -> Vec<u8> {
        // TODO: read column page from segment file
        vec![]
    }
}
