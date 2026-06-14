//! Create3D ROS2 sidecar bridge.
//!
//! Exposes the robotics bridge protocol over newline-delimited JSON on TCP.
//! Mock mode mirrors the in-process mock bridge. ROS2 mode delegates to the
//! Python sidecar script when rclpy is available in the environment.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    /// Comma-separated joint names used by mock mode and ROS2 filtering.
    #[arg(long, default_value = "shoulder,elbow")]
    joint_names: String,
    /// ROS2 joint states topic for `--ros2` mode.
    #[arg(long, default_value = "/joint_states")]
    joint_states_topic: String,
    /// ROS2 TF topic for `--ros2` mode.
    #[arg(long, default_value = "/tf")]
    tf_topic: String,
    /// ROS2 static TF topic for `--ros2` mode.
    #[arg(long, default_value = "/tf_static")]
    tf_static_topic: String,
    /// Root TF frame forwarded in live snapshots.
    #[arg(long, default_value = "base_link")]
    tf_root_frame: String,
    /// Disable live TF forwarding in `--ros2` mode.
    #[arg(long, default_value_t = false)]
    no_tf: bool,
    /// Publish synthetic joint states without ROS2 installed.
    #[arg(long, default_value_t = true)]
    mock: bool,
    /// Subscribe to live ROS2 joint states via the Python sidecar script.
    #[arg(long, default_value_t = false)]
    ros2: bool,
    /// Milliseconds between mock publish ticks.
    #[arg(long, default_value_t = 50)]
    tick_ms: u64,
}

fn main() {
    init_logging(&LoggingConfig::default());
    let cli = Cli::parse();

    let joint_names = parse_joint_names(&cli.joint_names);
    if joint_names.is_empty() {
        eprintln!("At least one joint name is required via --joint-names");
        std::process::exit(1);
    }

    if cli.ros2 && !cli.mock {
        let status = run_python_ros2_sidecar(&cli, &joint_names);
        std::process::exit(status);
    }

    run_mock_tcp_server(&cli, joint_names);
}

fn parse_joint_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn run_python_ros2_sidecar(cli: &Cli, joint_names: &[String]) -> i32 {
    let Some(script) = ros2_bridge_script_path() else {
        eprintln!(
            "ROS2 sidecar script not found. Set CREATE3D_ROS2_BRIDGE_PY to tools/ros2_sidecar/bridge.py"
        );
        return 1;
    };

    let python = std::env::var("CREATE3D_ROS2_BRIDGE_PYTHON").unwrap_or_else(|_| "python3".into());
    let mut command = Command::new(python);
    command
        .arg(script)
        .arg("--listen")
        .arg(&cli.listen)
        .arg("--joint-states-topic")
        .arg(&cli.joint_states_topic)
        .arg("--tf-topic")
        .arg(&cli.tf_topic)
        .arg("--tf-static-topic")
        .arg(&cli.tf_static_topic)
        .arg("--tf-root-frame")
        .arg(&cli.tf_root_frame)
        .arg("--joint-names")
        .arg(joint_names.join(","))
        .arg("--tick-ms")
        .arg(cli.tick_ms.to_string());
    if cli.no_tf {
        command.arg("--no-tf");
    }
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match command.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("Failed to launch ROS2 sidecar script: {err}");
            1
        }
    }
}

fn ros2_bridge_script_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CREATE3D_ROS2_BRIDGE_PY") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../../tools/ros2_sidecar/bridge.py"),
        manifest_dir.join("../../../tools/ros2_sidecar/bridge.py"),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.canonicalize().unwrap_or(candidate))
}

fn run_mock_tcp_server(cli: &Cli, joint_names: Vec<String>) {
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
                    if let Err(err) = serve_mock_client(stream, robot_name, joint_names, tick_ms) {
                        tracing::debug!("sidecar client session ended: {err}");
                    }
                });
            }
            Err(err) => tracing::warn!("sidecar accept failed: {err}"),
        }
    }
}

fn serve_mock_client(
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
