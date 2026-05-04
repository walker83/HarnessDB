pub mod config;
pub mod error;
pub mod row;

pub use error::{DrorisError, StorageError, QueryError, CatalogError, ParseError, PlanError, RpcError};
pub type Result<T> = std::result::Result<T, DrorisError>;
