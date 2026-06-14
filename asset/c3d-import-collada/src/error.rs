use thiserror::Error;

/// Collada import error.
#[derive(Debug, Error)]
pub enum ImportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid Collada payload.
    #[error("invalid collada: {0}")]
    Invalid(String),
}

/// Result alias for Collada import.
pub type ImportResult<T> = Result<T, ImportError>;
