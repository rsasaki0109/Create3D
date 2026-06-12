use c3d_core::AssetId;
use serde::{Deserialize, Serialize};

use super::point_cloud_ref::PointCloudCropBox;

/// Reference to a Gaussian splat asset attached to a scene entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianSplatRef {
    /// Gaussian splat asset identifier.
    pub asset_id: AssetId,
    /// Global opacity multiplier applied at render time.
    #[serde(default = "default_opacity_scale")]
    pub opacity_scale: f32,
    /// Global size multiplier applied at render time.
    #[serde(default = "default_size_scale")]
    pub size_scale: f32,
    /// Optional crop filter applied at render time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_filter: Option<PointCloudCropBox>,
}

fn default_opacity_scale() -> f32 {
    1.0
}

fn default_size_scale() -> f32 {
    1.0
}

impl GaussianSplatRef {
    /// Create a Gaussian splat reference with default render scales.
    pub fn new(asset_id: AssetId) -> Self {
        Self {
            asset_id,
            opacity_scale: 1.0,
            size_scale: 1.0,
            crop_filter: None,
        }
    }
}
