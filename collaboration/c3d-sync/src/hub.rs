use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use c3d_collab_core::{ClientId, OperationLog, OperationLogEntry, UserPresence};

use crate::policy::{filter_syncable_transaction, SyncPolicyError};
use crate::protocol::{SyncEnvelope, SyncMessage};
use crate::store::CollabStore;

/// In-memory sync hub used by the server and tests.
#[derive(Debug)]
pub struct SyncHub {
    workspace_id: String,
    log: OperationLog,
    log_path: Option<PathBuf>,
    store: CollabStore,
    store_dir: Option<PathBuf>,
    clients: HashMap<ClientId, UserPresence>,
}

impl SyncHub {
    /// Create a hub for a workspace.
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            log: OperationLog::new(),
            log_path: None,
            store: CollabStore::default(),
            store_dir: None,
            clients: HashMap::new(),
        }
    }

    /// Attach JSONL persistence for the operation log.
    pub fn with_log_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_path = Some(path.into());
        self
    }

    /// Attach on-disk store directory for comments and proposals.
    pub fn with_store_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.store_dir = Some(dir.into());
        self
    }

    /// Load persisted state from disk when paths are configured.
    pub fn load_persisted(&mut self) -> std::io::Result<()> {
        if let Some(path) = self.log_path.clone() {
            self.log = OperationLog::load(path)?;
        }
        if let Some(dir) = self.store_dir.clone() {
            self.store = CollabStore::load(dir)?;
        }
        Ok(())
    }

    /// Handle an incoming client message and return outbound messages.
    pub fn handle(
        &mut self,
        client_id: Option<ClientId>,
        message: SyncMessage,
    ) -> Result<(Option<ClientId>, Vec<SyncEnvelope>), SyncHubError> {
        match message {
            SyncMessage::Hello {
                workspace_id,
                user_name,
            } => {
                if workspace_id != self.workspace_id {
                    return Err(SyncHubError::WorkspaceMismatch);
                }
                let client_id = ClientId::new();
                let presence = UserPresence::new(client_id, user_name);
                self.clients.insert(client_id, presence.clone());
                Ok((
                    Some(client_id),
                    vec![SyncEnvelope::new(SyncMessage::Welcome {
                        client_id,
                        head_sequence: self.log.head_sequence(),
                        peers: self.clients.values().cloned().collect(),
                        comments: self.store.comments(),
                        proposals: self.store.proposals(),
                    })],
                ))
            }
            SyncMessage::PushTransaction { transaction } => {
                let client_id = client_id.ok_or(SyncHubError::NotJoined)?;
                let filtered = filter_syncable_transaction(&transaction)?;
                let entry = OperationLogEntry {
                    sequence: self.log.head_sequence() + 1,
                    branch: "main".into(),
                    author: client_id,
                    author_name: self
                        .clients
                        .get(&client_id)
                        .map(|presence| presence.user_name.clone())
                        .unwrap_or_else(|| "unknown".into()),
                    transaction: filtered,
                    timestamp_ms: now_ms(),
                };
                self.log.append(entry.clone(), self.log_path.as_deref())?;
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::LogEntry { entry })],
                ))
            }
            SyncMessage::Presence { mut presence } => {
                let client_id = client_id.ok_or(SyncHubError::NotJoined)?;
                if presence.client_id != client_id {
                    return Err(SyncHubError::ClientMismatch);
                }
                if presence.user_name.is_empty() {
                    if let Some(existing) = self.clients.get(&client_id) {
                        presence.user_name = existing.user_name.clone();
                    }
                }
                self.clients.insert(client_id, presence.clone());
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::Presence { presence })],
                ))
            }
            SyncMessage::CommentUpsert { comment } => {
                let _ = client_id.ok_or(SyncHubError::NotJoined)?;
                self.store.upsert_comment(comment.clone());
                if let Some(dir) = self.store_dir.clone() {
                    self.store.save(dir)?;
                }
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::CommentUpsert { comment })],
                ))
            }
            SyncMessage::CommentStatus { comment_id, status } => {
                let _ = client_id.ok_or(SyncHubError::NotJoined)?;
                self.store.set_comment_status(comment_id, status);
                if let Some(dir) = self.store_dir.clone() {
                    self.store.save(dir)?;
                }
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::CommentStatus {
                        comment_id,
                        status,
                    })],
                ))
            }
            SyncMessage::BranchProposalShare { proposal } => {
                let _ = client_id.ok_or(SyncHubError::NotJoined)?;
                self.store.upsert_proposal(proposal.clone());
                if let Some(dir) = self.store_dir.clone() {
                    self.store.save(dir)?;
                }
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::BranchProposalShare {
                        proposal,
                    })],
                ))
            }
            SyncMessage::BranchProposalStatus {
                proposal_id,
                status,
            } => {
                let _ = client_id.ok_or(SyncHubError::NotJoined)?;
                self.store.set_proposal_status(proposal_id, status);
                if let Some(dir) = self.store_dir.clone() {
                    self.store.save(dir)?;
                }
                Ok((
                    None,
                    vec![SyncEnvelope::new(SyncMessage::BranchProposalStatus {
                        proposal_id,
                        status,
                    })],
                ))
            }
            SyncMessage::Welcome { .. }
            | SyncMessage::LogEntry { .. }
            | SyncMessage::Error { .. } => Err(SyncHubError::ClientOnlyMessage),
        }
    }

    /// Remove a disconnected client from presence tracking.
    pub fn disconnect(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }

    /// Borrow the operation log.
    pub fn log(&self) -> &OperationLog {
        &self.log
    }

    /// Borrow the collaboration store.
    pub fn store(&self) -> &CollabStore {
        &self.store
    }
}

