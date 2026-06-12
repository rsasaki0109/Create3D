use std::collections::HashMap;
use std::path::{Path, PathBuf};

use c3d_collab_core::{
    BranchProposal, CommentId, CommentStatus, ProposalId, ProposalStatus, SceneComment,
};
use c3d_core::EntityId;

/// Local collaboration document store for comments and proposals.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CollabStore {
    comments: HashMap<CommentId, SceneComment>,
    proposals: HashMap<ProposalId, BranchProposal>,
}

impl CollabStore {
    /// Load comments and proposals from a project collab directory.
    pub fn load(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref();
        let comments: Vec<SceneComment> = read_json(dir.join("comments.json"))?;
        let proposals: Vec<BranchProposal> = read_json(dir.join("proposals.json"))?;
        Ok(Self {
            comments: comments
                .into_iter()
                .map(|comment| (comment.id, comment))
                .collect(),
            proposals: proposals
                .into_iter()
                .map(|proposal| (proposal.id, proposal))
                .collect(),
        })
    }

    /// Persist comments and proposals to disk.
    pub fn save(&self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let comments: Vec<_> = self.comments.values().cloned().collect();
        let proposals: Vec<_> = self.proposals.values().cloned().collect();
        write_json(&dir.join("comments.json"), &comments)?;
        write_json(&dir.join("proposals.json"), &proposals)?;
        Ok(())
    }

    /// Upsert a comment.
    pub fn upsert_comment(&mut self, comment: SceneComment) {
        self.comments.insert(comment.id, comment);
    }

    /// Update comment status.
    pub fn set_comment_status(&mut self, comment_id: CommentId, status: CommentStatus) {
        if let Some(comment) = self.comments.get_mut(&comment_id) {
            comment.status = status;
        }
    }

    /// Comments anchored to an entity.
    pub fn comments_for_entity(&self, entity_id: EntityId) -> Vec<&SceneComment> {
        self.comments
            .values()
            .filter(|comment| comment.entity_id == entity_id)
            .collect()
    }

    /// All comments in stable order.
    pub fn comments(&self) -> Vec<SceneComment> {
        let mut values: Vec<_> = self.comments.values().cloned().collect();
        values.sort_by_key(|comment| comment.created_at_ms);
        values
    }

    /// Upsert a branch proposal.
    pub fn upsert_proposal(&mut self, proposal: BranchProposal) {
        self.proposals.insert(proposal.id, proposal);
    }

    /// Update proposal status.
    pub fn set_proposal_status(&mut self, proposal_id: ProposalId, status: ProposalStatus) {
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            proposal.status = status;
        }
    }

    /// All branch proposals.
    pub fn proposals(&self) -> Vec<BranchProposal> {
        self.proposals.values().cloned().collect()
    }
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: PathBuf) -> std::io::Result<T> {
    if !path.is_file() {
        let value: T = serde_json::from_str("[]")
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        return Ok(value);
    }
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, text)
}
