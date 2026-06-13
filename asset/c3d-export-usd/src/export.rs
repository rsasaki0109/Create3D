use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use c3d_core::math::{Quat, Vec3};
use c3d_core::{AssetId, EntityId};
use c3d_scene_doc::{Entity, SceneDoc};
use c3d_scene_schema::Transform;

/// Texture payload written alongside USDA snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureExportData {
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// MIME type such as `image/png`.
    pub mime_type: String,
}

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
    /// Number of sidecar texture files written.
    pub texture_count: usize,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export a SceneDB document and mesh/material assets to an ASCII USD file.
pub fn export_scene_usda(
    scene: &SceneDoc,
    mesh_loader: impl Fn(AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: impl Fn(AssetId) -> Result<MaterialAssetData, ExportError>,
    texture_loader: impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
    output: impl AsRef<Path>,
) -> Result<UsdExportReport, ExportError> {
    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut names = NameRegistry::default();
    let mut stats = ExportStats::default();
    let mut texture_paths = HashMap::new();
    let mut roots = String::new();
    let mut ctx = ExportContext {
        mesh_loader: &mesh_loader,
        material_loader: &material_loader,
        texture_loader: &texture_loader,
        output,
        texture_paths: &mut texture_paths,
        names: &mut names,
        stats: &mut stats,
    };

    for entity in scene.entities().filter(|entity| entity.parent.is_none()) {
        if let Some(block) =
            export_entity_tree(entity.id, scene, 1, &["Scene".to_string()], &mut ctx)?
        {
            roots.push_str(&block);
        }
    }

    if ctx.stats.mesh_count == 0 {
        return Err(ExportError::EmptyScene);
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
        prim_count: ctx.stats.prim_count,
        mesh_count: ctx.stats.mesh_count,
        texture_count: texture_paths.len(),
        byte_length,
    })
}

fn export_entity_tree(
    entity_id: EntityId,
    scene: &SceneDoc,
    indent: usize,
    path_segments: &[String],
    ctx: &mut ExportContext<'_>,
) -> Result<Option<String>, ExportError> {
    let Some(entity) = scene.get(entity_id) else {
        return Ok(None);
    };

    let prim_name = ctx.names.unique(entity_label(entity));
    let mut current_path = path_segments.to_vec();
    current_path.push(prim_name.clone());
    let prim_path = absolute_prim_path(&current_path);

    let mut child_blocks = String::new();
    for child_id in &entity.children {
        if let Some(block) = export_entity_tree(*child_id, scene, indent + 1, &current_path, ctx)? {
            child_blocks.push_str(&block);
        }
    }

    let (mesh_block, material_block) = if let Some(mesh_ref) = &entity.mesh_ref {
        let mesh = (ctx.mesh_loader)(mesh_ref.asset_id)?;
        let material = entity
            .material_binding
            .as_ref()
            .map(|binding| (ctx.material_loader)(binding.material_id))
            .transpose()?
            .unwrap_or_default();
        ctx.stats.mesh_count += 1;
        let mesh_path = format!("{prim_path}/Geometry");
        let mesh_block = format_mesh_block(&mesh, &material, &mesh_path, indent + 1)?;
        let material_block = format_material_block(
            &material,
            &mesh_path,
            ctx.texture_loader,
            ctx.output,
            ctx.texture_paths,
            indent + 1,
        )?;
        (Some(mesh_block), Some(material_block))
    } else {
        (None, None)
    };

    if mesh_block.is_none() && child_blocks.is_empty() {
        return Ok(None);
    }

    let pad = "    ".repeat(indent);
    let inner = "    ".repeat(indent + 1);
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
    if let Some(material_block) = material_block {
        block.push('\n');
        block.push_str(&material_block);
    }
    block.push_str(&child_blocks);
    block.push_str(&format!("{pad}}}\n"));
    ctx.stats.prim_count += 1;
    Ok(Some(block))
}

