use c3d_core::AssetId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Point cloud asset error type.
#[derive(Debug, Error)]
pub enum PointCloudAssetError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Asset payload is invalid.
    #[error("invalid point cloud: {0}")]
    Invalid(String),
}

/// Result alias for point cloud asset operations.
pub type PointCloudAssetResult<T> = Result<T, PointCloudAssetError>;

/// Chunk index entry stored in the point cloud metadata blob.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointCloudChunkRecord {
    /// Stable chunk identifier within the asset.
    pub chunk_id: u32,
    /// Chunk bounds minimum corner.
    pub bounds_min: [f32; 3],
    /// Chunk bounds maximum corner.
    pub bounds_max: [f32; 3],
    /// Number of points stored in the chunk blob.
    pub point_count: u32,
    /// AssetDB blob id containing chunk payload bytes.
    pub blob_asset_id: AssetId,
    /// LOD stride used when the chunk is rendered at low detail.
    pub lod_stride: u32,
}

/// Point cloud metadata blob stored in AssetDB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCloudAssetData {
    /// Point cloud blob schema version.
    pub version: u32,
    /// Total number of points across all chunks.
    pub point_count: u64,
    /// Global bounds minimum corner.
    pub bounds_min: [f32; 3],
    /// Global bounds maximum corner.
    pub bounds_max: [f32; 3],
    /// Available point attributes in chunk payloads.
    pub has_rgb: bool,
    /// Whether intensity values are present.
    pub has_intensity: bool,
    /// Whether classification values are present.
    pub has_classification: bool,
    /// Spatial chunk index.
    pub chunks: Vec<PointCloudChunkRecord>,
}

impl PointCloudAssetData {
    /// Validate metadata and chunk index invariants.
    pub fn validate(&self) -> PointCloudAssetResult<()> {
        if self.chunks.is_empty() {
            return Err(PointCloudAssetError::Invalid(
                "point cloud has no chunks".into(),
            ));
        }
        if self.point_count == 0 {
            return Err(PointCloudAssetError::Invalid(
                "point cloud has zero points".into(),
            ));
        }
        for chunk in &self.chunks {
            if chunk.point_count == 0 {
                return Err(PointCloudAssetError::Invalid(format!(
                    "chunk {} is empty",
                    chunk.chunk_id
                )));
            }
            if chunk.lod_stride == 0 {
                return Err(PointCloudAssetError::Invalid(format!(
                    "chunk {} has invalid lod stride",
                    chunk.chunk_id
                )));
            }
        }
        Ok(())
    }

    /// Serialize to JSON bytes for blob storage.
    pub fn to_bytes(&self) -> PointCloudAssetResult<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|err| PointCloudAssetError::Serialization(err.to_string()))
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> PointCloudAssetResult<Self> {
        let asset: Self = serde_json::from_slice(bytes)
            .map_err(|err| PointCloudAssetError::Serialization(err.to_string()))?;
        asset.validate()?;
        Ok(asset)
    }
}

/// Helper for reading and writing point cloud assets.
#[derive(Debug, Clone, Default)]
pub struct PointCloudAsset;

impl PointCloudAsset {
    /// Decode point cloud metadata bytes.
    pub fn decode(bytes: &[u8]) -> PointCloudAssetResult<PointCloudAssetData> {
        PointCloudAssetData::from_bytes(bytes)
    }

    /// Encode point cloud metadata bytes.
    pub fn encode(asset: &PointCloudAssetData) -> PointCloudAssetResult<Vec<u8>> {
        asset.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trip() {
        let asset = PointCloudAssetData {
            version: 1,
            point_count: 3,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [1.0, 1.0, 1.0],
            has_rgb: true,
            has_intensity: false,
            has_classification: false,
            chunks: vec![PointCloudChunkRecord {
                chunk_id: 0,
                bounds_min: [0.0, 0.0, 0.0],
                bounds_max: [1.0, 1.0, 1.0],
                point_count: 3,
                blob_asset_id: AssetId::new(),
                lod_stride: 1,
            }],
        };
        let bytes = PointCloudAsset::encode(&asset).expect("encode");
        let restored = PointCloudAsset::decode(&bytes).expect("decode");
        assert_eq!(asset, restored);
    }
}
