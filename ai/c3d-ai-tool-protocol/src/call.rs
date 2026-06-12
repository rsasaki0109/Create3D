use serde::{Deserialize, Serialize};

/// One AI tool invocation with JSON arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Registered tool name.
    pub tool: String,
    /// Tool-specific arguments encoded as JSON.
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a tool call with empty object arguments.
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            arguments: serde_json::Value::Object(Default::default()),
        }
    }

    /// Create a tool call with JSON arguments.
    pub fn with_arguments(tool: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            arguments,
        }
    }
}
