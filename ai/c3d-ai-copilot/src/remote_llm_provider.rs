use std::time::Duration;

use c3d_ai_context::ContextPack;
use c3d_ai_tool_protocol::ToolCall;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::error::CopilotError;
use crate::mock_provider::MockModelProvider;
use crate::proposal::build_proposal;
use crate::response::CopilotResponse;
use crate::ModelProvider;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// Configuration for an OpenAI-compatible chat completions endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLlmConfig {
    /// Bearer token for the remote API.
    pub api_key: String,
    /// API base URL, without trailing slash.
    pub base_url: String,
    /// Model identifier passed to the chat completions endpoint.
    pub model: String,
}

impl RemoteLlmConfig {
    /// Read configuration from environment variables.
    pub fn from_env() -> Option<Self> {
        Self::from_parts(
            std::env::var("CREATE3D_COPILOT_API_KEY").ok(),
            std::env::var("CREATE3D_COPILOT_BASE_URL").ok(),
            std::env::var("CREATE3D_COPILOT_MODEL").ok(),
        )
    }

    /// Build configuration from optional overrides, falling back to environment values.
    pub fn from_parts(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
    ) -> Option<Self> {
        let api_key = api_key
            .or_else(|| std::env::var("CREATE3D_COPILOT_API_KEY").ok())
            .filter(|value| !value.trim().is_empty())?;
        Some(Self {
            api_key,
            base_url: base_url
                .or_else(|| std::env::var("CREATE3D_COPILOT_BASE_URL").ok())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model
                .or_else(|| std::env::var("CREATE3D_COPILOT_MODEL").ok())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        })
    }
}

/// Remote model provider using OpenAI-compatible chat completions with tool calls.
#[derive(Debug)]
pub struct RemoteLlmProvider {
    config: Option<RemoteLlmConfig>,
    mock: MockModelProvider,
    client: Client,
}

impl RemoteLlmProvider {
    /// Create a provider that falls back to the mock model when configuration is absent.
    pub fn new(config: Option<RemoteLlmConfig>) -> Self {
        Self {
            config,
            mock: MockModelProvider,
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Read configuration from environment variables.
    pub fn from_env() -> Self {
        Self::new(RemoteLlmConfig::from_env())
    }

    /// Whether a remote API key is configured.
    pub fn is_remote_configured(&self) -> bool {
        self.config.is_some()
    }
}

impl Default for RemoteLlmProvider {
    fn default() -> Self {
        Self::from_env()
    }
}

impl ModelProvider for RemoteLlmProvider {
    fn complete(
        &self,
        prompt: &str,
        context: &ContextPack,
    ) -> Result<CopilotResponse, CopilotError> {
        let Some(config) = self.config.as_ref() else {
            return self.mock.complete(prompt, context);
        };

        let request = build_chat_request(prompt, context, &config.model);
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", config.api_key))
            .json(&request)
            .send()
            .map_err(|err| CopilotError::Remote(err.to_string()))?;
        let status = response.status();
        let body: Value = response
            .json()
            .map_err(|err| CopilotError::Remote(err.to_string()))?;
        if !status.is_success() {
            return Err(CopilotError::Remote(format_remote_error(
                status.as_u16(),
                &body,
            )));
        }

        parse_chat_response(prompt, context, &body, &config.model)
    }
}

fn build_chat_request(prompt: &str, context: &ContextPack, model: &str) -> Value {
    let selection = context
        .selection_summary
        .clone()
        .unwrap_or_else(|| "Nothing is selected.".into());
    let selected_ids: Vec<String> = context
        .selection
        .iter()
        .map(|entity_id| entity_id.to_string())
        .collect();
    let system_prompt = format!(
        "You are Create3D Copilot, a 3D scene editing assistant.\n\
         Scene context: {}\n\
         Selection: {}\n\
         Selected entity ids: {}\n\
         Answer read-only questions in plain text.\n\
         For scene edits, call exactly one write tool: scene.translate_selection, \
         scene.set_entity_name, or scene.create_entity.\n\
         When renaming, prefer the selected entity id if the user did not specify one.",
        context.scene_summary,
        selection,
        selected_ids.join(", ")
    );

    json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": prompt },
        ],
        "tools": write_tool_definitions(),
        "tool_choice": "auto",
    })
}

fn write_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "scene.translate_selection",
                "description": "Translate selected entities by a delta vector.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                        "z": { "type": "number" },
                    },
                    "required": ["x", "y", "z"],
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "scene.set_entity_name",
                "description": "Rename one entity.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "entity_id": { "type": "string" },
                        "name": { "type": "string" },
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "scene.create_entity",
                "description": "Create a named empty entity at the origin.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            }
        }),
    ]
}

