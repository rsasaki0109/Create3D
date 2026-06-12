/// Result alias for RHI operations.
pub type RhiResult<T> = Result<T, RhiError>;

/// Errors returned by RHI backends.
#[derive(Debug, thiserror::Error)]
pub enum RhiError {
    /// Backend initialization failed.
    #[error("backend initialization failed: {0}")]
    Initialization(String),

    /// A GPU resource operation failed.
    #[error("gpu resource error: {0}")]
    Resource(String),

    /// Surface or swapchain failure.
    #[error("surface error: {0}")]
    Surface(String),

    /// Invalid handle or stale resource reference.
    #[error("invalid handle")]
    InvalidHandle,
}
