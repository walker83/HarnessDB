pub mod error;

pub use error::{CatalogError, DrorisError};
pub type Result<T> = std::result::Result<T, DrorisError>;
