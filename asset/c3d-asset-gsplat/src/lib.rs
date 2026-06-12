//! Gaussian splat asset metadata, chunk payloads, and residency helpers.

#![warn(missing_docs)]

mod asset;
mod chunk;
mod residency;

pub use asset::{
    GaussianSplatAsset, GaussianSplatAssetData, GaussianSplatAssetError, GaussianSplatChunkRecord,
};
pub use chunk::{GaussianSplatChunkPayload, GaussianSplatChunkResult};
pub use residency::{select_resident_chunks, ChunkSelection, ResidencyConfig};