struct ExportContext<'a> {
    mesh_loader: &'a dyn Fn(AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: &'a dyn Fn(AssetId) -> Result<MaterialAssetData, ExportError>,
    texture_loader: &'a dyn Fn(AssetId) -> Result<TextureExportData, ExportError>,
    output: &'a Path,
    texture_paths: &'a mut HashMap<AssetId, String>,
    names: &'a mut NameRegistry,
    stats: &'a mut ExportStats,
}

fn format_mesh_block(
    mesh: &MeshAssetData,
    material: &MaterialAssetData,
    mesh_prim_path: &str,
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
    let resolved = material
        .resolved()
        .map_err(|err| ExportError::Asset(err.to_string()))?;
    let color = resolved.base_color;

    let mut block = format!(
        "{pad}def Mesh \"Geometry\" (
{inner}    rel material:binding = <SurfaceMaterial>
{pad})
{pad}{{
"
    );
    block.push_str(&format!("{inner}point3f[] points = {points}\n"));
    block.push_str(&format!(
        "{inner}int[] faceVertexCounts = {face_vertex_counts}\n"
    ));
    block.push_str(&format!("{inner}int[] faceVertexIndices = {indices}\n"));
    if mesh.normals.len() == mesh.positions.len() {
        let normals = format_vec3_array(&mesh.normals);
        block.push_str(&format!("{inner}normal3f[] normals = {normals}\n"));
    }
    if mesh.uvs.len() == mesh.positions.len() {
        let uvs = format_vec2_array(&mesh.uvs);
        block.push_str(&format!("{inner}texCoord2f[] primvars:st = {uvs}\n"));
        block.push_str(&format!(
            "{inner}uniform token primvars:st:interpolation = \"vertex\"\n"
        ));
        let _ = mesh_prim_path;
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

fn format_material_block(
    material: &MaterialAssetData,
    mesh_prim_path: &str,
    texture_loader: &dyn Fn(AssetId) -> Result<TextureExportData, ExportError>,
    output: &Path,
    texture_paths: &mut HashMap<AssetId, String>,
    indent: usize,
) -> Result<String, ExportError> {
    let resolved = material
        .resolved()
        .map_err(|err| ExportError::Asset(err.to_string()))?;
    let color = resolved.base_color;
    let pad = "    ".repeat(indent);
    let inner = "    ".repeat(indent + 1);
    let deeper = "    ".repeat(indent + 2);

    let mut block = format!("{pad}def Material \"SurfaceMaterial\"\n{pad}{{\n");
    block.push_str(&format!(
        "{inner}token outputs:surface.connect = <PreviewSurface.outputs:surface>\n\n"
    ));
    block.push_str(&format!(
        "{inner}def Shader \"PreviewSurface\"\n{inner}{{\n"
    ));
    block.push_str(&format!(
        "{deeper}uniform token info:id = \"UsdPreviewSurface\"\n"
    ));
    block.push_str(&format!("{deeper}float inputs:roughness = 0.9\n"));
    block.push_str(&format!("{deeper}float inputs:metallic = 0.0\n"));

    if let Some(texture_id) = material.base_color_texture {
        let asset_ref = ensure_texture_file(texture_id, texture_loader, output, texture_paths)?;
        block.push_str(&format!(
            "{deeper}color3f inputs:diffuseColor.connect = <DiffuseMap.outputs:rgb>\n"
        ));
        block.push_str(&format!("{inner}}}\n\n"));
        block.push_str(&format!("{inner}def Shader \"DiffuseMap\"\n{inner}{{\n"));
        block.push_str(&format!(
            "{deeper}uniform token info:id = \"UsdUVTexture\"\n"
        ));
        block.push_str(&format!("{deeper}asset inputs:file = @{asset_ref}@\n"));
        block.push_str(&format!(
            "{deeper}float2[] inputs:st.connect = <{mesh_prim_path}.primvars:st>\n"
        ));
        block.push_str(&format!("{deeper}token outputs:rgb\n"));
        block.push_str(&format!("{inner}}}\n"));
    } else {
        block.push_str(&format!(
            "{deeper}color3f inputs:diffuseColor = ({}, {}, {})\n",
            fmt_f32(color[0]),
            fmt_f32(color[1]),
            fmt_f32(color[2])
        ));
        block.push_str(&format!("{inner}}}\n"));
    }

    block.push_str(&format!("{pad}}}\n"));
    Ok(block)
}

fn ensure_texture_file(
    texture_id: AssetId,
    texture_loader: &dyn Fn(AssetId) -> Result<TextureExportData, ExportError>,
    output: &Path,
    texture_paths: &mut HashMap<AssetId, String>,
) -> Result<String, ExportError> {
    if let Some(path) = texture_paths.get(&texture_id) {
        return Ok(path.clone());
    }

    let texture = texture_loader(texture_id)?;
    let extension = texture_extension(&texture.mime_type);
    let directory = texture_directory(output);
    fs::create_dir_all(&directory)?;
    let filename = format!("{texture_id}.{extension}");
    fs::write(directory.join(&filename), &texture.bytes)?;

    let relative = format!(
        "./{}_{}/{}",
        output
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("snapshot"),
        "textures",
        filename
    );
    texture_paths.insert(texture_id, relative.clone());
    Ok(relative)
}

fn texture_directory(output: &Path) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("snapshot");
    output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}_textures"))
}

fn texture_extension(mime_type: &str) -> &str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        _ => "bin",
    }
}

