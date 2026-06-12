use thiserror::Error;

use c3d_core::AssetId;

/// Asset database error type.
#[derive(Debug, Error)]
pub enum AssetError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Asset was not found.
    #[error("asset not found: {0}")]
    NotFound(AssetId),
    /// Blob was not found.
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    /// Invalid content hash string.
    #[error("invalid content hash: {0}")]
    InvalidHash(String),
}

/// Result alias for asset operations.
pub type AssetResult<T> = Result<T, AssetError>;
