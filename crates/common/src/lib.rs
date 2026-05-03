pub mod config;
pub mod error;
pub mod row;

pub use error::DrorisError;
pub type Result<T> = std::result::Result<T, DrorisError>;
