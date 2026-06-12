use crate::{ToolCall, ToolProtocolError, ToolRegistry};

/// Validate a tool call against the registry and granted permissions.
pub fn validate_tool_call(
    registry: &ToolRegistry,
    call: &ToolCall,
    granted: &[crate::ToolPermission],
) -> Result<(), ToolProtocolError> {
    let definition = registry
        .get(&call.tool)
        .ok_or_else(|| ToolProtocolError::UnknownTool(call.tool.clone()))?;

    for permission in definition.permissions {
        if !granted.contains(permission) {
            return Err(ToolProtocolError::MissingPermission(*permission));
        }
    }

    validate_arguments(call)
}

fn validate_arguments(call: &ToolCall) -> Result<(), ToolProtocolError> {
    match call.tool.as_str() {
        "scene.translate_selection" => {
            let x = call.arguments.get("x").and_then(|value| value.as_f64());
            let y = call.arguments.get("y").and_then(|value| value.as_f64());
            let z = call.arguments.get("z").and_then(|value| value.as_f64());
            if x.is_none() || y.is_none() || z.is_none() {
                return Err(ToolProtocolError::InvalidArguments {
                    tool: call.tool.clone(),
                    message: "expected numeric x, y, z fields".into(),
                });
            }
        }
        "scene.set_entity_name" => {
            let entity_id = call
                .arguments
                .get("entity_id")
                .and_then(|value| value.as_str());
            let name = call.arguments.get("name").and_then(|value| value.as_str());
            if entity_id.is_none() || name.is_none() {
                return Err(ToolProtocolError::InvalidArguments {
                    tool: call.tool.clone(),
                    message: "expected entity_id and name strings".into(),
                });
            }
        }
        "scene.create_entity" => {
            let name = call.arguments.get("name").and_then(|value| value.as_str());
            if name.is_none() {
                return Err(ToolProtocolError::InvalidArguments {
                    tool: call.tool.clone(),
                    message: "expected name string".into(),
                });
            }
        }
        "scene.list_entities" | "scene.inspect_selection" | "scene.summarize" => {}
        other => {
            return Err(ToolProtocolError::UnknownTool(other.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolPermission, ToolRegistry};

    #[test]
    fn write_tool_requires_scene_write_permission() {
        let registry = ToolRegistry::builtins();
        let call = ToolCall::with_arguments(
            "scene.translate_selection",
            serde_json::json!({ "x": 0.0, "y": 1.0, "z": 0.0 }),
        );
        let err = validate_tool_call(&registry, &call, &[ToolPermission::SceneRead]).unwrap_err();
        assert!(matches!(
            err,
            ToolProtocolError::MissingPermission(ToolPermission::SceneWrite)
        ));
    }

    #[test]
    fn translate_arguments_must_include_xyz() {
        let registry = ToolRegistry::builtins();
        let call = ToolCall::with_arguments("scene.translate_selection", serde_json::json!({}));
        let err = validate_tool_call(
            &registry,
            &call,
            &[ToolPermission::SceneRead, ToolPermission::SceneWrite],
        )
        .unwrap_err();
        assert!(matches!(err, ToolProtocolError::InvalidArguments { .. }));
    }
}
