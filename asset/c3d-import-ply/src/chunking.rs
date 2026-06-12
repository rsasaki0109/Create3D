use std::collections::HashMap;

use c3d_asset_pointcloud::PointCloudChunkPayload;

use crate::error::{ImportError, ImportResult};

/// Parsed point cloud before chunking.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedPointCloud {
    /// Positions in object space.
    pub positions: Vec<[f32; 3]>,
    /// Optional RGB colors.
    pub colors: Vec<[u8; 3]>,
    /// Optional scalar intensities.
    pub intensity: Vec<f32>,
    /// Optional classification ids.
    pub classification: Vec<u8>,
}

/// Split parsed points into spatial chunks for streaming residency.
pub fn chunk_point_cloud(
    parsed: ParsedPointCloud,
    target_points_per_chunk: usize,
) -> ImportResult<Vec<PointCloudChunkPayload>> {
    if parsed.positions.is_empty() {
        return Err(ImportError::Invalid("point cloud has no points".into()));
    }
    let target = target_points_per_chunk.max(256);
    let (bounds_min, bounds_max) = bounds(&parsed.positions);
    let volume = ((bounds_max[0] - bounds_min[0]).max(0.01)
        * (bounds_max[1] - bounds_min[1]).max(0.01)
        * (bounds_max[2] - bounds_min[2]).max(0.01))
    .max(0.01);
    let chunk_count = (parsed.positions.len() as f32 / target as f32)
        .ceil()
        .cbrt()
        .ceil()
        .max(1.0) as u32;
    let grid = chunk_count.max(1);
    let cell_size = [
        (bounds_max[0] - bounds_min[0]) / grid as f32,
        (bounds_max[1] - bounds_min[1]) / grid as f32,
        (bounds_max[2] - bounds_min[2]) / grid as f32,
    ];
    let _ = volume;

    let mut buckets: HashMap<(u32, u32, u32), PointCloudChunkPayload> = HashMap::new();
    for (index, position) in parsed.positions.iter().enumerate() {
        let cell = (
            cell_index(position[0], bounds_min[0], cell_size[0], grid),
            cell_index(position[1], bounds_min[1], cell_size[1], grid),
            cell_index(position[2], bounds_min[2], cell_size[2], grid),
        );
        let entry = buckets
            .entry(cell)
            .or_insert_with(|| PointCloudChunkPayload {
                positions: Vec::new(),
                colors: Vec::new(),
                intensity: Vec::new(),
                classification: Vec::new(),
            });
        entry.positions.push(*position);
        if let Some(color) = parsed.colors.get(index) {
            entry.colors.push(*color);
        }
        if let Some(value) = parsed.intensity.get(index) {
            entry.intensity.push(*value);
        }
        if let Some(value) = parsed.classification.get(index) {
            entry.classification.push(*value);
        }
    }

    Ok(buckets.into_values().collect())
}

fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (min, max)
}

fn cell_index(value: f32, min: f32, size: f32, grid: u32) -> u32 {
    if size <= f32::EPSILON {
        return 0;
    }
    (((value - min) / size).floor() as u32).min(grid - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_splits_large_cloud() {
        let parsed = ParsedPointCloud {
            positions: (0..10_000)
                .map(|index| {
                    [
                        (index % 100) as f32 * 0.1,
                        ((index / 100) % 100) as f32 * 0.1,
                        (index / 10_000) as f32,
                    ]
                })
                .collect(),
            colors: vec![[255, 255, 255]; 10_000],
            intensity: Vec::new(),
            classification: Vec::new(),
        };
        let chunks = chunk_point_cloud(parsed, 2_000).expect("chunk points");
        assert!(chunks.len() > 1);
        let total: usize = chunks.iter().map(|chunk| chunk.point_count()).sum();
        assert_eq!(total, 10_000);
    }
}
