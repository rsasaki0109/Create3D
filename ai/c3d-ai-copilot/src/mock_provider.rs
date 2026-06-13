use c3d_ai_context::ContextPack;
use c3d_ai_tool_protocol::ToolCall;

use crate::error::CopilotError;
use crate::proposal::build_proposal;
use crate::response::CopilotResponse;
use crate::ModelProvider;

/// Deterministic local provider used in tests and the Month 9 desktop prototype.
#[derive(Debug, Default, Clone, Copy)]
pub struct MockModelProvider;

impl ModelProvider for MockModelProvider {
    fn complete(
        &self,
        prompt: &str,
        context: &ContextPack,
    ) -> Result<CopilotResponse, CopilotError> {
        let prompt = prompt.trim();
        let lower = prompt.to_ascii_lowercase();

        if lower.contains("how many") || lower.contains("count") {
            return Ok(CopilotResponse::Answer(context.scene_summary.clone()));
        }

        if lower.contains("selected") || lower.contains("selection") {
            return Ok(CopilotResponse::Answer(
                context
                    .selection_summary
                    .clone()
                    .unwrap_or_else(|| "Nothing is selected.".into()),
            ));
        }

        if lower.contains("list") && lower.contains("entit") {
            return Ok(CopilotResponse::Answer(format!(
                "The scene currently contains {} entities.",
                context.scene_entity_count
            )));
        }

        if lower.contains("move") || lower.contains("translate") {
            let delta = parse_translation_delta(&lower);
            let call = ToolCall::with_arguments(
                "scene.translate_selection",
                serde_json::json!({ "x": delta.0, "y": delta.1, "z": delta.2 }),
            );
            return Ok(CopilotResponse::Proposal(build_proposal(
                prompt,
                call,
                format!(
                    "Translate selection by ({:.2}, {:.2}, {:.2}).",
                    delta.0, delta.1, delta.2
                ),
                context,
                "mock-local",
            )?));
        }

        if lower.starts_with("rename") || lower.contains("rename to") {
            let name = parse_rename_target(prompt).unwrap_or_else(|| "Renamed".into());
            let entity_id =
                context.selection.first().copied().ok_or_else(|| {
                    CopilotError::Unsupported("Select an entity to rename.".into())
                })?;
            let call = ToolCall::with_arguments(
                "scene.set_entity_name",
                serde_json::json!({ "entity_id": entity_id.to_string(), "name": name }),
            );
            return Ok(CopilotResponse::Proposal(build_proposal(
                prompt,
                call,
                format!("Rename selected entity to \"{name}\"."),
                context,
                "mock-local",
            )?));
        }

        if lower.contains("create") && lower.contains("entity") {
            let name = parse_create_name(prompt).unwrap_or_else(|| "New Entity".into());
            let call = ToolCall::with_arguments(
                "scene.create_entity",
                serde_json::json!({ "name": name }),
            );
            return Ok(CopilotResponse::Proposal(build_proposal(
                prompt,
                call,
                format!("Create entity \"{name}\"."),
                context,
                "mock-local",
            )?));
        }

        Ok(CopilotResponse::Answer(format!(
            "I can summarize the scene, describe the selection, translate selection (e.g. \"move up 1\"), rename selection (\"rename to Lamp\"), or create an entity (\"create entity Marker\"). Context: {}",
            context.scene_summary
        )))
    }
}

fn parse_translation_delta(prompt: &str) -> (f32, f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    if prompt.contains("up") {
        y += parse_amount(prompt).unwrap_or(1.0);
    }
    if prompt.contains("down") {
        y -= parse_amount(prompt).unwrap_or(1.0);
    }
    if prompt.contains("left") {
        x -= parse_amount(prompt).unwrap_or(1.0);
    }
    if prompt.contains("right") {
        x += parse_amount(prompt).unwrap_or(1.0);
    }
    if prompt.contains("forward") {
        z -= parse_amount(prompt).unwrap_or(1.0);
    }
    if prompt.contains("back") {
        z += parse_amount(prompt).unwrap_or(1.0);
    }

    if (x, y, z) == (0.0, 0.0, 0.0) {
        y = parse_amount(prompt).unwrap_or(1.0);
    }

    (x, y, z)
}

fn parse_amount(prompt: &str) -> Option<f32> {
    prompt
        .split_whitespace()
        .filter_map(|token| token.parse::<f32>().ok())
        .next()
}

fn parse_rename_target(prompt: &str) -> Option<String> {
    prompt
        .split("rename to")
        .nth(1)
        .or_else(|| prompt.split("rename").nth(1))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').trim_matches('\'').to_string())
}

fn parse_create_name(prompt: &str) -> Option<String> {
    prompt
        .split("create entity")
        .nth(1)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').trim_matches('\'').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_ai_context::ContextBuilder;
    use c3d_ai_tool_protocol::ToolRegistry;
    use c3d_core::EntityId;
    use c3d_scene_schema::Name;

    #[test]
    fn answers_scene_count_questions() {
        let mut scene = c3d_scene_doc::SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = c3d_scene_doc::Entity::new(entity_id);
        entity.name = Some(Name::new("Hero"));
        scene.insert_entity(entity, None).expect("insert");

        let context = ContextBuilder::build(&scene, &[], &ToolRegistry::builtins());
        let response = MockModelProvider
            .complete("how many entities?", &context)
            .expect("response");
        assert!(matches!(response, CopilotResponse::Answer(_)));
    }
}
