pub mod error;

pub use error::{DrorisError, CatalogError};
pub type Result<T> = std::result::Result<T, DrorisError>;
