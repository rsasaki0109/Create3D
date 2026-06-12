use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use c3d_collab_core::{
    BranchProposal, ClientId, CommentId, CommentStatus, OperationLogEntry, ProposalId,
    ProposalStatus, SceneComment, UserPresence,
};
use c3d_core::EntityId;
use c3d_scene_ops::Transaction;

use crate::policy::SyncPolicyError;
use crate::protocol::{SyncEnvelope, SyncMessage};

/// Configuration for a sync client connection.
#[derive(Debug, Clone)]
pub struct SyncClientConfig {
    /// Workspace identifier shared by collaborators.
    pub workspace_id: String,
    /// Display name shown in presence UI.
    pub user_name: String,
    /// Server address, e.g. `127.0.0.1:9731`.
    pub server_addr: String,
}

/// Events delivered to the editor from the sync client.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    /// Connected and welcomed by the server.
    Connected {
        /// Assigned client id.
        client_id: ClientId,
        /// Current log head sequence.
        head_sequence: u64,
    },
    /// Remote operation log entry to apply.
    LogEntry(OperationLogEntry),
    /// Peer presence update.
    Presence(UserPresence),
    /// Comment added or updated.
    Comment(SceneComment),
    /// Comment status changed.
    CommentStatus {
        /// Comment id.
        comment_id: CommentId,
        /// New status.
        status: CommentStatus,
    },
    /// Branch proposal shared.
    Proposal(BranchProposal),
    /// Branch proposal status changed.
    ProposalStatus {
        /// Proposal id.
        proposal_id: ProposalId,
        /// New status.
        status: ProposalStatus,
    },
    /// Sync error reported by server or transport.
    Error(String),
    /// Connection closed.
    Disconnected,
}

/// Background TCP sync client used by the desktop editor.
pub struct SyncClient {
    outbound: Sender<SyncEnvelope>,
    inbound: Receiver<SyncEvent>,
    client_id: Option<ClientId>,
    connected: bool,
}

impl SyncClient {
    /// Connect to a sync server and spawn background IO threads.
    pub fn connect(config: SyncClientConfig) -> Result<Self, SyncClientError> {
        let addr = config
            .server_addr
            .to_socket_addrs()
            .map_err(|err| SyncClientError::Connect(err.to_string()))?
            .next()
            .ok_or_else(|| SyncClientError::Connect("no socket addresses resolved".into()))?;
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .map_err(|err| SyncClientError::Connect(err.to_string()))?;
        stream
            .set_nonblocking(false)
            .map_err(|err| SyncClientError::Connect(err.to_string()))?;

        let (outbound_tx, outbound_rx) = mpsc::channel::<SyncEnvelope>();
        let (inbound_tx, inbound_rx) = mpsc::channel::<SyncEvent>();

        let reader_stream = stream
            .try_clone()
            .map_err(|err| SyncClientError::Connect(err.to_string()))?;
        thread::spawn(move || read_loop(reader_stream, inbound_tx));

        let writer_stream = stream;
        thread::spawn(move || write_loop(writer_stream, outbound_rx));

        outbound_tx
            .send(SyncEnvelope::new(SyncMessage::Hello {
                workspace_id: config.workspace_id,
                user_name: config.user_name,
            }))
            .map_err(|_| SyncClientError::Disconnected)?;

        Ok(Self {
            outbound: outbound_tx,
            inbound: inbound_rx,
            client_id: None,
            connected: true,
        })
    }

    /// Returns true when the background connection is active.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Returns the assigned client id after welcome.
    pub fn client_id(&self) -> Option<ClientId> {
        self.client_id
    }

