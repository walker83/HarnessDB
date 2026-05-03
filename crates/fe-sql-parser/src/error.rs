use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("syntax error at position {position}: {message}")]
    SyntaxError { position: usize, message: String },

    #[error("unsupported feature: {0}")]
    Unsupported(String),

    #[error("invalid SQL: {0}")]
    InvalidSql(String),
}
