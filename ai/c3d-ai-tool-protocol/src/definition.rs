use serde::{Deserialize, Serialize};

/// Permission required to invoke a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPermission {
    /// Read scene hierarchy and component summaries.
    SceneRead,
    /// Apply typed scene write operations.
    SceneWrite,
    /// Read asset metadata.
    AssetRead,
    /// Mutate asset blobs or manifests.
    AssetWrite,
}

/// Whether a tool mutates external state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolSideEffect {
    /// Read-only query.
    ReadOnly,
    /// Produces scene write operations requiring approval.
    SceneWrite,
}

/// Metadata describing one AI-callable tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    /// Stable tool identifier.
    pub name: &'static str,
    /// Human-readable description for model routing.
    pub description: &'static str,
    /// Permissions required to invoke the tool.
    pub permissions: &'static [ToolPermission],
    /// Side effect classification.
    pub side_effect: ToolSideEffect,
    /// Whether the tool supports dry-run preview.
    pub supports_preview: bool,
}

impl ToolDefinition {
    /// Returns true when the tool only reads scene state.
    pub fn is_read_only(&self) -> bool {
        matches!(self.side_effect, ToolSideEffect::ReadOnly)
    }
}
