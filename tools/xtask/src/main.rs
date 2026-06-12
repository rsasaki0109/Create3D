//! Developer task runner for the Create3D workspace.

use std::process::{Command, ExitCode};
use std::{env, path::PathBuf};

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

fn run_check(workspace_root: &PathBuf) -> std::io::Result<std::process::ExitStatus> {
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

fn run_cargo(workspace_root: &PathBuf, args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
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