/// Hub-side failures.
#[derive(Debug, thiserror::Error)]
pub enum SyncHubError {
    /// Client attempted to join the wrong workspace.
    #[error("workspace mismatch")]
    WorkspaceMismatch,
    /// Client sent messages before hello.
    #[error("client has not joined")]
    NotJoined,
    /// Presence update client id mismatch.
    #[error("presence client mismatch")]
    ClientMismatch,
    /// Server-only message received from client.
    #[error("invalid client message")]
    ClientOnlyMessage,
    /// Sync policy rejected a transaction.
    #[error(transparent)]
    Policy(#[from] SyncPolicyError),
    /// Persistence failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::{EntityId, UlidGenerator};
    use c3d_scene_ops::{SceneOperation, Transaction};
    use c3d_scene_schema::TransformOp;

    #[test]
    fn two_clients_sync_transform_via_hub() {
        let mut hub = SyncHub::new("demo");
        let (_, welcome_a) = hub
            .handle(
                None,
                SyncMessage::Hello {
                    workspace_id: "demo".into(),
                    user_name: "Alice".into(),
                },
            )
            .expect("alice hello");
        let client_a = match welcome_a[0].message.clone() {
            SyncMessage::Welcome { client_id, .. } => client_id,
            _ => panic!("welcome"),
        };

        let (_, welcome_b) = hub
            .handle(
                None,
                SyncMessage::Hello {
                    workspace_id: "demo".into(),
                    user_name: "Bob".into(),
                },
            )
            .expect("bob hello");
        let client_b = match welcome_b[0].message.clone() {
            SyncMessage::Welcome { client_id, .. } => client_id,
            _ => panic!("welcome"),
        };

        let entity_id = EntityId::new();
        let mut ids = UlidGenerator::new();
        let transaction = Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::TransformOp {
                entity_id,
                op: TransformOp::Translate(c3d_core::math::Vec3::new(1.0, 0.0, 0.0)),
            }],
        );

        let (_, broadcasts) = hub
            .handle(Some(client_a), SyncMessage::PushTransaction { transaction })
            .expect("push");
        assert!(matches!(
            broadcasts[0].message,
            SyncMessage::LogEntry { .. }
        ));

        let _ = client_b;
    }
}
