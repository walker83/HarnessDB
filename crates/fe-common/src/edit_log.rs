use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditLog {
    pub entries: Vec<EditLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditLogEntry {
    pub term: u64,
    pub index: u64,
    pub op_type: String,
    pub data: Vec<u8>,
}

impl EditLog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn append(&mut self, op_type: &str, data: Vec<u8>) {
        let index = self.entries.len() as u64;
        self.entries.push(EditLogEntry {
            term: 0,
            index,
            op_type: op_type.to_string(),
            data,
        });
    }
}

impl Default for EditLog {
    fn default() -> Self {
        Self::new()
    }
}