fn absolute_prim_path(segments: &[String]) -> String {
    format!("/{}", segments.join("/"))
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

fn format_vec2_array(values: &[[f32; 2]]) -> String {
    let entries: Vec<String> = values
        .iter()
        .map(|value| format!("({}, {})", fmt_f32(value[0]), fmt_f32(value[1])))
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
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{MaterialBinding, MeshRef};

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
            |asset_id| {
                let bytes = project
                    .texture_bytes(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                Ok(TextureExportData {
                    bytes,
                    mime_type: "image/png".into(),
                })
            },
            &output,
        )
        .expect("export");

        assert!(report.mesh_count >= 2);
        let contents = fs::read_to_string(&output).expect("read usda");
        assert!(contents.starts_with("#usda 1.0"));
        assert!(contents.contains("def Mesh \"Geometry\""));
        assert!(contents.contains("def Material \"SurfaceMaterial\""));
    }

    #[test]
    fn exports_sidecar_texture_for_material() {
        let mesh_id = AssetId::new();
        let material_id = AssetId::new();
        let texture_id = AssetId::new();
        let entity_id = EntityId::new();

        let mut scene = SceneDoc::new();
        let mut entity = Entity::new(entity_id);
        entity.mesh_ref = Some(MeshRef::new(mesh_id));
        entity.material_binding = Some(MaterialBinding::new(material_id));
        scene.insert_entity(entity, None).expect("insert entity");

        let mesh = MeshAssetData {
            version: 1,
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Vec::new(),
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let material = MaterialAssetData {
            version: 1,
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: Some(texture_id),
            graph: None,
        };
        let texture = TextureExportData {
            bytes: vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
                0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
                0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
                0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
                0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ],
            mime_type: "image/png".into(),
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("textured.usda");
        let report = export_scene_usda(
            &scene,
            |asset| {
                if asset == mesh_id {
                    Ok(mesh.clone())
                } else {
                    Err(ExportError::Asset(format!("unexpected mesh {asset}")))
                }
            },
            |asset| {
                if asset == material_id {
                    Ok(material.clone())
                } else {
                    Err(ExportError::Asset(format!("unexpected material {asset}")))
                }
            },
            |asset| {
                if asset == texture_id {
                    Ok(texture.clone())
                } else {
                    Err(ExportError::Asset(format!("unexpected texture {asset}")))
                }
            },
            &output,
        )
        .expect("export");

        assert_eq!(report.texture_count, 1);
        let contents = fs::read_to_string(&output).expect("read usda");
        assert!(contents.contains("UsdUVTexture"));
        assert!(contents.contains("primvars:st"));
        assert!(temp.path().join("textured_textures").is_dir());
    }
}
