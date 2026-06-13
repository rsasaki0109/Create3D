use std::collections::HashSet;
use std::fs;
use std::path::Path;

use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use c3d_core::math::{Quat, Vec3};
use c3d_core::EntityId;
use c3d_scene_doc::{Entity, SceneDoc};
use c3d_scene_schema::Transform;

/// Export failures.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Scene had no exportable mesh content.
    #[error("scene has no mesh entities to export")]
    EmptyScene,
    /// Asset lookup failure.
    #[error("asset error: {0}")]
    Asset(String),
}

/// Summary of an export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsdExportReport {
    /// Number of Xform prims written.
    pub prim_count: usize,
    /// Number of Mesh prims written.
    pub mesh_count: usize,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export a SceneDB document and mesh/material assets to an ASCII USD file.
pub fn export_scene_usda(
    scene: &SceneDoc,
    mesh_loader: impl Fn(c3d_core::AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: impl Fn(c3d_core::AssetId) -> Result<MaterialAssetData, ExportError>,
    output: impl AsRef<Path>,
) -> Result<UsdExportReport, ExportError> {
    let mut names = NameRegistry::default();
    let mut stats = ExportStats::default();
    let mut roots = String::new();

    for entity in scene.entities().filter(|entity| entity.parent.is_none()) {
        if let Some(block) = export_entity_tree(
            entity.id,
            scene,
            1,
            &mesh_loader,
            &material_loader,
            &mut names,
            &mut stats,
        )? {
            roots.push_str(&block);
        }
    }

    if stats.mesh_count == 0 {
        return Err(ExportError::EmptyScene);
    }

    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut document = String::from("#usda 1.0\n");
    document.push_str("(\n");
    document.push_str("    defaultPrim = \"Scene\"\n");
    document.push_str("    doc = \"Create3D mesh snapshot export\"\n");
    document.push_str("    metersPerUnit = 1\n");
    document.push_str("    upAxis = \"Y\"\n");
    document.push_str(")\n\n");
    document.push_str("def Xform \"Scene\"\n");
    document.push_str("{\n");
    document.push_str(&roots);
    document.push_str("}\n");

    fs::write(output, &document)?;
    let byte_length = fs::metadata(output)?.len();
    Ok(UsdExportReport {
        prim_count: stats.prim_count,
        mesh_count: stats.mesh_count,
        byte_length,
    })
}

fn export_entity_tree(
    entity_id: EntityId,
    scene: &SceneDoc,
    indent: usize,
    mesh_loader: &impl Fn(c3d_core::AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: &impl Fn(c3d_core::AssetId) -> Result<MaterialAssetData, ExportError>,
    names: &mut NameRegistry,
    stats: &mut ExportStats,
) -> Result<Option<String>, ExportError> {
    let Some(entity) = scene.get(entity_id) else {
        return Ok(None);
    };

    let mut child_blocks = String::new();
    for child_id in &entity.children {
        if let Some(block) = export_entity_tree(
            *child_id,
            scene,
            indent + 1,
            mesh_loader,
            material_loader,
            names,
            stats,
        )? {
            child_blocks.push_str(&block);
        }
    }

    let mesh_block = if let Some(mesh_ref) = &entity.mesh_ref {
        let mesh = mesh_loader(mesh_ref.asset_id)?;
        let material = entity
            .material_binding
            .as_ref()
            .map(|binding| material_loader(binding.material_id))
            .transpose()?
            .unwrap_or_default();
        stats.mesh_count += 1;
        Some(format_mesh_block(&mesh, &material, indent + 1)?)
    } else {
        None
    };

    if mesh_block.is_none() && child_blocks.is_empty() {
        return Ok(None);
    }

    let pad = "    ".repeat(indent);
    let inner = "    ".repeat(indent + 1);
    let prim_name = names.unique(entity_label(entity));
    let (translation, rotation, scale) = transform_to_trs(entity.transform);

    let mut block = format!("{pad}def Xform \"{prim_name}\"\n{pad}{{\n");
    block.push_str(&format!(
        "{inner}double3 xformOp:translate = ({}, {}, {})\n",
        fmt_f64(translation.x as f64),
        fmt_f64(translation.y as f64),
        fmt_f64(translation.z as f64)
    ));
    block.push_str(&format!(
        "{inner}quatf xformOp:orient = ({}, {}, {}, {})\n",
        fmt_f32(rotation.w),
        fmt_f32(rotation.x),
        fmt_f32(rotation.y),
        fmt_f32(rotation.z)
    ));
    block.push_str(&format!(
        "{inner}double3 xformOp:scale = ({}, {}, {})\n",
        fmt_f64(scale.x as f64),
        fmt_f64(scale.y as f64),
        fmt_f64(scale.z as f64)
    ));
    block.push_str(&format!(
        "{inner}uniform token[] xformOpOrder = [\"xformOp:translate\", \"xformOp:orient\", \"xformOp:scale\"]\n"
    ));
    if let Some(mesh_block) = mesh_block {
        block.push('\n');
        block.push_str(&mesh_block);
    }
    block.push_str(&child_blocks);
    block.push_str(&format!("{pad}}}\n"));
    stats.prim_count += 1;
    Ok(Some(block))
}

fn format_mesh_block(
    mesh: &MeshAssetData,
    material: &MaterialAssetData,
    indent: usize,
) -> Result<String, ExportError> {
    mesh.validate()
        .map_err(|err| ExportError::Asset(err.to_string()))?;

    let pad = "    ".repeat(indent);
    let inner = "    ".repeat(indent + 1);
    let triangle_count = mesh.indices.len() / 3;
    let face_counts = vec![3; triangle_count];
    let points = format_vec3_array(&mesh.positions);
    let indices = format_usda_int_array(&mesh.indices);
    let face_vertex_counts = format_usda_int_array(&face_counts);
    let color = material.base_color;

    let mut block = format!("{pad}def Mesh \"Geometry\"\n{pad}{{\n");
    block.push_str(&format!("{inner}point3f[] points = {points}\n"));
    block.push_str(&format!(
        "{inner}int[] faceVertexCounts = {face_vertex_counts}\n"
    ));
    block.push_str(&format!("{inner}int[] faceVertexIndices = {indices}\n"));
    if mesh.normals.len() == mesh.positions.len() {
        let normals = format_vec3_array(&mesh.normals);
        block.push_str(&format!("{inner}normal3f[] normals = {normals}\n"));
    }
    block.push_str(&format!(
        "{inner}color3f[] displayColor = [({}, {}, {})]\n",
        fmt_f32(color[0]),
        fmt_f32(color[1]),
        fmt_f32(color[2])
    ));
    block.push_str(&format!("{pad}}}\n"));
    Ok(block)
}

fn entity_label(entity: &Entity) -> String {
    entity
        .name
        .as_ref()
        .map(|name| name.value.clone())
        .unwrap_or_else(|| entity.id.to_string())
}

#[derive(Debug, Default)]
struct ExportStats {
    prim_count: usize,
    mesh_count: usize,
}

#[derive(Debug, Default)]
struct NameRegistry {
    used: HashSet<String>,
}

impl NameRegistry {
    fn unique(&mut self, label: String) -> String {
        let base = sanitize_prim_name(&label);
        let mut candidate = base.clone();
        let mut suffix = 2usize;
        while !self.used.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        candidate
    }
}

fn sanitize_prim_name(label: &str) -> String {
    let mut sanitized: String = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        sanitized = "Prim".into();
    }
    if sanitized.as_bytes()[0].is_ascii_digit() {
        sanitized = format!("_{sanitized}");
    }
    sanitized
}

fn transform_to_trs(transform: Transform) -> (Vec3, Quat, Vec3) {
    (transform.translation, transform.rotation, transform.scale)
}

fn format_vec3_array(values: &[[f32; 3]]) -> String {
    let entries: Vec<String> = values
        .iter()
        .map(|value| {
            format!(
                "({}, {}, {})",
                fmt_f32(value[0]),
                fmt_f32(value[1]),
                fmt_f32(value[2])
            )
        })
        .collect();
    format!("[{}]", entries.join(", "))
}

fn format_usda_int_array(values: &[u32]) -> String {
    let entries: Vec<String> = values.iter().map(|value| value.to_string()).collect();
    format!("[{}]", entries.join(", "))
}

fn fmt_f32(value: f32) -> String {
    if value.fract() == 0.0 && value.abs() < 1_000_000.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.6}")
    }
}

fn fmt_f64(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1_000_000.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.6}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_project::Project;

    #[test]
    fn exports_mesh_scene_sample_to_usda() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/mesh-scene");
        if !sample.join("manifest.c3d.toml").is_file() {
            return;
        }

        let project = Project::open(&sample).expect("open sample");
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("mesh-scene.usda");

        let report = export_scene_usda(
            project.scene(),
            |asset_id| {
                project
                    .mesh_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                project
                    .material_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            &output,
        )
        .expect("export");

        assert!(report.mesh_count >= 2);
        let contents = fs::read_to_string(&output).expect("read usda");
        assert!(contents.starts_with("#usda 1.0"));
        assert!(contents.contains("def Mesh \"Geometry\""));
        assert!(contents.contains("point3f[] points"));
    }
}
