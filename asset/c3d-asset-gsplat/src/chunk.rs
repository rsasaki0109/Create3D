use c3d_scene_schema::PointCloudCropBox;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chunk payload serialization failure.
#[derive(Debug, Error)]
pub enum GaussianSplatChunkError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Result alias for chunk payload operations.
pub type GaussianSplatChunkResult<T> = Result<T, GaussianSplatChunkError>;

/// Structure-of-arrays Gaussian splat chunk payload stored in AssetDB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianSplatChunkPayload {
    /// Local-space splat centers.
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

impl GaussianSplatChunkPayload {
    /// Returns the number of splats in this chunk.
    pub fn splat_count(&self) -> usize {
        self.positions.len()
    }

    /// Filter splats by crop box and return a new payload.
    pub fn crop(&self, crop: &PointCloudCropBox) -> Self {
        let mut filtered = Self {
            positions: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
            opacities: Vec::new(),
            colors: Vec::new(),
        };
        for (index, position) in self.positions.iter().enumerate() {
            if !crop.contains(*position) {
                continue;
            }
            filtered.positions.push(*position);
            if let Some(value) = self.rotations.get(index) {
                filtered.rotations.push(*value);
            }
            if let Some(value) = self.scales.get(index) {
                filtered.scales.push(*value);
            }
            if let Some(value) = self.opacities.get(index) {
                filtered.opacities.push(*value);
            }
            if let Some(value) = self.colors.get(index) {
                filtered.colors.push(*value);
            }
        }
        filtered
    }

    /// Serialize chunk payload to JSON bytes.
    pub fn to_bytes(&self) -> GaussianSplatChunkResult<Vec<u8>> {
        if self.positions.is_empty() {
            return Err(GaussianSplatChunkError::Serialization(
                "chunk has no positions".into(),
            ));
        }
        serde_json::to_vec(self)
            .map_err(|err| GaussianSplatChunkError::Serialization(err.to_string()))
    }

    /// Deserialize chunk payload from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> GaussianSplatChunkResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|err| GaussianSplatChunkError::Serialization(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_filters_splats() {
        let chunk = GaussianSplatChunkPayload {
            positions: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            rotations: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            scales: vec![[0.1, 0.1, 0.1], [0.1, 0.1, 0.1]],
            opacities: vec![1.0, 1.0],
            colors: vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        };
        let cropped = chunk.crop(&PointCloudCropBox {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        });
        assert_eq!(cropped.splat_count(), 1);
    }
}
