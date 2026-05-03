use thiserror::Error;

#[derive(Error, Debug)]
pub enum DrorisError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("query error: {0}")]
    Query(String),

    #[error("catalog error: {0}")]
    Catalog(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("plan error: {0}")]
    Plan(String),

    #[error("rpc error: {0}")]
    Rpc(String),

    #[error("internal error: {0}")]
    Internal(String),
}
