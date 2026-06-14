use std::collections::HashMap;
use std::fs;
use std::path::Path;

use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use c3d_asset_pointcloud::{PointCloudAssetData, PointCloudChunkPayload};
use c3d_core::math::{Quat, Vec3};
use c3d_core::{AssetId, EntityId};
use c3d_scene_doc::{Entity, SceneDoc};
use c3d_scene_schema::{PointCloudRef, Transform};
use serde_json::{json, Value};

/// Texture payload embedded into exported glTF buffers.
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
    /// JSON serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Scene had no exportable mesh or point cloud content.
    #[error("scene has no mesh or point cloud entities to export")]
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
    /// Number of triangle mesh glTF meshes written.
    pub mesh_count: usize,
    /// Number of point cloud entities exported as glTF POINTS meshes.
    pub point_cloud_count: usize,
    /// Total number of points written across all point cloud meshes.
    pub point_count: u64,
    /// Number of embedded texture images written.
    pub texture_count: usize,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export a SceneDB document with mesh and point cloud assets to a binary GLB file.
pub fn export_scene_glb(
    scene: &SceneDoc,
    mesh_loader: impl Fn(AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: impl Fn(AssetId) -> Result<MaterialAssetData, ExportError>,
    texture_loader: impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
    point_cloud_metadata_loader: impl Fn(AssetId) -> Result<PointCloudAssetData, ExportError>,
    point_cloud_chunk_loader: impl Fn(AssetId) -> Result<PointCloudChunkPayload, ExportError>,
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
            &texture_loader,
            &point_cloud_metadata_loader,
            &point_cloud_chunk_loader,
        )?;
        if let Some(index) = node_index {
            roots.push(index);
        }
    }

    if builder.mesh_count == 0 && builder.point_cloud_count == 0 {
        return Err(ExportError::EmptyScene);
    }

    let mesh_count = builder.mesh_count;
    let point_cloud_count = builder.point_cloud_count;
    let point_count = builder.point_count;
    let node_count = builder.node_count;
    let texture_count = builder.texture_count;
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
        point_cloud_count,
        point_count,
        texture_count,
        byte_length,
    })
}

