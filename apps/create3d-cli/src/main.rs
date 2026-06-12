//! Create3D command-line tools.

use std::path::PathBuf;

use c3d_core::{init_logging, LoggingConfig, UlidGenerator};
use c3d_project::Project;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "create3d-cli", about = "Create3D command-line tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Import a glTF/GLB file into a new or existing project.
    Import {
        /// glTF/GLB source file.
        #[arg(long)]
        input: PathBuf,
        /// Project output directory.
        #[arg(long)]
        output: PathBuf,
        /// Project name when creating a new project.
        #[arg(long, default_value = "imported-project")]
        name: String,
    },
    /// Import a PLY point cloud into a new or existing project.
    ImportPly {
        /// PLY source file.
        #[arg(long)]
        input: PathBuf,
        /// Project output directory.
        #[arg(long)]
        output: PathBuf,
        /// Project name when creating a new project.
        #[arg(long, default_value = "imported-pointcloud")]
        name: String,
    },
    /// Import a 3D Gaussian splat PLY file into a new or existing project.
    ImportGsplat {
        /// 3DGS PLY source file.
        #[arg(long)]
        input: PathBuf,
        /// Project output directory.
        #[arg(long)]
        output: PathBuf,
        /// Project name when creating a new project.
        #[arg(long, default_value = "imported-gsplat")]
        name: String,
    },
}

fn main() {
    init_logging(&LoggingConfig::default());
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Import {
            input,
            output,
            name,
        } => import_gltf(input, output, name),
        Command::ImportPly {
            input,
            output,
            name,
        } => import_ply(input, output, name),
        Command::ImportGsplat {
            input,
            output,
            name,
        } => import_gsplat(input, output, name),
    }
}

fn import_gltf(
    input: PathBuf,
    output: PathBuf,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ids = UlidGenerator::new();
    let mut project = open_or_create_project(&output, name)?;

    let report = project.import_gltf(&input, &mut ids)?;
    project.save()?;

    println!(
        "Imported {} entities, {} meshes, {} materials, {} textures into {}",
        report.entity_count,
        report.mesh_assets.len(),
        report.material_assets.len(),
        report.texture_assets.len(),
        output.display()
    );
    Ok(())
}

fn import_ply(
    input: PathBuf,
    output: PathBuf,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ids = UlidGenerator::new();
    let mut project = open_or_create_project(&output, name)?;

    let report = project.import_ply(&input, &mut ids)?;
    project.save()?;

    println!(
        "Imported point cloud with {} points in {} chunks into {} (entity {})",
        report.point_count,
        report.chunk_assets.len(),
        output.display(),
        report.entity_id
    );
    Ok(())
}

fn import_gsplat(
    input: PathBuf,
    output: PathBuf,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ids = UlidGenerator::new();
    let mut project = open_or_create_project(&output, name)?;

    let report = project.import_gsplat_ply(&input, &mut ids)?;
    project.save()?;

    println!(
        "Imported gaussian splats with {} splats in {} chunks into {} (entity {})",
        report.splat_count,
        report.chunk_assets.len(),
        output.display(),
        report.entity_id
    );
    Ok(())
}

fn open_or_create_project(
    output: &PathBuf,
    name: String,
) -> Result<Project, c3d_project::ProjectError> {
    if output.join("manifest.c3d.toml").is_file() {
        Project::open(output)
    } else {
        Project::create(output, name)
    }
}
