//! Create3D collaboration sync server prototype.

use std::net::TcpListener;

use c3d_sync::{SyncHub, SyncServer};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "create3d-sync-server",
    about = "Create3D collaboration sync server"
)]
struct Args {
    /// Bind address.
    #[arg(long, default_value = "127.0.0.1:9731")]
    bind: String,
    /// Workspace identifier served by this process.
    #[arg(long, default_value = "default-workspace")]
    workspace: String,
    /// Optional directory for operation log and collab store persistence.
    #[arg(long)]
    data_dir: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();
    let mut hub = SyncHub::new(&args.workspace);
    if let Some(dir) = &args.data_dir {
        hub = hub
            .with_log_path(dir.join("operation_log.jsonl"))
            .with_store_dir(dir.join("store"));
    }
    let server = SyncServer::new(hub);
    if args.data_dir.is_some() {
        server
            .load_persisted()
            .expect("load persisted collaboration state");
    }

    let listener = TcpListener::bind(&args.bind).expect("bind sync server");
    println!(
        "Create3D sync server listening on {} (workspace `{}`)",
        args.bind, args.workspace
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => server.handle_connection(stream),
            Err(err) => eprintln!("accept error: {err}"),
        }
    }
}
