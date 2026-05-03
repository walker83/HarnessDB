use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("type mismatch: expected {expected}, got {actual}")]
    Mismatch { expected: String, actual: String },

    #[error("invalid cast from {from} to {to}")]
    InvalidCast { from: String, to: String },

    #[error("column error: {0}")]
    Column(String),
}