#[allow(clippy::too_many_arguments)]
fn export_entity_tree(
    entity_id: EntityId,
    scene: &SceneDoc,
    builder: &mut GlbBuilder,
    mesh_loader: &impl Fn(AssetId) -> Result<MeshAssetData, ExportError>,
    material_loader: &impl Fn(AssetId) -> Result<MaterialAssetData, ExportError>,
    texture_loader: &impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
    point_cloud_metadata_loader: &impl Fn(AssetId) -> Result<PointCloudAssetData, ExportError>,
    point_cloud_chunk_loader: &impl Fn(AssetId) -> Result<PointCloudChunkPayload, ExportError>,
) -> Result<Option<usize>, ExportError> {
    let Some(entity) = scene.get(entity_id) else {
        return Ok(None);
    };

    let mut child_indices = Vec::new();
    for child_id in &entity.children {
        if let Some(index) = export_entity_tree(
            *child_id,
            scene,
            builder,
            mesh_loader,
            material_loader,
            texture_loader,
            point_cloud_metadata_loader,
            point_cloud_chunk_loader,
        )? {
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
        Some(builder.add_mesh(entity_label(entity), &mesh, &material, texture_loader)?)
    } else if let Some(point_cloud_ref) = &entity.point_cloud_ref {
        let metadata = point_cloud_metadata_loader(point_cloud_ref.asset_id)?;
        match collect_entity_point_cloud(point_cloud_ref, &metadata, point_cloud_chunk_loader)? {
            Some(points) => Some(builder.add_point_cloud(entity_label(entity), &points)?),
            None => None,
        }
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

struct CollectedPointCloud {
    positions: Vec<[f32; 3]>,
    colors: Vec<[u8; 3]>,
    include_color: bool,
}

fn collect_entity_point_cloud(
    point_cloud_ref: &PointCloudRef,
    metadata: &PointCloudAssetData,
    chunk_loader: &impl Fn(AssetId) -> Result<PointCloudChunkPayload, ExportError>,
) -> Result<Option<CollectedPointCloud>, ExportError> {
    let mut collected = CollectedPointCloud {
        positions: Vec::new(),
        colors: Vec::new(),
        include_color: metadata.has_rgb || metadata.has_intensity,
    };

    for record in &metadata.chunks {
        let mut payload = chunk_loader(record.blob_asset_id)?;
        if let Some(crop) = point_cloud_ref.crop_filter {
            payload = payload.crop(&crop);
        }
        if payload.point_count() == 0 {
            continue;
        }

        for (index, position) in payload.positions.iter().enumerate() {
            collected.positions.push(*position);
            if metadata.has_rgb {
                let color = payload
                    .colors
                    .get(index)
                    .copied()
                    .unwrap_or([255, 255, 255]);
                collected.colors.push(color);
            } else if metadata.has_intensity {
                let value = payload.intensity.get(index).copied().unwrap_or(0.0);
                let channel = intensity_to_u8(value);
                collected.colors.push([channel, channel, channel]);
            }
        }
    }

    if collected.positions.is_empty() {
        Ok(None)
    } else {
        Ok(Some(collected))
    }
}

fn intensity_to_u8(value: f32) -> u8 {
    value.clamp(0.0, 1.0).mul_add(255.0, 0.0) as u8
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
    images: Vec<Value>,
    samplers: Vec<Value>,
    textures: Vec<Value>,
    texture_indices: HashMap<AssetId, usize>,
    nodes: Vec<Value>,
    mesh_count: usize,
    point_cloud_count: usize,
    point_count: u64,
    node_count: usize,
    texture_count: usize,
}

impl GlbBuilder {
    fn new() -> Self {
        Self {
            bin: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            images: Vec::new(),
            samplers: Vec::new(),
            textures: Vec::new(),
            texture_indices: HashMap::new(),
            nodes: Vec::new(),
            mesh_count: 0,
            point_cloud_count: 0,
            point_count: 0,
            node_count: 0,
            texture_count: 0,
        }
    }

    fn add_mesh(
        &mut self,
        name: String,
        mesh: &MeshAssetData,
        material: &MaterialAssetData,
        texture_loader: &impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
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
        if mesh.uvs.len() == mesh.positions.len() {
            let uvs = align_offset(&mut self.bin);
            for uv in &mesh.uvs {
                for component in uv {
                    self.bin.extend_from_slice(&component.to_le_bytes());
                }
            }
            let view = self.buffer_view(uvs, mesh.uvs.len() * 8);
            let uv_accessor = self.float_vec2_accessor(view, mesh.uvs.len());
            attributes
                .as_object_mut()
                .expect("attributes object")
                .insert("TEXCOORD_0".into(), json!(uv_accessor));
        }

        let material_index = self.add_material(material, texture_loader)?;
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

    fn add_point_cloud(
        &mut self,
        name: String,
        points: &CollectedPointCloud,
    ) -> Result<usize, ExportError> {
        let positions = align_offset(&mut self.bin);
        for position in &points.positions {
            for component in position {
                self.bin.extend_from_slice(&component.to_le_bytes());
            }
        }
        let positions_view = self.buffer_view(positions, points.positions.len() * 12);
        let (min_pos, max_pos) = position_bounds(&points.positions);
        let positions_accessor =
            self.float_vec3_accessor(positions_view, points.positions.len(), min_pos, max_pos);

        let mut attributes = json!({ "POSITION": positions_accessor });
        if points.include_color && points.colors.len() == points.positions.len() {
            let colors = align_offset(&mut self.bin);
            for color in &points.colors {
                self.bin.extend_from_slice(color);
            }
            let view = self.buffer_view(colors, points.colors.len() * 3);
            let color_accessor = self.uchar_vec3_accessor(view, points.colors.len());
            attributes
                .as_object_mut()
                .expect("attributes object")
                .insert("COLOR_0".into(), json!(color_accessor));
        }

        let material_index = self.add_vertex_color_material()?;
        let mesh_index = self.meshes.len();
        self.meshes.push(json!({
            "name": name,
            "primitives": [{
                "attributes": attributes,
                "mode": 0,
                "material": material_index
            }]
        }));
        self.point_cloud_count += 1;
        self.point_count += points.positions.len() as u64;
        Ok(mesh_index)
    }

    fn add_vertex_color_material(&mut self) -> Result<usize, ExportError> {
        let material_index = self.materials.len();
        self.materials.push(json!({
            "name": format!("point-cloud-material-{material_index}"),
            "pbrMetallicRoughness": {
                "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0
            }
        }));
        Ok(material_index)
    }

    fn add_material(
        &mut self,
        material: &MaterialAssetData,
        texture_loader: &impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
    ) -> Result<usize, ExportError> {
        let resolved = material
            .resolved()
            .map_err(|err| ExportError::Asset(err.to_string()))?;
        let color = resolved.base_color;
        let mut pbr = json!({
            "baseColorFactor": [color[0], color[1], color[2], color[3]],
            "metallicFactor": 0.0,
            "roughnessFactor": 0.9
        });
        if let Some(texture_id) = material.base_color_texture {
            let texture_index = self.texture_index(texture_id, texture_loader)?;
            pbr["baseColorTexture"] = json!({ "index": texture_index });
        }
        let material_index = self.materials.len();
        self.materials.push(json!({
            "name": format!("material-{material_index}"),
            "pbrMetallicRoughness": pbr
        }));
        Ok(material_index)
    }

    fn texture_index(
        &mut self,
        texture_id: AssetId,
        texture_loader: &impl Fn(AssetId) -> Result<TextureExportData, ExportError>,
    ) -> Result<usize, ExportError> {
        if let Some(index) = self.texture_indices.get(&texture_id) {
            return Ok(*index);
        }

        if self.samplers.is_empty() {
            self.samplers.push(json!({
                "magFilter": 9729,
                "minFilter": 9729,
                "wrapS": 10497,
                "wrapT": 10497
            }));
        }

        let texture = texture_loader(texture_id)?;
        let offset = align_offset(&mut self.bin);
        self.bin.extend_from_slice(&texture.bytes);
        let view_index = self.buffer_view(offset, texture.bytes.len());
        let image_index = self.images.len();
        self.images.push(json!({
            "bufferView": view_index,
            "mimeType": texture.mime_type
        }));
        let texture_index = self.textures.len();
        self.textures.push(json!({
            "sampler": 0,
            "source": image_index
        }));
        self.texture_indices.insert(texture_id, texture_index);
        self.texture_count += 1;
        Ok(texture_index)
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

    fn float_vec2_accessor(&mut self, view: usize, count: usize) -> usize {
        let index = self.accessors.len();
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": 5126,
            "count": count,
            "type": "VEC2"
        }));
        index
    }

    fn uchar_vec3_accessor(&mut self, view: usize, count: usize) -> usize {
        let index = self.accessors.len();
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": 5121,
            "count": count,
            "type": "VEC3",
            "normalized": true
        }));
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
        let mut json = json!({
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
        if !self.images.is_empty() {
            json["images"] = json!(self.images);
            json["samplers"] = json!(self.samplers);
            json["textures"] = json!(self.textures);
        }
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
            |asset_id| {
                let bytes = project
                    .texture_bytes(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                let mime_type = project
                    .assets()
                    .get(asset_id)
                    .and_then(|record| record.mime_type.clone())
                    .filter(|mime| !mime.is_empty())
                    .unwrap_or_else(|| "image/png".into());
                Ok(TextureExportData { bytes, mime_type })
            },
            |asset_id| {
                project
                    .point_cloud_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = project
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                PointCloudChunkPayload::from_bytes(&bytes)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            &output,
        )
        .expect("export");

        assert!(report.mesh_count >= 2);
        assert_eq!(report.point_cloud_count, 0);
        assert!(output.is_file());

        let imported = import_gltf_path(&output).expect("re-import");
        assert!(!imported.meshes.is_empty());
    }

    #[test]
    fn exports_embedded_base_color_texture() {
        use c3d_scene_doc::Entity;
        use c3d_scene_schema::{MaterialBinding, MeshRef};

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
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let material = MaterialAssetData {
            version: 1,
            base_color: [0.2, 0.4, 0.8, 1.0],
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
        let output = temp.path().join("textured.glb");
        let report = export_scene_glb(
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
            |_| Err(ExportError::Asset("unexpected point cloud metadata".into())),
            |_| Err(ExportError::Asset("unexpected point cloud chunk".into())),
            &output,
        )
        .expect("export");

        assert_eq!(report.texture_count, 1);
        assert_eq!(report.point_cloud_count, 0);
        let gltf = read_glb_json(&output).expect("read glb json");
        assert_eq!(gltf["textures"].as_array().map(Vec::len), Some(1));
        assert!(gltf["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"].is_object());
    }

    #[test]
    fn exports_point_cloud_scene_sample_as_points_primitive() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/point-cloud-scene");
        if !sample.join("manifest.c3d.toml").is_file() {
            return;
        }

        let project = Project::open(&sample).expect("open sample");
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("point-cloud-scene.glb");

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
            |asset_id| {
                let data = project
                    .texture_export_data(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                Ok(TextureExportData {
                    bytes: data.bytes,
                    mime_type: data.mime_type,
                })
            },
            |asset_id| {
                project
                    .point_cloud_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = project
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                PointCloudChunkPayload::from_bytes(&bytes)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            &output,
        )
        .expect("export");

        assert_eq!(report.mesh_count, 0);
        assert!(report.point_cloud_count >= 1);
        assert!(report.point_count > 0);

        let gltf = read_glb_json(&output).expect("read glb json");
        let primitive = &gltf["meshes"][0]["primitives"][0];
        assert_eq!(primitive["mode"].as_u64(), Some(0));
        assert!(primitive["attributes"]["POSITION"].is_number());
        assert!(primitive["attributes"]["COLOR_0"].is_number());
    }

    fn read_glb_json(path: &Path) -> Result<Value, ExportError> {
        let bytes = fs::read(path)?;
        if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
            return Err(ExportError::Asset("invalid glb header".into()));
        }
        let json_length =
            u32::from_le_bytes(bytes[12..16].try_into().expect("json length")) as usize;
        let json_start = 20;
        let json_end = json_start + json_length;
        Ok(serde_json::from_slice(&bytes[json_start..json_end])?)
    }
}
