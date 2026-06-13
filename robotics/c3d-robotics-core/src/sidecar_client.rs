use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::protocol::{BridgeEnvelope, BridgeMessage};

/// Default localhost address for the ROS2 sidecar.
pub const DEFAULT_SIDECAR_ADDR: &str = "127.0.0.1:9741";

/// Configuration for a sidecar bridge connection.
#[derive(Debug, Clone)]
pub struct SidecarClientConfig {
    /// Sidecar listen address, e.g. `127.0.0.1:9741`.
    pub server_addr: String,
    /// Client identifier sent in the hello handshake.
    pub client_id: String,
}

/// Background TCP client for the ROS2 sidecar bridge.
pub struct SidecarClient {
    outbound: Sender<BridgeEnvelope>,
    inbound: Receiver<BridgeEnvelope>,
    connected: bool,
}

impl SidecarClient {
    /// Connect to a sidecar bridge and spawn background IO threads.
    pub fn connect(config: SidecarClientConfig) -> Result<Self, SidecarClientError> {
        let addr = config
            .server_addr
            .to_socket_addrs()
            .map_err(|err| SidecarClientError::Connect(err.to_string()))?
            .next()
            .ok_or_else(|| SidecarClientError::Connect("no socket addresses resolved".into()))?;
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .map_err(|err| SidecarClientError::Connect(err.to_string()))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|err| SidecarClientError::Connect(err.to_string()))?;

        let (outbound_tx, outbound_rx) = mpsc::channel::<BridgeEnvelope>();
        let (inbound_tx, inbound_rx) = mpsc::channel::<BridgeEnvelope>();

        let reader_stream = stream
            .try_clone()
            .map_err(|err| SidecarClientError::Connect(err.to_string()))?;
        thread::spawn(move || read_loop(reader_stream, inbound_tx));

        let writer_stream = stream;
        thread::spawn(move || write_loop(writer_stream, outbound_rx));

        outbound_tx
            .send(BridgeEnvelope::new(BridgeMessage::Hello {
                client_id: config.client_id,
            }))
            .map_err(|_| SidecarClientError::Disconnected)?;

        Ok(Self {
            outbound: outbound_tx,
            inbound: inbound_rx,
            connected: true,
        })
    }

    /// Returns true when the background connection is active.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Drain envelopes received from the sidecar since the last poll.
    pub fn poll_envelopes(&mut self) -> Vec<BridgeEnvelope> {
        let mut envelopes = Vec::new();
        while let Ok(envelope) = self.inbound.try_recv() {
            envelopes.push(envelope);
        }
        envelopes
    }

    /// Close the sidecar connection.
    pub fn disconnect(&mut self) {
        self.connected = false;
        drop(std::mem::replace(
            &mut self.outbound,
            mpsc::channel::<BridgeEnvelope>().0,
        ));
    }
}

fn read_loop(stream: TcpStream, inbound: Sender<BridgeEnvelope>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(line) if line.trim().is_empty() => {}
            Ok(line) => match BridgeEnvelope::from_json_line(&line) {
                Ok(envelope) => {
                    if inbound.send(envelope).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            },
            Err(_) => {
                break;
            }
        }
    }
}

fn write_loop(stream: TcpStream, outbound: Receiver<BridgeEnvelope>) {
    let mut stream = stream;
    for envelope in outbound {
        let Ok(line) = envelope.to_json_line() else {
            continue;
        };
        if writeln!(stream, "{line}").is_err() {
            break;
        }
        if stream.flush().is_err() {
            break;
        }
    }
}

/// Sidecar client connection failures.
#[derive(Debug, thiserror::Error)]
pub enum SidecarClientError {
    /// Failed to connect to the sidecar.
    #[error("sidecar connect failed: {0}")]
    Connect(String),
    /// Sidecar disconnected before the handshake completed.
    #[error("sidecar disconnected")]
    Disconnected,
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::mock_bridge::MockBridge;

    #[test]
    fn sidecar_client_receives_mock_envelopes() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            serve_mock_sidecar(stream);
        });

        let mut client = SidecarClient::connect(SidecarClientConfig {
            server_addr: addr.to_string(),
            client_id: "test-client".into(),
        })
        .expect("connect");

        let mut envelopes = Vec::new();
        for _ in 0..40 {
            envelopes.extend(client.poll_envelopes());
            if envelopes
                .iter()
                .any(|envelope| matches!(envelope.message, BridgeMessage::JointState(_)))
            {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }

        client.disconnect();
        server.join().expect("server thread");

        assert!(envelopes
            .iter()
            .any(|envelope| matches!(envelope.message, BridgeMessage::JointState(_))));
    }

    fn serve_mock_sidecar(stream: TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut writer = stream;
        let mut line = String::new();
        let _ = reader.read_line(&mut line);

        let mut bridge = MockBridge::new("preview_arm", vec!["shoulder".into()]);
        for _ in 0..3 {
            for envelope in bridge.next_envelopes() {
                let Ok(json) = envelope.to_json_line() else {
                    continue;
                };
                if writeln!(writer, "{json}").is_err() {
                    return;
                }
            }
            if writer.flush().is_err() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}
