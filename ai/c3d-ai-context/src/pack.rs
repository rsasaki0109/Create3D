use c3d_core::EntityId;
use serde::{Deserialize, Serialize};

/// Compact context delivered to model providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextPack {
    /// Number of entities in the scene.
    pub scene_entity_count: usize,
    /// Short natural-language scene summary.
    pub scene_summary: String,
    /// Optional summary of the current selection.
    pub selection_summary: Option<String>,
    /// Selected entity ids, if any.
    pub selection: Vec<EntityId>,
    /// Tool names the agent is allowed to call.
    pub available_tools: Vec<String>,
}
