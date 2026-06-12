use std::collections::HashMap;

use c3d_asset_db::AssetDb;
use c3d_asset_pointcloud::{
    select_resident_chunks, PointCloudAsset, PointCloudAssetData, PointCloudChunkPayload,
    ResidencyConfig,
};
use c3d_core::math::Mat4;
use c3d_core::AssetId;
use c3d_ecs::RenderPointCloud;
use c3d_rhi::{BufferHandle, BufferInit, RhiBackend};
use c3d_rhi_wgpu::WgpuBackend;
use c3d_scene_schema::PointCloudColorMode;

use crate::mesh::Vertex;

/// GPU-resident point cloud chunk draw data.
#[derive(Debug, Clone, Copy)]
pub struct CachedPointCloudDraw {
    /// Vertex buffer containing baked point colors.
    pub vertex_buffer: BufferHandle,
    /// Number of points to draw.
    pub vertex_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    asset_id: AssetId,
    chunk_id: u32,
    color_mode: PointCloudColorMode,
    crop_hash: u64,
    stride: u32,
}

/// Cache that uploads resident point cloud chunks to GPU buffers.
#[derive(Default)]
pub struct PointCloudGpuCache {
    entries: HashMap<CacheKey, CachedPointCloudDraw>,
}

impl PointCloudGpuCache {
    /// Drop all cached GPU uploads.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    /// Prepare resident chunks for a point cloud drawable.
    pub fn prepare(
        &mut self,
        backend: &mut WgpuBackend,
        assets: &AssetDb,
        point_cloud: RenderPointCloud,
        world: Mat4,
        camera_position: c3d_core::math::Vec3,
    ) -> c3d_rhi::RhiResult<Vec<CachedPointCloudDraw>> {
        let metadata_bytes = assets
            .read_blob(point_cloud.asset_id)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
        let metadata = PointCloudAsset::decode(&metadata_bytes)
            .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;

        let local_camera = world.inverse().transform_point3(camera_position);
        let selected = select_resident_chunks(&metadata, local_camera, ResidencyConfig::default());
        let crop_hash = crop_hash(point_cloud.crop_filter);

        let mut draws = Vec::with_capacity(selected.len());
        for selection in selected {
            let key = CacheKey {
                asset_id: point_cloud.asset_id,
                chunk_id: selection.chunk.chunk_id,
                color_mode: point_cloud.color_mode,
                crop_hash,
                stride: selection.stride,
            };
            if let Some(entry) = self.entries.get(&key) {
                draws.push(*entry);
                continue;
            }

            let chunk_bytes = assets
                .read_blob(selection.chunk.blob_asset_id)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
            let mut payload = PointCloudChunkPayload::from_bytes(&chunk_bytes)
                .map_err(|err| c3d_rhi::RhiError::Initialization(err.to_string()))?;
            if let Some(crop) = point_cloud.crop_filter {
                payload = payload.crop(&crop);
            }
            if payload.point_count() == 0 {
                continue;
            }

            let vertices = bake_vertices(
                &payload,
                &metadata,
                point_cloud.color_mode,
                selection.stride,
            );
            let vertex_buffer = backend.create_buffer_init(BufferInit {
                label: "point-cloud-chunk",
                contents: bytes_of_slice(&vertices),
                vertex: true,
                index: false,
                uniform: false,
            })?;
            let entry = CachedPointCloudDraw {
                vertex_buffer,
                vertex_count: vertices.len() as u32,
            };
            self.entries.insert(key, entry);
            draws.push(entry);
        }

        Ok(draws)
    }
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

fn bake_vertices(
    payload: &PointCloudChunkPayload,
    metadata: &PointCloudAssetData,
    color_mode: PointCloudColorMode,
    stride: u32,
) -> Vec<Vertex> {
    let stride = stride.max(1) as usize;
    payload
        .positions
        .iter()
        .enumerate()
        .step_by(stride)
        .map(|(index, position)| Vertex {
            position: *position,
            color: point_color(index, payload, metadata, color_mode),
        })
        .collect()
}

fn point_color(
    index: usize,
    payload: &PointCloudChunkPayload,
    metadata: &PointCloudAssetData,
    color_mode: PointCloudColorMode,
) -> [f32; 4] {
    match color_mode {
        PointCloudColorMode::Rgb => {
            if metadata.has_rgb {
                if let Some(color) = payload.colors.get(index) {
                    return [
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                        1.0,
                    ];
                }
            }
            [1.0, 1.0, 1.0, 1.0]
        }
        PointCloudColorMode::Intensity => {
            let value = if metadata.has_intensity {
                payload.intensity.get(index).copied().unwrap_or(0.5)
            } else {
                0.5
            };
            let clamped = value.clamp(0.0, 1.0);
            [clamped, clamped, clamped, 1.0]
        }
        PointCloudColorMode::Classification => {
            let class = if metadata.has_classification {
                payload.classification.get(index).copied().unwrap_or(0)
            } else {
                0
            };
            let rgb = classification_color(class);
            [rgb[0], rgb[1], rgb[2], 1.0]
        }
    }
}

fn classification_color(class: u8) -> [f32; 3] {
    const PALETTE: [[f32; 3]; 8] = [
        [0.90, 0.25, 0.25],
        [0.25, 0.80, 0.35],
        [0.25, 0.45, 0.95],
        [0.95, 0.75, 0.20],
        [0.75, 0.30, 0.85],
        [0.20, 0.85, 0.85],
        [0.95, 0.55, 0.20],
        [0.55, 0.55, 0.55],
    ];
    PALETTE[(class as usize) % PALETTE.len()]
}

fn bytes_of_slice<T: bytemuck::Pod>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_palette_is_stable() {
        assert_eq!(classification_color(0), classification_color(8));
    }
}
