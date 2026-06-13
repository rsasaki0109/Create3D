use std::fs;
use std::path::Path;

use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use c3d_core::math::{Quat, Vec3};
use c3d_core::EntityId;
use c3d_scene_doc::{Entity, SceneDoc};
use c3d_scene_schema::Transform;
use serde_json::{json, Value};

/// Export failures.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Scene had no exportable mesh content.
    #[error("scene has no mesh entities to export")]
    EmptyScene,
    /// Asset lookup failure.
    #[error("asset error: {0}")]
    Asset(String),
}

/// Summary of an export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GltfExportReport {
    /// Number of glTF nodes written.
    pub node_count: usize,
    /// Number of glTF meshes written.
    pub mesh_count: usize,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export a SceneDB document and mesh/material assets to a binary GLB file.
pub fn export_scene_glb(
    scene: &SceneDoc,
    mesh_loader: impl Fn(c3d_core::AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: impl Fn(c3d_core::AssetId) -> Result<MaterialAssetData, ExportError>,
    output: impl AsRef<Path>,
) -> Result<GltfExportReport, ExportError> {
    let mut builder = GlbBuilder::new();
    let mut roots = Vec::new();
    for entity in scene.entities().filter(|entity| entity.parent.is_none()) {
        let node_index = export_entity_tree(
            entity.id,
            scene,
            &mut builder,
            &mesh_loader,
            &material_loader,
        )?;
        if let Some(index) = node_index {
            roots.push(index);
        }
    }

    if builder.mesh_count == 0 {
        return Err(ExportError::EmptyScene);
    }

    let mesh_count = builder.mesh_count;
    let node_count = builder.node_count;
    let gltf = builder.finish(roots);
    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    write_glb(output, &gltf.json, &gltf.bin)?;
    let byte_length = fs::metadata(output)?.len();
    Ok(GltfExportReport {
        node_count,
        mesh_count,
        byte_length,
    })
}

fn export_entity_tree(
    entity_id: EntityId,
    scene: &SceneDoc,
    builder: &mut GlbBuilder,
    mesh_loader: &impl Fn(c3d_core::AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: &impl Fn(c3d_core::AssetId) -> Result<MaterialAssetData, ExportError>,
) -> Result<Option<usize>, ExportError> {
    let Some(entity) = scene.get(entity_id) else {
        return Ok(None);
    };

    let mut child_indices = Vec::new();
    for child_id in &entity.children {
        if let Some(index) =
            export_entity_tree(*child_id, scene, builder, mesh_loader, material_loader)?
        {
            child_indices.push(index);
        }
    }

    let mesh_index = if let Some(mesh_ref) = &entity.mesh_ref {
        let mesh = mesh_loader(mesh_ref.asset_id)?;
        let material = entity
            .material_binding
            .as_ref()
            .map(|binding| material_loader(binding.material_id))
            .transpose()?
            .unwrap_or_default();
        Some(builder.add_mesh(entity_label(entity), &mesh, &material)?)
    } else {
        None
    };

    if mesh_index.is_none() && child_indices.is_empty() {
        return Ok(None);
    }

    Ok(Some(builder.add_node(
        entity_label(entity),
        entity.transform,
        mesh_index,
        child_indices,
    )))
}

fn entity_label(entity: &Entity) -> String {
    entity
        .name
        .as_ref()
        .map(|name| name.value.clone())
        .unwrap_or_else(|| entity.id.to_string())
}

struct GlbDocument {
    json: Value,
    bin: Vec<u8>,
}

struct GlbBuilder {
    bin: Vec<u8>,
    accessors: Vec<Value>,
    buffer_views: Vec<Value>,
    meshes: Vec<Value>,
    materials: Vec<Value>,
    nodes: Vec<Value>,
    mesh_count: usize,
    node_count: usize,
}

impl GlbBuilder {
    fn new() -> Self {
        Self {
            bin: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            nodes: Vec::new(),
            mesh_count: 0,
            node_count: 0,
        }
    }

    fn add_mesh(
        &mut self,
        name: String,
        mesh: &MeshAssetData,
        material: &MaterialAssetData,
    ) -> Result<usize, ExportError> {
        mesh.validate()
            .map_err(|err| ExportError::Asset(err.to_string()))?;

        let positions = align_offset(&mut self.bin);
        for position in &mesh.positions {
            for component in position {
                self.bin.extend_from_slice(&component.to_le_bytes());
            }
        }
        let positions_view = self.buffer_view(positions, mesh.positions.len() * 12);
        let (min_pos, max_pos) = position_bounds(&mesh.positions);
        let positions_accessor =
            self.float_vec3_accessor(positions_view, mesh.positions.len(), min_pos, max_pos);

        let normals_accessor = if mesh.normals.len() == mesh.positions.len() {
            let normals = align_offset(&mut self.bin);
            for normal in &mesh.normals {
                for component in normal {
                    self.bin.extend_from_slice(&component.to_le_bytes());
                }
            }
            let view = self.buffer_view(normals, mesh.normals.len() * 12);
            Some(self.float_vec3_accessor(view, mesh.normals.len(), None, None))
        } else {
            None
        };

        let indices = align_offset(&mut self.bin);
        for index in &mesh.indices {
            self.bin.extend_from_slice(&index.to_le_bytes());
        }
        let indices_view = self.buffer_view(indices, mesh.indices.len() * 4);
        let indices_accessor = self.indices_accessor(indices_view, mesh.indices.len());

        let mut attributes = json!({ "POSITION": positions_accessor });
        if let Some(normal) = normals_accessor {
            attributes
                .as_object_mut()
                .expect("attributes object")
                .insert("NORMAL".into(), json!(normal));
        }

        let material_index = self.materials.len();
        let color = material.base_color;
        self.materials.push(json!({
            "name": format!("material-{material_index}"),
            "pbrMetallicRoughness": {
                "baseColorFactor": [color[0], color[1], color[2], color[3]],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.9
            }
        }));
        let mesh_index = self.meshes.len();
        self.meshes.push(json!({
            "name": name,
            "primitives": [{
                "attributes": attributes,
                "indices": indices_accessor,
                "material": material_index
            }]
        }));
        self.mesh_count += 1;
        Ok(mesh_index)
    }

    fn add_node(
        &mut self,
        name: String,
        transform: Transform,
        mesh: Option<usize>,
        children: Vec<usize>,
    ) -> usize {
        let (translation, rotation, scale) = transform_to_trs(transform);
        let mut node = json!({
            "name": name,
            "translation": [translation.x, translation.y, translation.z],
            "rotation": [rotation.x, rotation.y, rotation.z, rotation.w],
            "scale": [scale.x, scale.y, scale.z],
        });
        if let Some(mesh) = mesh {
            node["mesh"] = json!(mesh);
        }
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        let index = self.nodes.len();
        self.nodes.push(node);
        self.node_count += 1;
        index
    }

    fn buffer_view(&mut self, offset: usize, length: usize) -> usize {
        let index = self.buffer_views.len();
        self.buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": length
        }));
        index
    }

    fn float_vec3_accessor(
        &mut self,
        view: usize,
        count: usize,
        min: Option<[f32; 3]>,
        max: Option<[f32; 3]>,
    ) -> usize {
        let index = self.accessors.len();
        let mut accessor = json!({
            "bufferView": view,
            "componentType": 5126,
            "count": count,
            "type": "VEC3"
        });
        if let Some(min) = min {
            accessor["min"] = json!(min);
        }
        if let Some(max) = max {
            accessor["max"] = json!(max);
        }
        self.accessors.push(accessor);
        index
    }

    fn indices_accessor(&mut self, view: usize, count: usize) -> usize {
        let index = self.accessors.len();
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": 5125,
            "count": count,
            "type": "SCALAR"
        }));
        index
    }

    fn finish(self, roots: Vec<usize>) -> GlbDocument {
        let json = json!({
            "asset": {
                "version": "2.0",
                "generator": "Create3D"
            },
            "scene": 0,
            "scenes": [{ "nodes": roots }],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "materials": self.materials,
            "accessors": self.accessors,
            "bufferViews": self.buffer_views,
            "buffers": [{ "byteLength": self.bin.len() }]
        });
        GlbDocument {
            json,
            bin: self.bin,
        }
    }
}

