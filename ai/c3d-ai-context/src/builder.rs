use c3d_ai_tool_protocol::ToolRegistry;
use c3d_core::EntityId;
use c3d_scene_doc::SceneDoc;

use crate::ContextPack;

/// Builds compact AI context from scene state.
#[derive(Debug, Default, Clone, Copy)]
pub struct ContextBuilder;

impl ContextBuilder {
    /// Build a context pack for the current scene and selection.
    pub fn build(scene: &SceneDoc, selection: &[EntityId], registry: &ToolRegistry) -> ContextPack {
        let scene_entity_count = scene.entity_count();
        let mut named = 0usize;
        let mut mesh_entities = 0usize;
        let mut point_cloud_entities = 0usize;
        let mut gaussian_splat_entities = 0usize;

        for entity in scene.entities() {
            if entity.name.is_some() {
                named += 1;
            }
            if entity.mesh_ref.is_some() {
                mesh_entities += 1;
            }
            if entity.point_cloud_ref.is_some() {
                point_cloud_entities += 1;
            }
            if entity.gaussian_splat_ref.is_some() {
                gaussian_splat_entities += 1;
            }
        }

        let scene_summary = format!(
            "Scene has {scene_entity_count} entities ({named} named, {mesh_entities} meshes, {point_cloud_entities} point clouds, {gaussian_splat_entities} gaussian splats)."
        );

        let selection_summary = if selection.is_empty() {
            None
        } else {
            Some(format_selection(scene, selection))
        };

        ContextPack {
            scene_entity_count,
            scene_summary,
            selection_summary,
            selection: selection.to_vec(),
            available_tools: registry.tool_names(),
        }
    }
}

fn format_selection(scene: &SceneDoc, selection: &[EntityId]) -> String {
    let mut parts = Vec::with_capacity(selection.len());
    for entity_id in selection {
        if let Some(entity) = scene.get(*entity_id) {
            let name = entity
                .name
                .as_ref()
                .map(|value| value.value.as_str())
                .unwrap_or("<unnamed>");
            let translation = entity.transform.translation;
            parts.push(format!(
                "{name} ({entity_id}) at ({:.2}, {:.2}, {:.2})",
                translation.x, translation.y, translation.z
            ));
        } else {
            parts.push(format!("missing entity {entity_id}"));
        }
    }
    format!(
        "Selected {} entity/entities: {}",
        selection.len(),
        parts.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_ai_tool_protocol::ToolRegistry;
    use c3d_scene_schema::Name;

    #[test]
    fn builds_scene_and_selection_summary() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = c3d_scene_doc::Entity::new(entity_id);
        entity.name = Some(Name::new("Hero"));
        scene.insert_entity(entity, None).expect("insert");

        let pack = ContextBuilder::build(&scene, &[entity_id], &ToolRegistry::builtins());
        assert_eq!(pack.scene_entity_count, 1);
        assert!(pack.scene_summary.contains("1 entities"));
        assert!(pack
            .selection_summary
            .as_deref()
            .is_some_and(|summary| summary.contains("Hero")));
    }
}
