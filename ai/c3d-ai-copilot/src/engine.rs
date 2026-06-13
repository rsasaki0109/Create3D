use c3d_ai_context::{ContextBuilder, ContextPack};
use c3d_ai_tool_protocol::{ToolPermission, ToolRegistry};
use c3d_core::{EntityId, UlidGenerator};
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::TransactionManager;

use crate::error::CopilotError;
use crate::provider::ModelProvider;
use crate::response::{CopilotProposal, CopilotResponse};

/// Copilot orchestrator for ask, preview, and commit flows.
pub struct CopilotEngine {
    provider: Box<dyn ModelProvider>,
    registry: ToolRegistry,
    _granted_permissions: Vec<ToolPermission>,
}

impl CopilotEngine {
    /// Create a Copilot engine backed by the built-in mock provider.
    pub fn mock() -> Self {
        Self::new(Box::new(crate::MockModelProvider))
    }

    /// Create a Copilot engine using the remote LLM provider when configured.
    pub fn configured() -> Self {
        Self::new(Box::new(crate::RemoteLlmProvider::from_env()))
    }

    /// Create a Copilot engine with a custom model provider.
    pub fn new(provider: Box<dyn ModelProvider>) -> Self {
        Self {
            provider,
            registry: ToolRegistry::builtins(),
            _granted_permissions: vec![ToolPermission::SceneRead, ToolPermission::SceneWrite],
        }
    }

    /// Ask Copilot a question or request an edit proposal.
    pub fn ask(
        &self,
        prompt: &str,
        scene: &SceneDoc,
        selection: &[EntityId],
    ) -> Result<CopilotResponse, CopilotError> {
        let context = self.build_context(scene, selection);
        self.provider.complete(prompt, &context)
    }

    /// Build a preview manager by applying a proposal to a cloned scene.
    pub fn preview(
        proposal: &CopilotProposal,
        scene: &SceneDoc,
        ids: &mut UlidGenerator,
    ) -> Result<TransactionManager, CopilotError> {
        let transaction = proposal.clone().into_transaction(ids.next_transaction_id());
        let mut manager = TransactionManager::new(scene.clone());
        manager.apply(transaction)?;
        Ok(manager)
    }

    /// Borrow the tool registry exposed to agents.
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    fn build_context(&self, scene: &SceneDoc, selection: &[EntityId]) -> ContextPack {
        ContextBuilder::build(scene, selection, &self.registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;
    use c3d_scene_schema::Name;

    #[test]
    fn preview_applies_proposed_translation() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = c3d_scene_doc::Entity::new(entity_id);
        entity.name = Some(Name::new("Hero"));
        scene.insert_entity(entity, None).expect("insert");

        let engine = CopilotEngine::mock();
        let response = engine
            .ask("move up 1", &scene, &[entity_id])
            .expect("proposal");
        let proposal = match response {
            CopilotResponse::Proposal(proposal) => proposal,
            CopilotResponse::Answer(answer) => panic!("expected proposal, got {answer}"),
        };

        let mut ids = UlidGenerator::new();
        let preview = CopilotEngine::preview(&proposal, &scene, &mut ids).expect("preview");
        let translation = preview
            .scene()
            .get(entity_id)
            .expect("entity")
            .transform
            .translation;
        assert_eq!(translation.y, 1.0);
        assert_eq!(
            scene
                .get(entity_id)
                .expect("original")
                .transform
                .translation
                .y,
            0.0
        );
    }
}
