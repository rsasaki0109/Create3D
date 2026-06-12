use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use c3d_collab_core::ClientId;

use crate::hub::SyncHub;
use crate::protocol::{SyncEnvelope, SyncMessage};

/// TCP sync server that broadcasts collaboration events to connected clients.
#[derive(Debug, Clone)]
pub struct SyncServer {
    inner: Arc<SyncServerInner>,
}

#[derive(Debug)]
struct SyncServerInner {
    hub: Mutex<SyncHub>,
    sessions: Mutex<HashMap<ClientId, Sender<SyncEnvelope>>>,
}

impl SyncServer {
    /// Create a server backed by a configured hub.
    pub fn new(hub: SyncHub) -> Self {
        Self {
            inner: Arc::new(SyncServerInner {
                hub: Mutex::new(hub),
                sessions: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Load persisted hub state when paths are configured.
    pub fn load_persisted(&self) -> std::io::Result<()> {
        self.inner.hub.lock().expect("hub lock").load_persisted()
    }

    /// Handle one client connection on a background thread.
    pub fn handle_connection(&self, stream: TcpStream) {
        let server = self.clone();
        thread::spawn(move || server.serve_client(stream));
    }

    fn serve_client(&self, stream: TcpStream) {
        let (outbound_tx, outbound_rx) = mpsc::channel();
        let mut writer = stream.try_clone().expect("clone stream");
        thread::spawn(move || write_loop(&mut writer, outbound_rx));

        let reader = BufReader::new(stream);
        let mut client_id = None;

        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim().is_empty() {
                continue;
            }

            let envelope = match SyncEnvelope::from_json_line(&line) {
                Ok(envelope) => envelope,
                Err(err) => {
                    let _ = outbound_tx.send(SyncEnvelope::new(SyncMessage::Error {
                        message: err.to_string(),
                    }));
                    continue;
                }
            };

            let result = {
                let mut hub = self.inner.hub.lock().expect("hub lock");
                hub.handle(client_id, envelope.message)
            };

            match result {
                Ok((assigned, replies)) => {
                    if let Some(id) = assigned {
                        client_id = Some(id);
                        self.inner
                            .sessions
                            .lock()
                            .expect("sessions lock")
                            .insert(id, outbound_tx.clone());
                    }
                    for reply in replies {
                        self.deliver(client_id, reply);
                    }
                }
                Err(err) => {
                    let _ = outbound_tx.send(SyncEnvelope::new(SyncMessage::Error {
                        message: err.to_string(),
                    }));
                }
            }
        }

        if let Some(client_id) = client_id {
            self.inner
                .sessions
                .lock()
                .expect("sessions lock")
                .remove(&client_id);
            self.inner
                .hub
                .lock()
                .expect("hub lock")
                .disconnect(client_id);
        }
    }

    fn deliver(&self, sender: Option<ClientId>, envelope: SyncEnvelope) {
        if is_broadcast(&envelope.message) {
            let exclude_sender = matches!(envelope.message, SyncMessage::LogEntry { .. });
            let sessions = self.inner.sessions.lock().expect("sessions lock");
            for (client_id, tx) in sessions.iter() {
                if exclude_sender && sender == Some(*client_id) {
                    continue;
                }
                let _ = tx.send(envelope.clone());
            }
            return;
        }

        if let Some(client_id) = sender {
            if let Some(tx) = self
                .inner
                .sessions
                .lock()
                .expect("sessions lock")
                .get(&client_id)
            {
                let _ = tx.send(envelope);
            }
        }
    }
}

fn is_broadcast(message: &SyncMessage) -> bool {
    matches!(
        message,
        SyncMessage::LogEntry { .. }
            | SyncMessage::Presence { .. }
            | SyncMessage::CommentUpsert { .. }
            | SyncMessage::CommentStatus { .. }
            | SyncMessage::BranchProposalShare { .. }
            | SyncMessage::BranchProposalStatus { .. }
    )
}

fn write_loop(writer: &mut TcpStream, outbound: mpsc::Receiver<SyncEnvelope>) {
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
