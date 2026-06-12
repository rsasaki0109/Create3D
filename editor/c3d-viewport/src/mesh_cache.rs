use std::collections::{HashMap, HashSet};

use c3d_asset_db::AssetDb;
use c3d_asset_material::{MaterialAsset, MaterialAssetData};
use c3d_asset_mesh::{MeshAsset, MeshAssetData};
use c3d_core::AssetId;
use c3d_rhi::{BufferHandle, BufferInit, IndexFormat, RhiBackend};
use c3d_rhi_wgpu::WgpuBackend;

use crate::mesh::Vertex;
use crate::mode::ViewportShadingMode;

/// GPU-resident mesh data keyed by mesh asset id.
#[derive(Debug, Clone, Copy)]
pub struct CachedMeshDraw {
    /// Vertex buffer handle.
    pub vertex_buffer: BufferHandle,
    /// Index buffer handle.
    pub index_buffer: BufferHandle,
    /// Wireframe line vertex buffer handle.
    pub wireframe_buffer: BufferHandle,
    /// Number of indices to draw.
    pub index_count: u32,
    /// Number of wireframe vertices to draw.
    pub wireframe_vertex_count: u32,
    /// Index buffer format.
    pub index_format: IndexFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    mesh_id: AssetId,
    material_id: Option<AssetId>,
    shading_mode: ViewportShadingMode,
}

/// Cache that uploads imported mesh assets to GPU buffers.
#[derive(Default)]
pub struct MeshGpuCache {
    entries: HashMap<CacheKey, CachedMeshDraw>,
}