    /// Poll inbound sync events without blocking.
    pub fn poll_events(&mut self) -> Vec<SyncEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.inbound.try_recv() {
            if let SyncEvent::Connected { client_id, .. } = event {
                self.client_id = Some(client_id);
            }
            if matches!(event, SyncEvent::Disconnected) {
                self.connected = false;
            }
            events.push(event);
        }
        events
    }

    /// Push a local transaction to the shared log.
    pub fn push_transaction(&self, transaction: Transaction) -> Result<(), SyncClientError> {
        self.send(SyncEnvelope::new(SyncMessage::PushTransaction {
            transaction,
        }))
    }

    /// Broadcast updated presence information.
    pub fn update_presence(
        &self,
        user_name: &str,
        selected_entity: Option<EntityId>,
        cursor_viewport: Option<[f32; 2]>,
    ) -> Result<(), SyncClientError> {
        let client_id = self.client_id.ok_or(SyncClientError::NotJoined)?;
        self.send(SyncEnvelope::new(SyncMessage::Presence {
            presence: UserPresence {
                client_id,
                user_name: user_name.to_string(),
                selected_entity,
                cursor_viewport,
            },
        }))
    }

    /// Share or update a comment.
    pub fn upsert_comment(&self, comment: SceneComment) -> Result<(), SyncClientError> {
        self.send(SyncEnvelope::new(SyncMessage::CommentUpsert { comment }))
    }

    /// Resolve or reopen a comment.
    pub fn set_comment_status(
        &self,
        comment_id: CommentId,
        status: CommentStatus,
    ) -> Result<(), SyncClientError> {
        self.send(SyncEnvelope::new(SyncMessage::CommentStatus {
            comment_id,
            status,
        }))
    }

    /// Share a branch proposal bundle.
    pub fn share_proposal(&self, proposal: BranchProposal) -> Result<(), SyncClientError> {
        self.send(SyncEnvelope::new(SyncMessage::BranchProposalShare {
            proposal,
        }))
    }

    /// Update branch proposal status.
    pub fn set_proposal_status(
        &self,
        proposal_id: ProposalId,
        status: ProposalStatus,
    ) -> Result<(), SyncClientError> {
        self.send(SyncEnvelope::new(SyncMessage::BranchProposalStatus {
            proposal_id,
            status,
        }))
    }

    fn send(&self, envelope: SyncEnvelope) -> Result<(), SyncClientError> {
        self.outbound
            .send(envelope)
            .map_err(|_| SyncClientError::Disconnected)
    }
}

fn read_loop(stream: TcpStream, inbound: Sender<SyncEvent>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                match SyncEnvelope::from_json_line(&line) {
                    Ok(envelope) => dispatch_message(envelope.message, &inbound),
                    Err(err) => {
                        let _ = inbound.send(SyncEvent::Error(err.to_string()));
                    }
                }
            }
            Err(_) => {
                let _ = inbound.send(SyncEvent::Disconnected);
                break;
            }
        }
    }
}

fn write_loop(stream: TcpStream, outbound: Receiver<SyncEnvelope>) {
    let mut writer = stream;
    for envelope in outbound.iter() {
        match envelope.to_json_line() {
            Ok(line) => {
                if writeln!(writer, "{line}").is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn dispatch_message(message: SyncMessage, inbound: &Sender<SyncEvent>) {
    let event = match message {
        SyncMessage::Welcome {
            client_id,
            head_sequence,
            ..
        } => SyncEvent::Connected {
            client_id,
            head_sequence,
        },
        SyncMessage::LogEntry { entry } => SyncEvent::LogEntry(entry),
        SyncMessage::Presence { presence } => SyncEvent::Presence(presence),
        SyncMessage::CommentUpsert { comment } => SyncEvent::Comment(comment),
        SyncMessage::CommentStatus { comment_id, status } => {
            SyncEvent::CommentStatus { comment_id, status }
        }
        SyncMessage::BranchProposalShare { proposal } => SyncEvent::Proposal(proposal),
        SyncMessage::BranchProposalStatus {
            proposal_id,
            status,
        } => SyncEvent::ProposalStatus {
            proposal_id,
            status,
        },
        SyncMessage::Error { message } => SyncEvent::Error(message),
        SyncMessage::Hello { .. } | SyncMessage::PushTransaction { .. } => return,
    };
    let _ = inbound.send(event);
}

/// Sync client failures.
#[derive(Debug, thiserror::Error)]
pub enum SyncClientError {
    /// Could not connect to the sync server.
    #[error("connect failed: {0}")]
    Connect(String),
    /// Client has not received welcome yet.
    #[error("client has not joined")]
    NotJoined,
    /// Background connection is closed.
    #[error("sync client disconnected")]
    Disconnected,
    /// Policy rejected a transaction before send.
    #[error(transparent)]
    Policy(#[from] SyncPolicyError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SyncHub;
    use std::io::BufReader;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn client_receives_log_entry_from_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let hub = Arc::new(Mutex::new(SyncHub::new("demo")));

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut client_id = None;
            let reader = BufReader::new(stream.try_clone().expect("clone"));
            for line in reader.lines().map_while(Result::ok) {
                let envelope = SyncEnvelope::from_json_line(&line).expect("parse");
                let mut hub = hub.lock().expect("hub");
                let (assigned, replies) = hub.handle(client_id, envelope.message).expect("handle");
                if let Some(id) = assigned {
                    client_id = Some(id);
                }
                for reply in replies {
                    let _ = writeln!(stream, "{}", reply.to_json_line().expect("json"));
                }
            }
        });

        let mut client = SyncClient::connect(SyncClientConfig {
            workspace_id: "demo".into(),
            user_name: "Alice".into(),
            server_addr: addr.to_string(),
        })
        .expect("connect");

        let mut connected = false;
        for _ in 0..20 {
            for event in client.poll_events() {
                if matches!(event, SyncEvent::Connected { .. }) {
                    connected = true;
                }
            }
            if connected {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(connected);
    }
}
