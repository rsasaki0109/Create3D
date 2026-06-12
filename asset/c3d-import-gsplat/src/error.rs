use thiserror::Error;

/// 3DGS PLY import error type.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(String),
    /// Unsupported or malformed PLY payload.
    #[error("invalid gsplat ply: {0}")]
    Invalid(String),
}

/// Result alias for 3DGS import operations.
pub type ImportResult<T> = Result<T, ImportError>;
