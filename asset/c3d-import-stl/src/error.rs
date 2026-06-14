use thiserror::Error;

/// STL import error.
#[derive(Debug, Error)]
pub enum ImportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid STL payload.
    #[error("invalid stl: {0}")]
    Invalid(String),
}

/// Result alias for STL import.
pub type ImportResult<T> = Result<T, ImportError>;
