use c3d_core::EntityId;
use serde::{Deserialize, Serialize};

use crate::CommentId;

/// Comment lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentStatus {
    /// Active comment awaiting resolution.
    Open,
    /// Comment resolved but retained for audit.
    Resolved,
}

/// Entity-anchored collaboration comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneComment {
    /// Stable comment identifier.
    pub id: CommentId,
    /// Entity the comment is anchored to.
    pub entity_id: EntityId,
    /// Display name of the author.
    pub author_name: String,
    /// Comment body text.
    pub text: String,
    /// Open or resolved state.
    pub status: CommentStatus,
    /// Creation timestamp in milliseconds since UNIX epoch.
    pub created_at_ms: u64,
}

impl SceneComment {
    /// Create a new open comment.
    pub fn open(
        entity_id: EntityId,
        author_name: impl Into<String>,
        text: impl Into<String>,
        created_at_ms: u64,
    ) -> Self {
        Self {
            id: CommentId::new(),
            entity_id,
            author_name: author_name.into(),
            text: text.into(),
            status: CommentStatus::Open,
            created_at_ms,
        }
    }
}
