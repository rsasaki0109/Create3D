use thiserror::Error;

/// glTF import error type.
#[derive(Debug, Error)]
pub enum ImportError {
    /// Source file could not be read.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// glTF parsing failed.
    #[error("gltf error: {0}")]
    Gltf(String),
    /// Imported content was invalid.
    #[error("invalid import: {0}")]
    Invalid(String),
    /// Image decoding failed.
    #[error("image error: {0}")]
    Image(String),
}

/// Result alias for import operations.
pub type ImportResult<T> = Result<T, ImportError>;
