//! AI tool protocol schema, registry, and permission validation.

#![warn(missing_docs)]

mod call;
mod definition;
mod error;
mod registry;
mod validate;

pub use call::ToolCall;
pub use definition::{ToolDefinition, ToolPermission, ToolSideEffect};
pub use error::ToolProtocolError;
pub use registry::ToolRegistry;
pub use validate::validate_tool_call;
