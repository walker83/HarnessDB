use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub status_code: i32,
    pub error_msgs: Vec<String>,
}

impl Status {
    pub const OK: i32 = 0;
    pub const INTERNAL_ERROR: i32 = 1;
    pub const CATALOG_ERROR: i32 = 2;
    pub const ANALYSIS_ERROR: i32 = 3;

    pub fn ok() -> Self {
        Self { status_code: Self::OK, error_msgs: vec![] }
    }

    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self { status_code: code, error_msgs: vec![msg.into()] }
    }

    pub fn is_ok(&self) -> bool {
        self.status_code == Self::OK
    }
}
