use c3d_core::AssetId;
use serde::{Deserialize, Serialize};

use crate::ContentHash;

/// Asset category stored in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    /// Triangle mesh geometry.
    Mesh,
    /// PBR material description.
    Material,
    /// Raw texture image bytes.
    Texture,
    /// Point cloud metadata blob.
    PointCloud,
    /// Point cloud chunk payload blob.
    PointCloudChunk,
    /// Gaussian splat metadata blob.
    GaussianSplat,
    /// Gaussian splat chunk payload blob.
    GaussianSplatChunk,
}

/// Metadata entry stored in the asset index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    /// Stable project-scoped asset identifier.
    pub id: AssetId,
    /// Asset category.
    pub kind: AssetKind,
    /// Content hash of the primary blob.
    pub content_hash: ContentHash,
    /// Human-readable asset name.
    pub name: String,
    /// MIME type when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Alias used by higher-level crates.
pub type AssetEntry = AssetRecord;

/// Serialized asset index file (`index.c3dassetdb`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIndexDocument {
    /// Index schema version.
    pub version: u32,
    /// Asset records in stable id order.
    pub assets: Vec<AssetRecord>,
}

impl Default for AssetIndexDocument {
    fn default() -> Self {
        Self {
            version: 1,
            assets: Vec::new(),
        }
    }
}
