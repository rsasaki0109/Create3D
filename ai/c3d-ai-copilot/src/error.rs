use thiserror::Error;

use c3d_ai_tool_protocol::ToolProtocolError;
use c3d_scene_doc::SceneError;

/// Errors raised by Copilot orchestration.
#[derive(Debug, Error)]
pub enum CopilotError {
    /// Tool protocol validation failed.
    #[error(transparent)]
    ToolProtocol(#[from] ToolProtocolError),
    /// Scene operation failed.
    #[error(transparent)]
    Scene(#[from] SceneError),
    /// Copilot could not interpret the request.
    #[error("{0}")]
    Unsupported(String),
    /// Identifier parsing failed.
    #[error("{0}")]
    InvalidInput(String),
    /// Remote model provider failed.
    #[error("remote provider error: {0}")]
    Remote(String),
}
