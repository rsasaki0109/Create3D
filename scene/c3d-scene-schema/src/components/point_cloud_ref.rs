use c3d_core::AssetId;
use serde::{Deserialize, Serialize};

/// Viewport attribute color mode for point clouds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
pub enum PointCloudColorMode {
    /// Use per-point RGB colors when available.
    #[default]
    Rgb,
    /// Map scalar intensity to a grayscale ramp.
    Intensity,
    /// Map classification ids to distinct colors.
    Classification,
}

impl PointCloudColorMode {
    /// Human-readable label for UI controls.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Intensity => "Intensity",
            Self::Classification => "Classification",
        }
    }

    /// Iterate supported color modes.
    pub fn all() -> [Self; 3] {
        [Self::Rgb, Self::Intensity, Self::Classification]
    }
}

/// Axis-aligned crop filter applied at render or import time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointCloudCropBox {
    /// Minimum corner in local space.
    pub min: [f32; 3],
    /// Maximum corner in local space.
    pub max: [f32; 3],
}

impl PointCloudCropBox {
    /// Returns true when the point lies inside the crop box.
    pub fn contains(&self, position: [f32; 3]) -> bool {
        position[0] >= self.min[0]
            && position[0] <= self.max[0]
            && position[1] >= self.min[1]
            && position[1] <= self.max[1]
            && position[2] >= self.min[2]
            && position[2] <= self.max[2]
    }
}

/// Reference to a point cloud asset attached to a scene entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCloudRef {
    /// Point cloud asset identifier.
    pub asset_id: AssetId,
    /// Viewport color mode for rendering.
    #[serde(default)]
    pub color_mode: PointCloudColorMode,
    /// Optional crop filter applied at render time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_filter: Option<PointCloudCropBox>,
}

impl PointCloudRef {
    /// Create a point cloud reference with default RGB coloring.
    pub fn new(asset_id: AssetId) -> Self {
        Self {
            asset_id,
            color_mode: PointCloudColorMode::default(),
            crop_filter: None,
        }
    }
}
