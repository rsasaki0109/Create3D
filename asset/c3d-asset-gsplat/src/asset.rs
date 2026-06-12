use c3d_core::AssetId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Gaussian splat asset error type.
#[derive(Debug, Error)]
pub enum GaussianSplatAssetError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Asset payload is invalid.
    #[error("invalid gaussian splat asset: {0}")]
    Invalid(String),
}

/// Result alias for Gaussian splat asset operations.
pub type GaussianSplatAssetResult<T> = Result<T, GaussianSplatAssetError>;

/// Chunk index entry stored in the Gaussian splat metadata blob.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaussianSplatChunkRecord {
    /// Stable chunk identifier within the asset.
    pub chunk_id: u32,
    /// Chunk bounds minimum corner.
    pub bounds_min: [f32; 3],
    /// Chunk bounds maximum corner.
    pub bounds_max: [f32; 3],
    /// Number of splats stored in the chunk blob.
    pub splat_count: u32,
    /// AssetDB blob id containing chunk payload bytes.
    pub blob_asset_id: AssetId,
    /// LOD stride used when the chunk is rendered at low detail.
    pub lod_stride: u32,
}

/// Gaussian splat metadata blob stored in AssetDB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianSplatAssetData {
    /// Splat blob schema version.
    pub version: u32,
    /// Total number of splats across all chunks.
    pub splat_count: u64,
    /// Global bounds minimum corner.
    pub bounds_min: [f32; 3],
    /// Global bounds maximum corner.
    pub bounds_max: [f32; 3],
    /// Spherical harmonics degree stored in source data.
    pub sh_degree: u32,
    /// Spatial chunk index.
    pub chunks: Vec<GaussianSplatChunkRecord>,
}

impl GaussianSplatAssetData {
    /// Validate metadata and chunk index invariants.
    pub fn validate(&self) -> GaussianSplatAssetResult<()> {
        if self.chunks.is_empty() {
            return Err(GaussianSplatAssetError::Invalid(
                "gaussian splat asset has no chunks".into(),
            ));
        }
        if self.splat_count == 0 {
            return Err(GaussianSplatAssetError::Invalid(
                "gaussian splat asset has zero splats".into(),
            ));
        }
        for chunk in &self.chunks {
            if chunk.splat_count == 0 {
                return Err(GaussianSplatAssetError::Invalid(format!(
                    "chunk {} is empty",
                    chunk.chunk_id
                )));
            }
            if chunk.lod_stride == 0 {
                return Err(GaussianSplatAssetError::Invalid(format!(
                    "chunk {} has invalid lod stride",
                    chunk.chunk_id
                )));
            }
        }
        Ok(())
    }

    /// Serialize to JSON bytes for blob storage.
    pub fn to_bytes(&self) -> GaussianSplatAssetResult<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self)
            .map_err(|err| GaussianSplatAssetError::Serialization(err.to_string()))
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> GaussianSplatAssetResult<Self> {
        let asset: Self = serde_json::from_slice(bytes)
            .map_err(|err| GaussianSplatAssetError::Serialization(err.to_string()))?;
        asset.validate()?;
        Ok(asset)
    }
}

/// Helper for reading and writing Gaussian splat assets.
#[derive(Debug, Clone, Default)]
pub struct GaussianSplatAsset;

impl GaussianSplatAsset {
    /// Decode Gaussian splat metadata bytes.
    pub fn decode(bytes: &[u8]) -> GaussianSplatAssetResult<GaussianSplatAssetData> {
        GaussianSplatAssetData::from_bytes(bytes)
    }

    /// Encode Gaussian splat metadata bytes.
    pub fn encode(asset: &GaussianSplatAssetData) -> GaussianSplatAssetResult<Vec<u8>> {
        asset.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trip() {
        let asset = GaussianSplatAssetData {
            version: 1,
            splat_count: 2,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [1.0, 1.0, 1.0],
            sh_degree: 0,
            chunks: vec![GaussianSplatChunkRecord {
                chunk_id: 0,
                bounds_min: [0.0, 0.0, 0.0],
                bounds_max: [1.0, 1.0, 1.0],
                splat_count: 2,
                blob_asset_id: AssetId::new(),
                lod_stride: 1,
            }],
        };
        let bytes = GaussianSplatAsset::encode(&asset).expect("encode");
        let restored = GaussianSplatAsset::decode(&bytes).expect("decode");
        assert_eq!(asset, restored);
    }
}
