use std::collections::HashMap;

use c3d_asset_gsplat::GaussianSplatChunkPayload;

use crate::error::{ImportError, ImportResult};

/// Parsed Gaussian splat cloud before chunking.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedGaussianSplatCloud {
    /// Splat centers in object space.
    pub positions: Vec<[f32; 3]>,
    /// Unit quaternions in xyzw order.
    pub rotations: Vec<[f32; 4]>,
    /// Linear scales after activation.
    pub scales: Vec<[f32; 3]>,
    /// Opacity values in 0..1 after activation.
    pub opacities: Vec<f32>,
    /// RGB colors baked from SH degree-0 coefficients.
    pub colors: Vec<[f32; 3]>,
}

/// Split parsed splats into spatial chunks for streaming residency.
pub fn chunk_gaussian_splats(
    parsed: ParsedGaussianSplatCloud,
    target_splats_per_chunk: usize,
) -> ImportResult<Vec<GaussianSplatChunkPayload>> {
    if parsed.positions.is_empty() {
        return Err(ImportError::Invalid(
            "gaussian splat cloud has no splats".into(),
        ));
    }
    let target = target_splats_per_chunk.max(256);
    let (bounds_min, bounds_max) = bounds(&parsed.positions);
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

    let mut buckets: HashMap<(u32, u32, u32), GaussianSplatChunkPayload> = HashMap::new();
    for (index, position) in parsed.positions.iter().enumerate() {
        let cell = (
            cell_index(position[0], bounds_min[0], cell_size[0], grid),
            cell_index(position[1], bounds_min[1], cell_size[1], grid),
            cell_index(position[2], bounds_min[2], cell_size[2], grid),
        );
        let entry = buckets
            .entry(cell)
            .or_insert_with(|| GaussianSplatChunkPayload {
                positions: Vec::new(),
                rotations: Vec::new(),
                scales: Vec::new(),
                opacities: Vec::new(),
                colors: Vec::new(),
            });
        entry.positions.push(*position);
        if let Some(value) = parsed.rotations.get(index) {
            entry.rotations.push(*value);
        }
        if let Some(value) = parsed.scales.get(index) {
            entry.scales.push(*value);
        }
        if let Some(value) = parsed.opacities.get(index) {
            entry.opacities.push(*value);
        }
        if let Some(value) = parsed.colors.get(index) {
            entry.colors.push(*value);
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
    (((value - min) / size).floor() as u32).min(grid.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_splits_large_cloud() {
        let parsed = ParsedGaussianSplatCloud {
            positions: (0..10_000)
                .map(|index| [index as f32 * 0.01, 0.0, 0.0])
                .collect(),
            rotations: vec![[0.0, 0.0, 0.0, 1.0]; 10_000],
            scales: vec![[0.05, 0.05, 0.05]; 10_000],
            opacities: vec![0.8; 10_000],
            colors: vec![[1.0, 1.0, 1.0]; 10_000],
        };
        let chunks = chunk_gaussian_splats(parsed, 2_048).expect("chunk");
        assert!(chunks.len() > 1);
    }
}
