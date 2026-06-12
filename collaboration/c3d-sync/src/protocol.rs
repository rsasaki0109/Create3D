use c3d_collab_core::{
    BranchProposal, ClientId, CommentId, OperationLogEntry, ProposalId, SceneComment, UserPresence,
};
use c3d_scene_ops::Transaction;
use serde::{Deserialize, Serialize};

/// Wire protocol version.
pub const SYNC_PROTOCOL_VERSION: u32 = 1;

/// Envelope for newline-delimited JSON sync messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEnvelope {
    /// Protocol version.
    pub version: u32,
    /// Message payload.
    pub message: SyncMessage,
}

impl SyncEnvelope {
    /// Wrap a message in the current protocol version.
    pub fn new(message: SyncMessage) -> Self {
        Self {
            version: SYNC_PROTOCOL_VERSION,
            message,
        }
    }

    /// Serialize to a single JSON line.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse a JSON line into an envelope.
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

/// Messages exchanged between sync clients and the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Client hello when joining a workspace.
    Hello {
        /// Workspace identifier.
        workspace_id: String,
        /// Display name for presence.
        user_name: String,
    },
    /// Server welcome with replay head and peer snapshot.
    Welcome {
        /// Assigned client id.
        client_id: ClientId,
        /// Current log head sequence.
        head_sequence: u64,
        /// Known peers at join time.
        peers: Vec<UserPresence>,
        /// Open comments snapshot.
        comments: Vec<SceneComment>,
        /// Branch proposals snapshot.
        proposals: Vec<BranchProposal>,
    },
    /// Push a local transaction to the shared log.
    PushTransaction {
        /// Transaction to append when supported by policy.
        transaction: Transaction,
    },
    /// Broadcast of a committed log entry.
    LogEntry {
        /// Committed operation log entry.
        entry: OperationLogEntry,
    },
    /// Presence update from a client.
    Presence {
        /// Updated presence state.
        presence: UserPresence,
    },
    /// Add or update a comment.
    CommentUpsert {
        /// Comment payload.
        comment: SceneComment,
    },
    /// Resolve or reopen a comment.
    CommentStatus {
        /// Target comment id.
        comment_id: CommentId,
        /// New status.
        status: c3d_collab_core::CommentStatus,
    },
    /// Share a branch/proposal bundle.
    BranchProposalShare {
        /// Proposal payload.
        proposal: BranchProposal,
    },
    /// Update branch proposal status.
    BranchProposalStatus {
        /// Target proposal id.
        proposal_id: ProposalId,
        /// New status.
        status: c3d_collab_core::ProposalStatus,
    },
    /// Policy or protocol error reported to client.
    Error {
        /// Human-readable message.
        message: String,
    },
}
