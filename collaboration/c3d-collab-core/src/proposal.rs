use c3d_scene_ops::{SceneOperation, TransactionProvenance};
use serde::{Deserialize, Serialize};

use crate::{ClientId, ProposalId};

/// Branch/proposal lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Draft proposal visible only to author.
    Draft,
    /// Shared proposal awaiting review.
    Proposed,
    /// Accepted and ready to merge.
    Accepted,
    /// Rejected by reviewers.
    Rejected,
}

/// Proposed scene edit bundle, often originating from AI Copilot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchProposal {
    /// Stable proposal identifier.
    pub id: ProposalId,
    /// Short title shown in collaboration UI.
    pub title: String,
    /// Author client id.
    pub author: ClientId,
    /// Author display name.
    pub author_name: String,
    /// Proposal lifecycle state.
    pub status: ProposalStatus,
    /// Scene operations contained in the proposal.
    pub operations: Vec<SceneOperation>,
    /// Optional provenance metadata when created by AI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<TransactionProvenance>,
}

impl BranchProposal {
    /// Create a new proposed branch from scene operations.
    pub fn propose(
        title: impl Into<String>,
        author: ClientId,
        author_name: impl Into<String>,
        operations: Vec<SceneOperation>,
        provenance: Option<TransactionProvenance>,
    ) -> Self {
        Self {
            id: ProposalId::new(),
            title: title.into(),
            author,
            author_name: author_name.into(),
            status: ProposalStatus::Proposed,
            operations,
            provenance,
        }
    }
}
