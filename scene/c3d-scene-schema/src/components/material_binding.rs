use c3d_core::AssetId;
use serde::{Deserialize, Serialize};

/// Material assignment for an entity (placeholder schema for Month 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialBinding {
    /// Material asset identifier.
    pub material_id: AssetId,
    /// Optional material slot name.
    pub slot: Option<String>,
}

impl MaterialBinding {
    /// Create a material binding.
    pub fn new(material_id: AssetId) -> Self {
        Self {
            material_id,
            slot: None,
        }
    }
}
