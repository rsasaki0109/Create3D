//! Create3D ROS2 sidecar bridge.
//!
//! Exposes the robotics bridge protocol over newline-delimited JSON on TCP.
//! Beta ships a mock mode that mirrors the in-process mock bridge. A future
//! `--ros2` mode can subscribe to live ROS2 topics in an environment with
//! rclpy/rclcpp installed.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use c3d_core::logging::{init_logging, LoggingConfig};
use c3d_robotics_core::{BridgeEnvelope, MockBridge, DEFAULT_SIDECAR_ADDR};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "create3d-ros2-bridge", about = "Create3D ROS2 sidecar bridge")]
struct Cli {
    /// TCP listen address.
    #[arg(long, default_value = DEFAULT_SIDECAR_ADDR)]
    listen: String,
    /// Robot name used by mock mode.
    #[arg(long, default_value = "preview_arm")]
    robot_name: String,
    /// Comma-separated joint names used by mock mode.
    #[arg(long, default_value = "shoulder,elbow")]
    joint_names: String,
    /// Publish synthetic joint states without ROS2 installed.
    #[arg(long, default_value_t = true)]
    mock: bool,
    /// Reserved for future live ROS2 subscription mode.
    #[arg(long, default_value_t = false)]
    ros2: bool,
    /// Milliseconds between mock publish ticks.
    #[arg(long, default_value_t = 50)]
    tick_ms: u64,
}

fn main() {
    init_logging(&LoggingConfig::default());
    let cli = Cli::parse();

    if cli.ros2 && !cli.mock {
        eprintln!(
            "ROS2 mode is not bundled in Beta yet. Re-run with --mock or omit --ros2 to use synthetic joint states."
        );
        std::process::exit(1);
    }

    let joint_names: Vec<String> = cli
        .joint_names
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    if joint_names.is_empty() {
        eprintln!("At least one joint name is required via --joint-names");
        std::process::exit(1);
    }

    let listener = TcpListener::bind(&cli.listen).unwrap_or_else(|err| {
        eprintln!("Failed to bind {}: {err}", cli.listen);
        std::process::exit(1);
    });
    tracing::info!(
        "Create3D ROS2 sidecar listening on {} (mock={}, joints={:?})",
        cli.listen,
        cli.mock,
        joint_names
    );

    let running = Arc::new(AtomicBool::new(true));
    let tick_ms = cli.tick_ms;
    let robot_name = cli.robot_name.clone();

    for stream in listener.incoming() {
        if !running.load(Ordering::Relaxed) {
            break;
        }
        match stream {
            Ok(stream) => {
                let robot_name = robot_name.clone();
                let joint_names = joint_names.clone();
                thread::spawn(move || {
                    if let Err(err) = serve_client(stream, robot_name, joint_names, tick_ms) {
                        tracing::debug!("sidecar client session ended: {err}");
                    }
                });
            }
            Err(err) => tracing::warn!("sidecar accept failed: {err}"),
        }
    }
}

fn serve_client(
    stream: TcpStream,
    robot_name: String,
    joint_names: Vec<String>,
    tick_ms: u64,
) -> Result<(), String> {
    let peer = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".into());
    tracing::info!("sidecar client connected from {peer}");

    let mut reader = BufReader::new(stream.try_clone().map_err(|err| err.to_string())?);
    let mut writer = stream;

    let mut line = String::new();
    reader.read_line(&mut line).map_err(|err| err.to_string())?;
    if !line.trim().is_empty() {
        match BridgeEnvelope::from_json_line(line.trim()) {
            Ok(envelope) => tracing::debug!("sidecar hello: {:?}", envelope.message),
            Err(err) => tracing::warn!("sidecar hello parse failed: {err}"),
        }
    }

    let mut bridge = MockBridge::new(robot_name, joint_names);
    loop {
        for envelope in bridge.next_envelopes() {
            let json = envelope.to_json_line().map_err(|err| err.to_string())?;
            writeln!(writer, "{json}").map_err(|err| err.to_string())?;
        }
        writer.flush().map_err(|err| err.to_string())?;
        thread::sleep(Duration::from_millis(tick_ms));
    }
}
