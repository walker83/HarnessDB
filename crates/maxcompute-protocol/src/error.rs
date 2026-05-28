//! Error types for the MaxCompute protocol layer.

use thiserror::Error;

/// Error type for MaxCompute operations.
#[derive(Debug, Error)]
pub enum McError {
    /// XML serialization/deserialization error.
    #[error("XML error: {0}")]
    XmlError(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Invalid request error.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience alias for `Result<T, McError>`.
pub type McResult<T> = Result<T, McError>;
