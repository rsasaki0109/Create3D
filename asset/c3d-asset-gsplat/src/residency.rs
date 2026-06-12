use c3d_core::math::Vec3;

use crate::{GaussianSplatAssetData, GaussianSplatChunkRecord};

/// Configuration for chunk residency selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResidencyConfig {
    /// Maximum number of chunks resident at once.
    pub max_resident_chunks: usize,
    /// Distance at which full-detail chunks are preferred.
    pub full_detail_distance: f32,
    /// Distance beyond which chunks are skipped entirely.
    pub cull_distance: f32,
}

impl Default for ResidencyConfig {
    fn default() -> Self {
        Self {
            max_resident_chunks: 8,
            full_detail_distance: 20.0,
            cull_distance: 200.0,
        }
    }
}

/// Selected chunk with LOD stride for GPU upload.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkSelection {
    /// Chunk metadata record.
    pub chunk: GaussianSplatChunkRecord,
    /// Effective splat stride for this frame.
    pub stride: u32,
}

/// Select resident chunks near the camera without requiring all GPU data at once.
pub fn select_resident_chunks(
    asset: &GaussianSplatAssetData,
    camera_position: Vec3,
    config: ResidencyConfig,
) -> Vec<ChunkSelection> {
    let mut ranked: Vec<(f32, ChunkSelection)> = asset
        .chunks
        .iter()
        .copied()
        .filter_map(|chunk| {
            let center = chunk_center(&chunk);
            let distance = (center - camera_position).length();
            if distance > config.cull_distance {
                return None;
            }
            let stride = if distance <= config.full_detail_distance {
                1
            } else {
                chunk.lod_stride.max(2)
            };
            Some((distance, ChunkSelection { chunk, stride }))
        })
        .collect();
    ranked.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked
        .into_iter()
        .take(config.max_resident_chunks)
        .map(|(_, selection)| selection)
        .collect()
}

fn chunk_center(chunk: &GaussianSplatChunkRecord) -> Vec3 {
    Vec3::new(
        (chunk.bounds_min[0] + chunk.bounds_max[0]) * 0.5,
        (chunk.bounds_min[1] + chunk.bounds_max[1]) * 0.5,
        (chunk.bounds_min[2] + chunk.bounds_max[2]) * 0.5,
    )
}

#[cfg(test)]
mod tests {
    use c3d_core::AssetId;

    use super::*;

    fn sample_asset(chunk_count: usize) -> GaussianSplatAssetData {
        let chunks = (0..chunk_count)
            .map(|index| {
                let offset = index as f32 * 10.0;
                GaussianSplatChunkRecord {
                    chunk_id: index as u32,
                    bounds_min: [offset, 0.0, 0.0],
                    bounds_max: [offset + 1.0, 1.0, 1.0],
                    splat_count: 100,
                    blob_asset_id: AssetId::new(),
                    lod_stride: 4,
                }
            })
            .collect();
        GaussianSplatAssetData {
            version: 1,
            splat_count: (chunk_count * 100) as u64,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [chunk_count as f32 * 10.0, 1.0, 1.0],
            sh_degree: 0,
            chunks,
        }
    }

    #[test]
    fn residency_limits_gpu_chunks() {
        let asset = sample_asset(32);
        let selected = select_resident_chunks(
            &asset,
            Vec3::ZERO,
            ResidencyConfig {
                max_resident_chunks: 4,
                ..ResidencyConfig::default()
            },
        );
        assert_eq!(selected.len(), 4);
    }
}
