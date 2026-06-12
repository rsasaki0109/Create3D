use thiserror::Error;

/// Errors raised while validating or dispatching AI tool calls.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolProtocolError {
    /// The requested tool is not registered.
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    /// The caller lacks a required permission.
    #[error("missing permission: {0:?}")]
    MissingPermission(super::ToolPermission),
    /// Tool arguments failed validation.
    #[error("invalid arguments for {tool}: {message}")]
    InvalidArguments {
        /// Tool name.
        tool: String,
        /// Validation message.
        message: String,
    },
}
