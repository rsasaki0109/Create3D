//! Copilot orchestration, tool execution, and preview planning.

#![warn(missing_docs)]

mod engine;
mod error;
mod executor;
mod mock_provider;
mod provider;
mod remote_stub_provider;
mod response;

pub use engine::CopilotEngine;
pub use error::CopilotError;
pub use executor::{ToolExecutionResult, ToolExecutor};
pub use mock_provider::MockModelProvider;
pub use provider::ModelProvider;
pub use remote_stub_provider::RemoteStubProvider;
pub use response::{CopilotProposal, CopilotResponse};

pub use c3d_scene_ops::TransactionProvenance;
