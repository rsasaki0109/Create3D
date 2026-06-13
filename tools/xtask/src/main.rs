//! Developer task runner for the Create3D workspace.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, io};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Create3D developer tasks")]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    /// Run formatting, clippy, and tests.
    Check,
    /// Run workspace tests.
    Test,
    /// Run clippy with warnings denied.
    Clippy,
    /// Format the workspace.
    Fmt,
    /// Generate sample projects under `samples/`.
    Samples,
    /// Run the scene replay benchmark via create3d-cli.
    Bench {
        /// Number of entities in the benchmark scene.
        #[arg(long, default_value_t = 1_024)]
        entities: usize,
        /// Number of translate replay iterations.
        #[arg(long, default_value_t = 256)]
        iterations: usize,
    },
    /// Build release binaries for Alpha packaging.
    Package,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let workspace_root = workspace_root();

    let result = match cli.command {
        CommandKind::Check => run_check(&workspace_root),
        CommandKind::Test => run_cargo(&workspace_root, &["test", "--workspace"]),
        CommandKind::Clippy => run_cargo(
            &workspace_root,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        CommandKind::Fmt => run_cargo(&workspace_root, &["fmt", "--all"]),
        CommandKind::Samples => generate_samples(&workspace_root),
        CommandKind::Bench {
            entities,
            iterations,
        } => run_bench(&workspace_root, entities, iterations),
        CommandKind::Package => run_package(&workspace_root),
    };

    match result {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("xtask error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_check(workspace_root: &Path) -> io::Result<std::process::ExitStatus> {
    let fmt = run_cargo(workspace_root, &["fmt", "--all", "--check"])?;
    if !fmt.success() {
        return Ok(fmt);
    }

    let clippy = run_cargo(
        workspace_root,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    if !clippy.success() {
        return Ok(clippy);
    }

    run_cargo(workspace_root, &["test", "--workspace"])
}

fn generate_samples(workspace_root: &Path) -> io::Result<std::process::ExitStatus> {
    let samples_root = workspace_root.join("samples");
    fs::create_dir_all(&samples_root)?;

    let templates = [
        ("mesh-scene", "Mesh Scene"),
        ("point-cloud-scene", "Point Cloud Scene"),
        ("gaussian-splat-scene", "Gaussian Splat Scene"),
        ("urdf-robot-scene", "URDF Robot Scene"),
        ("ai-editing-demo", "AI Editing Demo"),
    ];

    for (template, name) in templates {
        let output = samples_root.join(template);
        if output.exists() {
            fs::remove_dir_all(&output)?;
        }
        let status = run_cargo(
            workspace_root,
            &[
                "run",
                "-p",
                "create3d-cli",
                "--",
                "create",
                "--output",
                output.to_str().expect("utf8 path"),
                "--name",
                name,
                "--template",
                template,
            ],
        )?;
        if !status.success() {
            return Ok(status);
        }
    }

    println!("Generated sample projects under {}", samples_root.display());
    Command::new("cargo")
        .arg("--version")
        .current_dir(workspace_root)
        .status()
}

fn run_bench(
    workspace_root: &Path,
    entities: usize,
    iterations: usize,
) -> io::Result<std::process::ExitStatus> {
    run_cargo(
        workspace_root,
        &[
            "run",
            "-p",
            "create3d-cli",
            "--",
            "bench",
            "--entities",
            &entities.to_string(),
            "--iterations",
            &iterations.to_string(),
        ],
    )
}

fn run_package(workspace_root: &Path) -> io::Result<std::process::ExitStatus> {
    let apps = [
        "create3d-desktop",
        "create3d-cli",
        "create3d-sync-server",
        "create3d-ros2-bridge",
    ];
    for app in apps {
        let status = run_cargo(workspace_root, &["build", "-p", app, "--release"])?;
        if !status.success() {
            return Ok(status);
        }
    }
    println!(
        "Release binaries available under {}/target/release/",
        workspace_root.display()
    );
    Command::new("cargo")
        .arg("--version")
        .current_dir(workspace_root)
        .status()
}

fn run_cargo(workspace_root: &Path, args: &[&str]) -> io::Result<std::process::ExitStatus> {
    Command::new("cargo")
        .args(args)
        .current_dir(workspace_root)
        .status()
}

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|tools| tools.parent())
        .expect("xtask should live at tools/xtask")
        .to_path_buf()
}
