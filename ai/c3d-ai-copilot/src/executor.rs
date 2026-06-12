use c3d_ai_context::ContextBuilder;
use c3d_ai_tool_protocol::{validate_tool_call, ToolCall, ToolPermission, ToolRegistry};
use c3d_core::{EntityId, UlidGenerator};
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::SceneOperation;
use c3d_scene_schema::{Name, Transform, TransformOp};

use crate::CopilotError;

/// JSON output from a read-only tool execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolExecutionResult {
    /// Structured tool output.
    pub output: serde_json::Value,
}

/// Executes validated tool calls against scene state.
#[derive(Debug, Default, Clone, Copy)]
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute a read-only tool call.
    pub fn execute_read(
        call: &ToolCall,
        scene: &SceneDoc,
        selection: &[EntityId],
    ) -> Result<ToolExecutionResult, CopilotError> {
        let registry = ToolRegistry::builtins();
        validate_tool_call(&registry, call, &[ToolPermission::SceneRead])?;

        let output = match call.tool.as_str() {
            "scene.list_entities" => {
                let entities: Vec<_> = scene
                    .entities()
                    .map(|entity| {
                        serde_json::json!({
                            "id": entity.id.to_string(),
                            "name": entity.name.as_ref().map(|name| name.value.clone()),
                        })
                    })
                    .collect();
                serde_json::json!({ "entities": entities })
            }
            "scene.inspect_selection" => serde_json::json!({
                "selection": selection
                    .iter()
                    .map(|entity_id| entity_id.to_string())
                    .collect::<Vec<_>>(),
            }),
            "scene.summarize" => {
                let pack = ContextBuilder::build(scene, selection, &registry);
                serde_json::json!({
                    "summary": pack.scene_summary,
                    "selection": pack.selection_summary,
                })
            }
            other => {
                return Err(CopilotError::Unsupported(format!(
                    "read tool not implemented: {other}"
                )));
            }
        };

        Ok(ToolExecutionResult { output })
    }

    /// Compile a write tool call into scene operations.
    pub fn compile_write(
        call: &ToolCall,
        selection: &[EntityId],
        ids: &mut UlidGenerator,
    ) -> Result<Vec<SceneOperation>, CopilotError> {
        let registry = ToolRegistry::builtins();
        validate_tool_call(
            &registry,
            call,
            &[ToolPermission::SceneRead, ToolPermission::SceneWrite],
        )?;

        match call.tool.as_str() {
            "scene.translate_selection" => {
                if selection.is_empty() {
                    return Err(CopilotError::Unsupported(
                        "Select at least one entity to translate.".into(),
                    ));
                }
                let x = call.arguments["x"].as_f64().unwrap_or(0.0) as f32;
                let y = call.arguments["y"].as_f64().unwrap_or(0.0) as f32;
                let z = call.arguments["z"].as_f64().unwrap_or(0.0) as f32;
                Ok(selection
                    .iter()
                    .map(|entity_id| SceneOperation::TransformOp {
                        entity_id: *entity_id,
                        op: TransformOp::Translate(c3d_core::math::Vec3::new(x, y, z)),
                    })
                    .collect())
            }
            "scene.set_entity_name" => {
                let entity_id = call.arguments["entity_id"]
                    .as_str()
                    .ok_or_else(|| CopilotError::InvalidInput("entity_id required".into()))
                    .and_then(|value| {
                        EntityId::parse(value)
                            .map_err(|err| CopilotError::InvalidInput(err.to_string()))
                    })?;
                let name = call.arguments["name"]
                    .as_str()
                    .ok_or_else(|| CopilotError::InvalidInput("name required".into()))?;
                Ok(vec![SceneOperation::SetName {
                    entity_id,
                    name: Name::new(name),
                }])
            }
            "scene.create_entity" => {
                let name = call.arguments["name"]
                    .as_str()
                    .ok_or_else(|| CopilotError::InvalidInput("name required".into()))?;
                Ok(vec![SceneOperation::CreateEntity {
                    entity_id: ids.next_entity_id(),
                    parent: None,
                    name: Some(Name::new(name)),
                    transform: Transform::IDENTITY,
                    mesh_ref: None,
                    material_binding: None,
                    point_cloud_ref: None,
                    gaussian_splat_ref: None,
                }])
            }
            other => Err(CopilotError::Unsupported(format!(
                "write tool not implemented: {other}"
            ))),
        }
    }
}
