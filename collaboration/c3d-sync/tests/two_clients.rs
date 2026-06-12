//! Integration test for two sync clients sharing transform operations.

use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use c3d_core::{EntityId, UlidGenerator};
use c3d_scene_ops::{SceneOperation, Transaction};
use c3d_scene_schema::TransformOp;
use c3d_sync::{SyncClient, SyncClientConfig, SyncEvent, SyncHub, SyncServer};

#[test]
fn two_clients_receive_shared_transform() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = SyncServer::new(SyncHub::new("demo"));

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            server.handle_connection(stream);
        }
    });

    let mut alice = SyncClient::connect(SyncClientConfig {
        workspace_id: "demo".into(),
        user_name: "Alice".into(),
        server_addr: addr.to_string(),
    })
    .expect("alice connect");
    wait_for_connected(&mut alice);

    let mut bob = SyncClient::connect(SyncClientConfig {
        workspace_id: "demo".into(),
        user_name: "Bob".into(),
        server_addr: addr.to_string(),
    })
    .expect("bob connect");
    wait_for_connected(&mut bob);

    let entity_id = EntityId::new();
    let mut ids = UlidGenerator::new();
    alice
        .push_transaction(Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::TransformOp {
                entity_id,
                op: TransformOp::Translate(c3d_core::math::Vec3::new(2.0, 0.0, 0.0)),
            }],
        ))
        .expect("push");

    let mut received = false;
    for _ in 0..50 {
        for event in bob.poll_events() {
            if let SyncEvent::LogEntry(entry) = event {
                assert_eq!(entry.sequence, 1);
                received = true;
            }
        }
        if received {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(received);
}

fn wait_for_connected(client: &mut SyncClient) {
    for _ in 0..50 {
        for event in client.poll_events() {
            if matches!(event, SyncEvent::Connected { .. }) {
                return;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("client did not connect");
}
