use c3d_ai_context::ContextPack;

use crate::error::CopilotError;
use crate::mock_provider::MockModelProvider;
use crate::response::CopilotResponse;
use crate::ModelProvider;

/// Remote provider stub that accepts an API key and delegates to the mock model.
#[derive(Debug, Clone)]
pub struct RemoteStubProvider {
    api_key: Option<String>,
    mock: MockModelProvider,
}

impl RemoteStubProvider {
    /// Create a provider using an optional API key.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key: api_key.filter(|value| !value.trim().is_empty()),
            mock: MockModelProvider,
        }
    }

    /// Read `CREATE3D_COPILOT_API_KEY` when present.
    pub fn from_env() -> Self {
        Self::new(std::env::var("CREATE3D_COPILOT_API_KEY").ok())
    }
}

impl ModelProvider for RemoteStubProvider {
    fn complete(
        &self,
        prompt: &str,
        context: &ContextPack,
    ) -> Result<CopilotResponse, CopilotError> {
        let response = self.mock.complete(prompt, context)?;
        if self.api_key.is_some() {
            Ok(annotate_remote_stub(response))
        } else {
            Ok(response)
        }
    }
}

fn annotate_remote_stub(response: CopilotResponse) -> CopilotResponse {
    match response {
        CopilotResponse::Answer(answer) => CopilotResponse::Answer(format!(
            "[remote stub: API key configured, local mock execution]\n{answer}"
        )),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;
    use c3d_scene_doc::SceneDoc;

    #[test]
    fn remote_stub_annotates_answers_when_key_present() {
        let provider = RemoteStubProvider::new(Some("test-key".into()));
        let context = c3d_ai_context::ContextBuilder::build(
            &SceneDoc::new(),
            &[],
            &c3d_ai_tool_protocol::ToolRegistry::builtins(),
        );
        let response = provider
            .complete("how many entities?", &context)
            .expect("response");
        match response {
            CopilotResponse::Answer(answer) => {
                assert!(answer.contains("remote stub"));
            }
            CopilotResponse::Proposal(_) => panic!("expected answer"),
        }
    }

    #[test]
    fn remote_stub_without_key_matches_mock_output() {
        let provider = RemoteStubProvider::new(None);
        let mock = MockModelProvider;
        let context = c3d_ai_context::ContextBuilder::build(
            &SceneDoc::new(),
            &[EntityId::new()],
            &c3d_ai_tool_protocol::ToolRegistry::builtins(),
        );
        let stub = provider
            .complete("what is selected?", &context)
            .expect("stub");
        let mock = mock.complete("what is selected?", &context).expect("mock");
        assert_eq!(stub, mock);
    }
}
