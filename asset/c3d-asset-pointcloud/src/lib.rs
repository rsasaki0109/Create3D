//! Point cloud asset metadata, chunk payloads, and residency helpers.

#![warn(missing_docs)]

mod asset;
mod chunk;
mod residency;

pub use asset::{
    PointCloudAsset, PointCloudAssetData, PointCloudAssetError, PointCloudChunkRecord,
};
pub use c3d_scene_schema::{PointCloudColorMode, PointCloudCropBox};
pub use chunk::{PointCloudChunkPayload, PointCloudChunkResult};
pub use residency::{select_resident_chunks, ChunkSelection, ResidencyConfig};

/// Alias kept for renderer code readability.
pub type PointColorMode = PointCloudColorMode;
