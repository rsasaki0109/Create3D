use std::collections::HashMap;

use c3d_asset_db::AssetDb;
use c3d_asset_gsplat::{
    select_resident_chunks, GaussianSplatAsset, GaussianSplatChunkPayload, ResidencyConfig,
};
use c3d_core::math::{Mat4, Quat, Vec3};
use c3d_core::AssetId;
use c3d_ecs::RenderGaussianSplat;
use c3d_rhi::{BufferHandle, BufferInit, RhiBackend};
use c3d_rhi_wgpu::WgpuBackend;

use crate::mesh::Vertex;

/// GPU-resident Gaussian splat draw data for one resident chunk.
#[derive(Debug, Clone, Copy)]
pub struct CachedSplatDraw {
    /// Vertex buffer containing baked billboard quads.
    pub vertex_buffer: BufferHandle,
    /// Index buffer for quad triangles.
    pub index_buffer: BufferHandle,
    /// Number of indices to draw.
    pub index_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    asset_id: AssetId,
    chunk_id: u32,
    crop_hash: u64,
    opacity_scale_bits: u32,
    size_scale_bits: u32,
    stride: u32,
}

/// Cache that uploads resident Gaussian splat chunks to GPU buffers.
#[derive(Default)]
pub struct SplatGpuCache {
    entries: HashMap<CacheKey, CachedSplatDraw>,
}

impl SplatGpuCache {
    /// Drop all cached GPU uploads.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    /// Prepare resident chunks for a Gaussian splat drawable.
    pub fn prepare(
        &mut self,
        backend: &mut WgpuBackend,
        assets: &AssetDb,
        splat: RenderGaussianSplat,
        world: Mat4,
        view: Mat4,
        camera_position: Vec3,
    ) -> c3d_rhi::RhiResult<Vec<CachedSplatDraw>> {
        let metadata_bytes = assets
            .read_blob(splat.asset_id)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
        let metadata = GaussianSplatAsset::decode(&metadata_bytes)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;

        let local_camera = world.inverse().transform_point3(camera_position);
        let selected = select_resident_chunks(&metadata, local_camera, ResidencyConfig::default());
        let crop_hash = crop_hash(splat.crop_filter);

        let mut draws = Vec::with_capacity(selected.len());
        for selection in selected {
            let key = CacheKey {
                asset_id: splat.asset_id,
                chunk_id: selection.chunk.chunk_id,
                crop_hash,
                opacity_scale_bits: splat.opacity_scale.to_bits(),
                size_scale_bits: splat.size_scale.to_bits(),
                stride: selection.stride,
            };
            if let Some(entry) = self.entries.get(&key) {
                draws.push(*entry);
                continue;
            }

            let chunk_bytes = assets
                .read_blob(selection.chunk.blob_asset_id)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
            let mut payload = GaussianSplatChunkPayload::from_bytes(&chunk_bytes)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
            if let Some(crop) = splat.crop_filter {
                payload = payload.crop(&crop);
            }
            if payload.splat_count() == 0 {
                continue;
            }

            let (vertices, indices) = bake_chunk_quads(
                &payload,
                world,
                view,
                splat.opacity_scale,
                splat.size_scale,
                selection.stride,
            );
            if indices.is_empty() {
                continue;
            }

            let vertex_buffer = backend.create_buffer_init(BufferInit {
                label: "gsplat-vertices",
                contents: bytes_of_slice(&vertices),
                vertex: true,
                index: false,
                uniform: false,
            })?;
            let index_buffer = backend.create_buffer_init(BufferInit {
                label: "gsplat-indices",
                contents: bytes_of_slice(&indices),
                vertex: false,
                index: true,
                uniform: false,
            })?;
            let entry = CachedSplatDraw {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
            };
            self.entries.insert(key, entry);
            draws.push(entry);
        }

        let _ = metadata;
        Ok(draws)
    }
}

fn bake_chunk_quads(
    payload: &GaussianSplatChunkPayload,
    world: Mat4,
    view: Mat4,
    opacity_scale: f32,
    size_scale: f32,
    stride: u32,
) -> (Vec<Vertex>, Vec<u16>) {
    let stride = stride.max(1) as usize;
    let mut ranked: Vec<(f32, usize)> = payload
        .positions
        .iter()
        .enumerate()
        .step_by(stride)
        .map(|(index, position)| {
            let world_pos = world.transform_point3(Vec3::from_array(*position));
            let view_pos = view.transform_point3(world_pos);
            (view_pos.z, index)
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut vertices = Vec::with_capacity(ranked.len() * 4);
    let mut indices = Vec::with_capacity(ranked.len() * 6);
    for (_, index) in ranked {
        let position = Vec3::from_array(payload.positions[index]);
        let rotation = payload
            .rotations
            .get(index)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let scale = payload
            .scales
            .get(index)
            .copied()
            .unwrap_or([0.05, 0.05, 0.05]);
        let opacity = payload.opacities.get(index).copied().unwrap_or(1.0) * opacity_scale;
        let color = payload
            .colors
            .get(index)
            .copied()
            .unwrap_or([1.0, 1.0, 1.0]);
        let rgba = [color[0], color[1], color[2], opacity.clamp(0.0, 1.0)];
        let base = vertices.len() as u16;
        for corner in splat_corners(position, rotation, scale, size_scale) {
            vertices.push(Vertex {
                position: corner.to_array(),
                color: rgba,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

fn splat_corners(
    position: Vec3,
    rotation: [f32; 4],
    scale: [f32; 3],
    size_scale: f32,
) -> [Vec3; 4] {
    let quat = Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]);
    let half = Vec3::new(
        scale[0] * size_scale * 0.5,
        scale[1] * size_scale * 0.5,
        scale[2] * size_scale * 0.5,
    );
    let corners = [
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
    ];
    corners.map(|corner| {
        let scaled = Vec3::new(corner.x * half.x, corner.y * half.y, corner.z * half.z);
        position + quat * scaled
    })
}

fn crop_hash(crop: Option<c3d_scene_schema::PointCloudCropBox>) -> u64 {
    match crop {
        Some(value) => {
            let mut hash = 0u64;
            for component in value.min {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(component.to_bits() as u64);
            }
            for component in value.max {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(component.to_bits() as u64);
            }
            hash
        }
        None => 0,
    }
}

fn bytes_of_slice<T: bytemuck::Pod>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}
