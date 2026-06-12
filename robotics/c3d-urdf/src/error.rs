use thiserror::Error;

/// URDF parse/import error.
#[derive(Debug, Error)]
pub enum UrdfError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// XML parse failure.
    #[error("xml parse error: {0}")]
    Xml(String),
    /// URDF content is invalid.
    #[error("invalid urdf: {0}")]
    Invalid(String),
}

/// Result alias for URDF operations.
pub type UrdfResult<T> = Result<T, UrdfError>;
