//! Basic PBR material assets for Create3D.

#![warn(missing_docs)]

mod graph;
mod material;

pub use graph::{MaterialGraphData, MaterialGraphError, MaterialGraphNode, MaterialGraphNodeKind};
pub use material::{MaterialAsset, MaterialAssetData};
