use c3d_core::AssetId;
use serde::{Deserialize, Serialize};

/// Mesh topology interpretation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyMode {
    /// Triangle mesh.
    Triangles,
    /// Preserve authored polygons where possible.
    Polygons,
}

/// Reference to a mesh asset (placeholder schema for Month 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRef {
    /// Logical mesh asset identifier.
    pub asset_id: AssetId,
    /// Optional submesh name inside the asset.
    pub submesh: Option<String>,
    /// Topology interpretation mode.
    pub topology_mode: TopologyMode,
}

impl MeshRef {
    /// Create a mesh reference.
    pub fn new(asset_id: AssetId) -> Self {
        Self {
            asset_id,
            submesh: None,
            topology_mode: TopologyMode::Triangles,
        }
    }
}