fn parse_chat_response(
    prompt: &str,
    context: &ContextPack,
    body: &Value,
    model_id: &str,
) -> Result<CopilotResponse, CopilotError> {
    let message = body
        .pointer("/choices/0/message")
        .ok_or_else(|| CopilotError::Remote("chat response missing message".into()))?;

    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        if let Some(first) = tool_calls.first() {
            let call = parse_tool_call(first, context)?;
            if is_write_tool(&call.tool) {
                let summary = tool_call_summary(&call);
                return Ok(CopilotResponse::Proposal(build_proposal(
                    prompt, call, summary, context, model_id,
                )?));
            }
        }
    }

    let content = message
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| CopilotError::Remote("chat response missing content".into()))?;
    Ok(CopilotResponse::Answer(content))
}

fn parse_tool_call(value: &Value, context: &ContextPack) -> Result<ToolCall, CopilotError> {
    let function = value
        .get("function")
        .ok_or_else(|| CopilotError::Remote("tool call missing function payload".into()))?;
    let tool = function
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| CopilotError::Remote("tool call missing name".into()))?
        .to_string();
    let arguments = match function.get("arguments") {
        Some(Value::String(raw)) => serde_json::from_str(raw)
            .map_err(|err| CopilotError::Remote(format!("invalid tool arguments JSON: {err}")))?,
        Some(other) => other.clone(),
        None => json!({}),
    };
    let mut call = ToolCall::with_arguments(tool, arguments);
    normalize_write_call(&mut call, context)?;
    Ok(call)
}

fn normalize_write_call(call: &mut ToolCall, context: &ContextPack) -> Result<(), CopilotError> {
    if call.tool == "scene.set_entity_name" && call.arguments.get("entity_id").is_none() {
        if let Some(entity_id) = context.selection.first() {
            call.arguments["entity_id"] = json!(entity_id.to_string());
        }
    }
    Ok(())
}

fn is_write_tool(tool: &str) -> bool {
    matches!(
        tool,
        "scene.translate_selection" | "scene.set_entity_name" | "scene.create_entity"
    )
}

fn tool_call_summary(call: &ToolCall) -> String {
    match call.tool.as_str() {
        "scene.translate_selection" => {
            let x = call.arguments["x"].as_f64().unwrap_or(0.0);
            let y = call.arguments["y"].as_f64().unwrap_or(0.0);
            let z = call.arguments["z"].as_f64().unwrap_or(0.0);
            format!("Translate selection by ({x:.2}, {y:.2}, {z:.2}).")
        }
        "scene.set_entity_name" => {
            let name = call.arguments["name"]
                .as_str()
                .unwrap_or("Renamed")
                .to_string();
            format!("Rename entity to \"{name}\".")
        }
        "scene.create_entity" => {
            let name = call.arguments["name"]
                .as_str()
                .unwrap_or("New Entity")
                .to_string();
            format!("Create entity \"{name}\".")
        }
        other => format!("Apply tool {other}."),
    }
}

fn format_remote_error(status: u16, body: &Value) -> String {
    if let Some(message) = body.pointer("/error/message").and_then(Value::as_str) {
        format!("remote API returned {status}: {message}")
    } else {
        format!("remote API returned {status}: {body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::CopilotProposal;
    use c3d_ai_context::ContextBuilder;
    use c3d_ai_tool_protocol::ToolRegistry;
    use c3d_core::EntityId;
    use c3d_scene_doc::SceneDoc;
    use c3d_scene_schema::Name;

    #[test]
    fn config_from_parts_requires_api_key() {
        assert!(RemoteLlmConfig::from_parts(None, None, None).is_none());
        assert!(RemoteLlmConfig::from_parts(Some("key".into()), None, None).is_some());
    }

    #[test]
    fn parses_answer_from_chat_response() {
        let context = ContextBuilder::build(&SceneDoc::new(), &[], &ToolRegistry::builtins());
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "The scene has one entity."
                }
            }]
        });
        let response =
            parse_chat_response("how many entities?", &context, &body, "gpt-test").expect("answer");
        assert!(matches!(response, CopilotResponse::Answer(_)));
    }

    #[test]
    fn parses_write_proposal_from_tool_call() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = c3d_scene_doc::Entity::new(entity_id);
        entity.name = Some(Name::new("Hero"));
        scene.insert_entity(entity, None).expect("insert");

        let context = ContextBuilder::build(&scene, &[entity_id], &ToolRegistry::builtins());
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "scene.translate_selection",
                            "arguments": "{\"x\": 0, \"y\": 1.5, \"z\": 0}"
                        }
                    }]
                }
            }]
        });
        let response =
            parse_chat_response("move up 1.5", &context, &body, "gpt-test").expect("proposal");
        match response {
            CopilotResponse::Proposal(CopilotProposal { summary, .. }) => {
                assert!(summary.contains("1.50"));
            }
            CopilotResponse::Answer(answer) => panic!("expected proposal, got {answer}"),
        }
    }

    #[test]
    fn provider_without_key_uses_mock() {
        let provider = RemoteLlmProvider::new(None);
        let context = ContextBuilder::build(&SceneDoc::new(), &[], &ToolRegistry::builtins());
        let remote = provider
            .complete("how many entities?", &context)
            .expect("mock fallback");
        let mock = MockModelProvider
            .complete("how many entities?", &context)
            .expect("mock");
        assert_eq!(remote, mock);
    }
}
