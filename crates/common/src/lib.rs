pub mod error;

pub use error::{CatalogError, DharnessError, ProcedureError};
pub type Result<T> = std::result::Result<T, DharnessError>;
