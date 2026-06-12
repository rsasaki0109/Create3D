use std::path::Path;

use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use c3d_core::math::{Quat, Vec3};
use c3d_scene_schema::Transform;
use gltf::image::Format;
use gltf::mesh::util::ReadIndices;
use image::{ExtendedColorType, ImageEncoder};

use crate::{ImportError, ImportResult};

/// Imported texture bytes prior to AssetDB insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTexture {
    /// Debug name.
    pub name: String,
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// MIME type when known.
    pub mime_type: String,
}

/// Imported material prior to AssetDB insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMaterial {
    /// Debug name.
    pub name: String,
    /// Material payload.
    pub data: MaterialAssetData,
    /// Index into [`GltfImportResult::textures`].
    pub texture_index: Option<usize>,
}

/// Imported mesh prior to AssetDB insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMesh {
    /// Debug name.
    pub name: String,
    /// Mesh payload.
    pub data: MeshAssetData,
    /// Index into [`GltfImportResult::materials`].
    pub material_index: Option<usize>,
}

/// Scene node imported from glTF.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedNode {
    /// Optional node name.
    pub name: Option<String>,
    /// Local transform.
    pub transform: Transform,
    /// Index into [`GltfImportResult::meshes`].
    pub mesh_index: Option<usize>,
    /// Child nodes in scene order.
    pub children: Vec<ImportedNode>,
}

/// Parsed glTF content ready for AssetDB and SceneDB insertion.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GltfImportResult {
    /// Imported textures.
    pub textures: Vec<ImportedTexture>,
    /// Imported materials.
    pub materials: Vec<ImportedMaterial>,
    /// Imported meshes.
    pub meshes: Vec<ImportedMesh>,
    /// Root scene nodes.
    pub root_nodes: Vec<ImportedNode>,
}

/// Import a glTF/GLB file from disk.
pub fn import_gltf_path(path: impl AsRef<Path>) -> ImportResult<GltfImportResult> {
    let bytes = std::fs::read(path)?;
    import_gltf_bytes(&bytes)
}

/// Import a glTF/GLB file from memory.
pub fn import_gltf_bytes(bytes: &[u8]) -> ImportResult<GltfImportResult> {
    let (document, buffer_data, image_data) =
        gltf::import_slice(bytes).map_err(|err| ImportError::Gltf(err.to_string()))?;

    let mut textures = Vec::new();
    for (index, texture) in document.textures().enumerate() {
        let image = texture.source();
        let image = &image_data[image.index()];
        let (bytes, mime_type) = encode_gltf_image(image)?;
        textures.push(ImportedTexture {
            name: texture
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("texture-{index}")),
            bytes,
            mime_type,
        });
    }

    let mut materials = Vec::new();
    for (index, material) in document.materials().enumerate() {
        let pbr = material.pbr_metallic_roughness();
        let base_color = pbr.base_color_factor();
        let texture_index = pbr.base_color_texture().map(|info| info.texture().index());
        materials.push(ImportedMaterial {
            name: material
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("material-{index}")),
            data: MaterialAssetData {
                version: 1,
                base_color,
                base_color_texture: None,
                graph: None,
            },
            texture_index,
        });
    }

    let mut meshes = Vec::new();
    for (mesh_index, mesh) in document.meshes().enumerate() {
        for (primitive_index, primitive) in mesh.primitives().enumerate() {
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()].0));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| ImportError::Invalid("mesh primitive missing positions".into()))?
                .map(|value| [value[0], value[1], value[2]])
                .collect();
            let normals = reader
                .read_normals()
                .map(|iter| iter.map(|value| [value[0], value[1], value[2]]).collect())
                .unwrap_or_default();
            let uvs = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().map(|value| [value[0], value[1]]).collect())
                .unwrap_or_default();
            let indices = read_indices(reader.read_indices())?;

            let name = mesh
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("mesh-{mesh_index}"));
            let primitive_name = if mesh.primitives().len() > 1 {
                format!("{name}-primitive-{primitive_index}")
            } else {
                name
            };

            meshes.push(ImportedMesh {
                name: primitive_name,
                data: MeshAssetData {
                    version: 1,
                    positions,
                    normals,
                    uvs,
                    tangents: Vec::new(),
                    indices,
                },
                material_index: primitive.material().index(),
            });
        }
    }

    let default_scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or_else(|| ImportError::Invalid("glTF file has no scenes".into()))?;

    let mut root_nodes = Vec::new();
    for node in default_scene.nodes() {
        root_nodes.push(read_node(node)?);
    }

    Ok(GltfImportResult {
        textures,
        materials,
        meshes,
        root_nodes,
    })
}

fn read_node(node: gltf::Node<'_>) -> ImportResult<ImportedNode> {
    let (translation, rotation, scale) = node.transform().decomposed();
    let transform = Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation: Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    };

    let mesh_index = node.mesh().map(|mesh| mesh.index());
    let mut children = Vec::new();
    for child in node.children() {
        children.push(read_node(child)?);
    }

    Ok(ImportedNode {
        name: node.name().map(str::to_string),
        transform,
        mesh_index,
        children,
    })
}

fn read_indices(indices: Option<ReadIndices<'_>>) -> ImportResult<Vec<u32>> {
    let Some(indices) = indices else {
        return Err(ImportError::Invalid(
            "mesh primitive missing indices".into(),
        ));
    };

    Ok(match indices {
        ReadIndices::U8(iter) => iter.map(u32::from).collect(),
        ReadIndices::U16(iter) => iter.map(u32::from).collect(),
        ReadIndices::U32(iter) => iter.collect(),
    })
}

fn encode_gltf_image(image: &gltf::image::Data) -> ImportResult<(Vec<u8>, String)> {
    let rgba = match image.format {
        Format::R8G8B8A8 => image.pixels.clone(),
        Format::R8G8B8 => image
            .pixels
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        Format::R8 => image
            .pixels
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        other => {
            return Err(ImportError::Image(format!(
                "unsupported glTF image format: {other:?}"
            )))
        }
    };

    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&rgba, image.width, image.height, ExtendedColorType::Rgba8)
        .map_err(|err| ImportError::Image(err.to_string()))?;
    Ok((bytes, "image/png".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_glb_is_rejected() {
        let err = import_gltf_bytes(b"not-a-glb").expect_err("invalid glb");
        assert!(matches!(err, ImportError::Gltf(_)));
    }
}
