use c3d_ai_context::ContextPack;

use crate::{CopilotError, CopilotResponse};

/// Model provider interface for Copilot.
pub trait ModelProvider: Send + Sync {
    /// Produce an answer or edit proposal from user input and context.
    fn complete(
        &self,
        prompt: &str,
        context: &ContextPack,
    ) -> Result<CopilotResponse, CopilotError>;
}
