pub mod error;

pub use error::{CatalogError, DharnessError};
pub type Result<T> = std::result::Result<T, DharnessError>;
