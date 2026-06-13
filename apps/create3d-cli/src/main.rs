//! Create3D command-line tools.

use std::path::PathBuf;
use std::time::Instant;

use c3d_core::{init_logging, LoggingConfig, UlidGenerator};
use c3d_project::{Project, ProjectTemplate};
use c3d_scene_ops::{SceneOperation, Transaction, TransactionManager};
use c3d_scene_schema::TransformOp;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "create3d-cli", about = "Create3D command-line tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a project from a built-in template.
    Create {
        /// Project output directory.
        #[arg(long)]
        output: PathBuf,
        /// Project name.
        #[arg(long, default_value = "create3d-project")]
        name: String,
        /// Template id (`list-templates` for options).
        #[arg(long, default_value = "mesh-scene")]
        template: String,
    },
    /// List built-in project templates.
    ListTemplates,
    /// Benchmark large-scene transaction replay throughput.
    Bench {
        /// Number of entities to create for the benchmark scene.
        #[arg(long, default_value_t = 1_024)]
        entities: usize,
        /// Number of translate replay iterations.
        #[arg(long, default_value_t = 256)]
        iterations: usize,
    },
    /// Export mesh entities from a project to a GLB snapshot.
    ExportGltf {
        /// Existing project directory.
        #[arg(long)]
        project: PathBuf,
        /// Output GLB file path.
        #[arg(long)]
        output: PathBuf,
    },
    /// Export mesh entities from a project to a USDA snapshot.
    ExportUsd {
        /// Existing project directory.
        #[arg(long)]
        project: PathBuf,
        /// Output USDA file path.
        #[arg(long)]
        output: PathBuf,
    },
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
    /// Import a URDF robot into a new or existing project.
    ImportUrdf {
        /// URDF source file.
        #[arg(long)]
        input: PathBuf,
        /// Project output directory.
        #[arg(long)]
        output: PathBuf,
        /// Project name when creating a new project.
        #[arg(long, default_value = "imported-robot")]
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
        Command::Create {
            output,
            name,
            template,
        } => create_project(output, name, template),
        Command::ListTemplates => list_templates(),
        Command::Bench {
            entities,
            iterations,
        } => bench_scene_replay(entities, iterations),
        Command::ExportGltf { project, output } => export_gltf(project, output),
        Command::ExportUsd { project, output } => export_usd(project, output),
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
        Command::ImportUrdf {
            input,
            output,
            name,
        } => import_urdf(input, output, name),
    }
}

fn create_project(
    output: PathBuf,
    name: String,
    template: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let template = ProjectTemplate::parse(&template).ok_or_else(|| {
        format!("unknown template `{template}`; run `list-templates` for valid ids")
    })?;
    if output.exists() && !output.join("manifest.c3d.toml").is_file() {
        return Err(format!(
            "output path `{}` exists and is not a Create3D project",
            output.display()
        )
        .into());
    }
    if output.join("manifest.c3d.toml").is_file() {
        return Err(format!(
            "project already exists at `{}`; choose an empty directory",
            output.display()
        )
        .into());
    }
    let project = Project::create_from_template(&output, name, template)?;
    println!(
        "Created `{}` template project at {} ({} entities)",
        template.id(),
        output.display(),
        project.scene().entity_count()
    );
    Ok(())
}

fn list_templates() -> Result<(), Box<dyn std::error::Error>> {
    for template in ProjectTemplate::all() {
        println!("{}\t{}", template.id(), template.description());
    }
    Ok(())
}

fn bench_scene_replay(
    entities: usize,
    iterations: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let mut project = Project::create_from_template(temp.path(), "bench", ProjectTemplate::Empty)?;
    let mut ids = UlidGenerator::new();
    let mut entity_ids = Vec::with_capacity(entities);
    for index in 0..entities {
        let entity_id = ids.next_entity_id();
        entity_ids.push(entity_id);
        c3d_scene_ops::apply_operations(
            project.scene_mut(),
            &[SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(format!("Entity{index}").into()),
                transform: Default::default(),
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
                robot_root: None,
                robot_link: None,
                robot_joint: None,
            }],
        )?;
    }

    let mut manager = TransactionManager::new(project.scene().clone());
    let start = Instant::now();
    for step in 0..iterations {
        let entity_id = entity_ids[step % entity_ids.len()];
        manager.apply(Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::TransformOp {
                entity_id,
                op: TransformOp::Translate(c3d_core::math::Vec3::new(0.01, 0.0, 0.0)),
            }],
        ))?;
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    println!(
        "bench scene replay: {entities} entities, {iterations} translate ops in {:.2?} ({ops_per_sec:.0} ops/s)",
        elapsed
    );
    Ok(())
}

fn export_gltf(project: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let project = Project::open(&project)?;
    let report = project.export_gltf(&output)?;
    println!(
        "Exported {} meshes ({} nodes) to {} ({} bytes)",
        report.mesh_count,
        report.node_count,
        output.display(),
        report.byte_length
    );
    Ok(())
}

fn export_usd(project: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let project = Project::open(&project)?;
    let report = project.export_usd(&output)?;
    println!(
        "Exported {} meshes ({} xforms) to {} ({} bytes)",
        report.mesh_count,
        report.prim_count,
        output.display(),
        report.byte_length
    );
    Ok(())
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

fn import_urdf(
    input: PathBuf,
    output: PathBuf,
    name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ids = UlidGenerator::new();
    let mut project = open_or_create_project(&output, name)?;

    let report = project.import_urdf(&input, &mut ids)?;
    project.save()?;

    println!(
        "Imported URDF robot `{}` with {} links into {} (root entity {})",
        report.robot_name,
        report.link_entities.len(),
        output.display(),
        report.root_entity_id
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
