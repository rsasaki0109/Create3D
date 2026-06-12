use c3d_scene_schema::PointCloudCropBox;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chunk payload serialization failure.
#[derive(Debug, Error)]
pub enum PointCloudChunkError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Result alias for chunk payload operations.
pub type PointCloudChunkResult<T> = Result<T, PointCloudChunkError>;

/// Structure-of-arrays chunk payload stored in AssetDB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCloudChunkPayload {
    /// Local-space positions for this chunk.
    pub positions: Vec<[f32; 3]>,
    /// Optional RGB colors in 0..255.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<[u8; 3]>,
    /// Optional scalar intensity values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intensity: Vec<f32>,
    /// Optional classification ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classification: Vec<u8>,
}

impl PointCloudChunkPayload {
    /// Returns the number of points in this chunk.
    pub fn point_count(&self) -> usize {
        self.positions.len()
    }

    /// Filter points by crop box and return a new payload.
    pub fn crop(&self, crop: &PointCloudCropBox) -> Self {
        let mut filtered = Self {
            positions: Vec::new(),
            colors: Vec::new(),
            intensity: Vec::new(),
            classification: Vec::new(),
        };
        for (index, position) in self.positions.iter().enumerate() {
            if !crop.contains(*position) {
                continue;
            }
            filtered.positions.push(*position);
            if let Some(color) = self.colors.get(index) {
                filtered.colors.push(*color);
            }
            if let Some(value) = self.intensity.get(index) {
                filtered.intensity.push(*value);
            }
            if let Some(value) = self.classification.get(index) {
                filtered.classification.push(*value);
            }
        }
        filtered
    }

    /// Serialize chunk payload to JSON bytes.
    pub fn to_bytes(&self) -> PointCloudChunkResult<Vec<u8>> {
        if self.positions.is_empty() {
            return Err(PointCloudChunkError::Serialization(
                "chunk has no positions".into(),
            ));
        }
        serde_json::to_vec(self).map_err(|err| PointCloudChunkError::Serialization(err.to_string()))
    }

    /// Deserialize chunk payload from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> PointCloudChunkResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|err| PointCloudChunkError::Serialization(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_filters_points() {
        let chunk = PointCloudChunkPayload {
            positions: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            colors: vec![[255, 0, 0], [0, 255, 0]],
            intensity: Vec::new(),
            classification: Vec::new(),
        };
        let cropped = chunk.crop(&PointCloudCropBox {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        });
        assert_eq!(cropped.point_count(), 1);
    }
}
