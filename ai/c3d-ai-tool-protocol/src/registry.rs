use std::collections::HashMap;

use crate::{ToolDefinition, ToolPermission, ToolSideEffect};

/// Registry of AI-callable tools exposed to agents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register the Month 9 built-in tool set.
    pub fn builtins() -> Self {
        let mut registry = Self::new();
        for tool in builtin_tools() {
            registry.register(tool);
        }
        registry
    }

    /// Register a tool definition.
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.to_string(), tool);
    }

    /// Lookup a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Iterate registered tools in stable name order.
    pub fn tools(&self) -> impl Iterator<Item = &ToolDefinition> {
        let mut tools: Vec<_> = self.tools.values().collect();
        tools.sort_by_key(|tool| tool.name);
        tools.into_iter()
    }

    /// Return tool names in stable order.
    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }
}

fn builtin_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "scene.list_entities",
            description: "List entity ids and names in the scene.",
            permissions: &[ToolPermission::SceneRead],
            side_effect: ToolSideEffect::ReadOnly,
            supports_preview: false,
        },
        ToolDefinition {
            name: "scene.inspect_selection",
            description: "Summarize currently selected entities.",
            permissions: &[ToolPermission::SceneRead],
            side_effect: ToolSideEffect::ReadOnly,
            supports_preview: false,
        },
        ToolDefinition {
            name: "scene.summarize",
            description: "Return a compact scene overview.",
            permissions: &[ToolPermission::SceneRead],
            side_effect: ToolSideEffect::ReadOnly,
            supports_preview: false,
        },
        ToolDefinition {
            name: "scene.translate_selection",
            description: "Translate selected entities by a delta vector.",
            permissions: &[ToolPermission::SceneRead, ToolPermission::SceneWrite],
            side_effect: ToolSideEffect::SceneWrite,
            supports_preview: true,
        },
        ToolDefinition {
            name: "scene.set_entity_name",
            description: "Rename one entity.",
            permissions: &[ToolPermission::SceneRead, ToolPermission::SceneWrite],
            side_effect: ToolSideEffect::SceneWrite,
            supports_preview: true,
        },
        ToolDefinition {
            name: "scene.create_entity",
            description: "Create a named empty entity at the origin.",
            permissions: &[ToolPermission::SceneWrite],
            side_effect: ToolSideEffect::SceneWrite,
            supports_preview: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_register_expected_tools() {
        let registry = ToolRegistry::builtins();
        assert!(registry.get("scene.list_entities").is_some());
        assert!(registry.get("scene.translate_selection").is_some());
        assert_eq!(registry.tool_names().len(), 6);
    }
}
