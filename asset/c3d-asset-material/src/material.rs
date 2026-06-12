use c3d_core::AssetId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::graph::MaterialGraphData;

/// Material asset error type.
#[derive(Debug, Error)]
pub enum MaterialAssetError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Result alias for material asset operations.
pub type MaterialAssetResult<T> = Result<T, MaterialAssetError>;

/// CPU-side material payload stored in AssetDB blobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialAssetData {
    /// Material blob schema version.
    pub version: u32,
    /// Linear RGBA base color factor.
    pub base_color: [f32; 4],
    /// Optional base color texture asset id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color_texture: Option<AssetId>,
    /// Optional node graph used to derive material parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<MaterialGraphData>,
}

impl Default for MaterialAssetData {
    fn default() -> Self {
        Self {
            version: 1,
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            graph: None,
        }
    }
}

impl MaterialAssetData {
    /// Resolve effective material parameters, evaluating an embedded graph when present.
    pub fn resolved(&self) -> MaterialAssetResult<Self> {
        if let Some(graph) = &self.graph {
            let mut resolved = graph
                .evaluate()
                .map_err(|err| MaterialAssetError::Serialization(err.to_string()))?;
            resolved.base_color_texture = self.base_color_texture;
            Ok(resolved)
        } else {
            Ok(self.clone())
        }
    }

    /// Serialize to JSON bytes for blob storage.
    pub fn to_bytes(&self) -> MaterialAssetResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|err| MaterialAssetError::Serialization(err.to_string()))
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> MaterialAssetResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|err| MaterialAssetError::Serialization(err.to_string()))
    }
}

/// Helper for reading and writing material assets.
#[derive(Debug, Clone, Default)]
pub struct MaterialAsset;

impl MaterialAsset {
    /// Decode material asset bytes.
    pub fn decode(bytes: &[u8]) -> MaterialAssetResult<MaterialAssetData> {
        MaterialAssetData::from_bytes(bytes)
    }

    /// Encode material asset bytes.
    pub fn encode(material: &MaterialAssetData) -> MaterialAssetResult<Vec<u8>> {
        material.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_round_trip() {
        let material = MaterialAssetData {
            version: 1,
            base_color: [0.8, 0.2, 0.1, 1.0],
            base_color_texture: Some(AssetId::new()),
            graph: Some(MaterialGraphData::from_base_color([0.8, 0.2, 0.1, 1.0])),
        };
        let bytes = MaterialAsset::encode(&material).expect("encode material");
        let restored = MaterialAsset::decode(&bytes).expect("decode material");
        assert_eq!(material, restored);
    }
}