impl MeshGpuCache {
    /// Ensure a mesh asset is uploaded and return its GPU draw data.
    pub fn prepare(
        &mut self,
        backend: &mut WgpuBackend,
        assets: &AssetDb,
        mesh_id: AssetId,
        material_id: Option<AssetId>,
        shading_mode: ViewportShadingMode,
    ) -> c3d_rhi::RhiResult<CachedMeshDraw> {
        let key = CacheKey {
            mesh_id,
            material_id,
            shading_mode,
        };
        if let Some(entry) = self.entries.get(&key) {
            return Ok(*entry);
        }

        let mesh_bytes = assets
            .read_blob(mesh_id)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
        let mesh = MeshAsset::decode(&mesh_bytes)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;

        let material = if let Some(material_id) = material_id {
            let bytes = assets
                .read_blob(material_id)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
            MaterialAsset::decode(&bytes)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?
                .resolved()
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?
        } else {
            MaterialAssetData::default()
        };

        let texture = if shading_mode == ViewportShadingMode::Material {
            if let Some(texture_id) = material.base_color_texture {
                let bytes = assets
                    .read_blob(texture_id)
                    .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
                Some(
                    decode_texture_rgba(&bytes)
                        .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?,
                )
            } else {
                None
            }
        } else {
            None
        };

        let vertices = bake_vertices(&mesh, &material, texture.as_ref(), shading_mode);
        let wireframe_vertices = wireframe_vertices(&mesh);
        let (indices, index_format) = encode_indices(&mesh.indices);

        let vertex_buffer = backend.create_buffer_init(BufferInit {
            label: "imported-mesh-vertices",
            contents: cast_slice(&vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;
        let index_buffer = backend.create_buffer_init(BufferInit {
            label: "imported-mesh-indices",
            contents: cast_slice(&indices),
            vertex: false,
            index: true,
            uniform: false,
        })?;
        let wireframe_buffer = backend.create_buffer_init(BufferInit {
            label: "imported-mesh-wireframe",
            contents: cast_slice(&wireframe_vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;

        let entry = CachedMeshDraw {
            vertex_buffer,
            index_buffer,
            wireframe_buffer,
            index_count: mesh.indices.len() as u32,
            wireframe_vertex_count: wireframe_vertices.len() as u32,
            index_format,
        };
        self.entries.insert(key, entry);
        Ok(entry)
    }

    /// Borrow cached GPU mesh data when available.
    pub fn get(
        &self,
        mesh_id: AssetId,
        material_id: Option<AssetId>,
        shading_mode: ViewportShadingMode,
    ) -> Option<CachedMeshDraw> {
        self.entries
            .get(&CacheKey {
                mesh_id,
                material_id,
                shading_mode,
            })
            .copied()
    }

    /// Drop cached GPU data for one mesh asset.
    pub fn invalidate(&mut self, mesh_id: AssetId) {
        self.entries.retain(|key, _| key.mesh_id != mesh_id);
    }

    /// Drop all cached GPU mesh data.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

fn bake_vertices(
    mesh: &MeshAssetData,
    material: &MaterialAssetData,
    texture: Option<&DecodedTexture>,
    shading_mode: ViewportShadingMode,
) -> Vec<Vertex> {
    mesh.positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let color = match shading_mode {
                ViewportShadingMode::Solid => {
                    let normal = mesh.normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]);
                    let shade = normal[1].abs() * 0.35 + 0.45;
                    [shade, shade, shade, 1.0]
                }
                ViewportShadingMode::Wireframe => [0.9, 0.9, 0.9, 1.0],
                ViewportShadingMode::Material => {
                    let uv = mesh.uvs.get(index).copied().unwrap_or([0.0, 0.0]);
                    let mut color = material.base_color;
                    if let Some(texture) = texture {
                        let sample = sample_texture(texture, uv);
                        color = [
                            color[0] * sample[0],
                            color[1] * sample[1],
                            color[2] * sample[2],
                            color[3] * sample[3],
                        ];
                    }
                    color
                }
            };
            Vertex {
                position: *position,
                color,
            }
        })
        .collect()
}

fn wireframe_vertices(mesh: &MeshAssetData) -> Vec<Vertex> {
    let mut edges = HashSet::new();
    for triangle in mesh.indices.chunks_exact(3) {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let edge = if a < b { (a, b) } else { (b, a) };
            edges.insert(edge);
        }
    }

    edges
        .into_iter()
        .flat_map(|(a, b)| {
            let start = mesh.positions[a as usize];
            let end = mesh.positions[b as usize];
            let color = [0.95, 0.95, 0.95, 1.0];
            [
                Vertex {
                    position: start,
                    color,
                },
                Vertex {
                    position: end,
                    color,
                },
            ]
        })
        .collect()
}

fn encode_indices(indices: &[u32]) -> (Vec<u8>, IndexFormat) {
    let needs_u32 = indices.iter().any(|index| *index > u16::MAX as u32);
    if needs_u32 {
        (cast_vec(indices), IndexFormat::Uint32)
    } else {
        let packed: Vec<u16> = indices.iter().map(|index| *index as u16).collect();
        (cast_vec(&packed), IndexFormat::Uint16)
    }
}

struct DecodedTexture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn decode_texture_rgba(bytes: &[u8]) -> Result<DecodedTexture, String> {
    let image = image::load_from_memory(bytes).map_err(|err| err.to_string())?;
    let rgba = image.to_rgba8();
    Ok(DecodedTexture {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

fn sample_texture(texture: &DecodedTexture, uv: [f32; 2]) -> [f32; 4] {
    if texture.width == 0 || texture.height == 0 {
        return [1.0, 1.0, 1.0, 1.0];
    }

    let u = uv[0].clamp(0.0, 1.0);
    let v = 1.0 - uv[1].clamp(0.0, 1.0);
    let x = ((u * texture.width as f32) as u32).min(texture.width - 1);
    let y = ((v * texture.height as f32) as u32).min(texture.height - 1);
    let index = ((y * texture.width + x) * 4) as usize;
    let pixel = &texture.pixels[index..index + 4];
    [
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
        pixel[3] as f32 / 255.0,
    ]
}

fn cast_slice<T: bytemuck::Pod>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

fn cast_vec<T: bytemuck::Pod>(data: &[T]) -> Vec<u8> {
    bytemuck::cast_slice(data).to_vec()
}
