use c3d_ai_tool_protocol::ToolCall;
use c3d_scene_ops::{SceneOperation, Transaction, TransactionProvenance};

/// Response returned by a model provider.
#[derive(Debug, Clone, PartialEq)]
pub enum CopilotResponse {
    /// Read-only answer for the user.
    Answer(String),
    /// Scene edit proposal requiring preview and approval.
    Proposal(CopilotProposal),
}

/// Scene edit plan produced by Copilot.
#[derive(Debug, Clone, PartialEq)]
pub struct CopilotProposal {
    /// Short summary shown in the approval UI.
    pub summary: String,
    /// Tool calls that produced the plan.
    pub tool_calls: Vec<ToolCall>,
    /// Typed scene operations to preview/commit.
    pub operations: Vec<SceneOperation>,
    /// Provenance attached on commit.
    pub provenance: TransactionProvenance,
}

impl CopilotProposal {
    /// Build a transaction with provenance for commit or preview.
    pub fn into_transaction(self, transaction_id: c3d_core::TransactionId) -> Transaction {
        Transaction::with_provenance(transaction_id, self.operations, self.provenance)
    }
}