fn transform_to_trs(transform: Transform) -> (Vec3, Quat, Vec3) {
    (transform.translation, transform.rotation, transform.scale)
}

fn position_bounds(positions: &[[f32; 3]]) -> (Option<[f32; 3]>, Option<[f32; 3]>) {
    if positions.is_empty() {
        return (None, None);
    }
    let mut min = positions[0];
    let mut max = positions[0];
    for position in positions.iter().skip(1) {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (Some(min), Some(max))
}

fn align_offset(buffer: &mut Vec<u8>) -> usize {
    while !buffer.len().is_multiple_of(4) {
        buffer.push(0);
    }
    buffer.len()
}

fn write_glb(path: &Path, json: &Value, bin: &[u8]) -> Result<(), ExportError> {
    let mut json_bytes = serde_json::to_vec(json)?;
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut bin_bytes = bin.to_vec();
    while !bin_bytes.len().is_multiple_of(4) {
        bin_bytes.push(0);
    }

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    let mut output = Vec::with_capacity(total_length);
    output.extend_from_slice(b"glTF");
    output.extend_from_slice(&2u32.to_le_bytes());
    output.extend_from_slice(&(total_length as u32).to_le_bytes());
    output.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(b"JSON");
    output.extend_from_slice(&json_bytes);
    output.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(b"BIN\0");
    output.extend_from_slice(&bin_bytes);
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_import_gltf::import_gltf_path;
    use c3d_project::Project;

    #[test]
    fn exports_mesh_scene_sample_and_round_trips() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/mesh-scene");
        if !sample.join("manifest.c3d.toml").is_file() {
            return;
        }

        let project = Project::open(&sample).expect("open sample");
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("mesh-scene.glb");

        let report = export_scene_glb(
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
        assert!(output.is_file());

        let imported = import_gltf_path(&output).expect("re-import");
        assert!(!imported.meshes.is_empty());
    }
}
