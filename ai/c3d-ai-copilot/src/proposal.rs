use c3d_ai_context::ContextPack;
use c3d_ai_tool_protocol::{validate_tool_call, ToolCall, ToolPermission, ToolRegistry};
use c3d_core::UlidGenerator;

use crate::error::CopilotError;
use crate::executor::ToolExecutor;
use crate::response::CopilotProposal;
use crate::TransactionProvenance;

/// Build a validated scene edit proposal from a tool call.
pub fn build_proposal(
    prompt: &str,
    call: ToolCall,
    summary: String,
    context: &ContextPack,
    model_id: &str,
) -> Result<CopilotProposal, CopilotError> {
    let registry = ToolRegistry::builtins();
    validate_tool_call(
        &registry,
        &call,
        &[ToolPermission::SceneRead, ToolPermission::SceneWrite],
    )?;

    let mut ids = UlidGenerator::new();
    let operations = ToolExecutor::compile_write(&call, context.selection.as_slice(), &mut ids)?;
    Ok(CopilotProposal {
        summary,
        tool_calls: vec![call.clone()],
        operations,
        provenance: TransactionProvenance {
            agent: "copilot".into(),
            user_prompt: prompt.to_string(),
            model_id: model_id.to_string(),
            tool_names: vec![call.tool],
        },
    })
}
