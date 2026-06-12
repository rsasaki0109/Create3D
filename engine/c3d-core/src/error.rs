//! Typed errors for `Create3D` core operations.

use thiserror::Error;

/// Result alias used across `Create3D` crates.
pub type C3dResult<T> = Result<T, C3dError>;

/// Core error type for foundational crates.
#[derive(Debug, Error)]
pub enum C3dError {
    /// Invalid identifier string or binary representation.
    #[error("invalid id: {0}")]
    InvalidId(String),

    /// Serialization or deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Generic validation failure.
    #[error("validation error: {0}")]
    Validation(String),

    /// An internal invariant was violated.
    #[error("internal error: {0}")]
    Internal(String),
}

impl C3dError {
    /// Create a validation error with the given message.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create an internal error with the given message.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}
