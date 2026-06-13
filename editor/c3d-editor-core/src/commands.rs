/// Describes a command exposed to the palette and shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorCommand {
    /// Stable command identifier.
    pub id: &'static str,
    /// Human-readable label shown in the palette.
    pub label: &'static str,
    /// Optional keyboard shortcut hint.
    pub shortcut: Option<&'static str>,
}

/// Registry of built-in editor commands.
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    commands: Vec<EditorCommand>,
}

impl CommandRegistry {
    /// Create the default Month 5 command set.
    pub fn default_commands() -> Self {
        Self {
            commands: vec![
                EditorCommand {
                    id: "edit.undo",
                    label: "Undo",
                    shortcut: Some("Ctrl+Z"),
                },
                EditorCommand {
                    id: "edit.redo",
                    label: "Redo",
                    shortcut: Some("Ctrl+Y"),
                },
                EditorCommand {
                    id: "scene.open_project",
                    label: "Open Project",
                    shortcut: None,
                },
                EditorCommand {
                    id: "scene.export_glb",
                    label: "Export GLB Snapshot",
                    shortcut: None,
                },
                EditorCommand {
                    id: "project.save",
                    label: "Save Project",
                    shortcut: None,
                },
                EditorCommand {
                    id: "scene.import_glb",
                    label: "Import GLB",
                    shortcut: None,
                },
                EditorCommand {
                    id: "scene.import_ply",
                    label: "Import PLY Point Cloud",
                    shortcut: None,
                },
                EditorCommand {
                    id: "scene.import_gsplat",
                    label: "Import 3DGS PLY",
                    shortcut: None,
                },
                EditorCommand {
                    id: "scene.import_urdf",
                    label: "Import URDF",
                    shortcut: None,
                },
                EditorCommand {
                    id: "pointcloud.crop_derived",
                    label: "Crop Point Cloud To Derived Asset",
                    shortcut: None,
                },
                EditorCommand {
                    id: "gsplat.crop_derived",
                    label: "Crop Gaussian Splat To Derived Asset",
                    shortcut: None,
                },
                EditorCommand {
                    id: "mesh.create_cube",
                    label: "Create Cube",
                    shortcut: None,
                },
                EditorCommand {
                    id: "mesh.create_plane",
                    label: "Create Plane",
                    shortcut: None,
                },
                EditorCommand {
                    id: "asset.generate_thumbnail",
                    label: "Generate Mesh Thumbnail",
                    shortcut: None,
                },
                EditorCommand {
                    id: "view.focus_selection",
                    label: "Focus Selection",
                    shortcut: Some("F"),
                },
                EditorCommand {
                    id: "selection.clear",
                    label: "Clear Selection",
                    shortcut: Some("Esc"),
                },
            ],
        }
    }

    /// Iterate registered commands.
    pub fn commands(&self) -> &[EditorCommand] {
        &self.commands
    }

    /// Filter commands by case-insensitive substring match.
    pub fn search(&self, query: &str) -> Vec<EditorCommand> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.commands.clone();
        }
        self.commands
            .iter()
            .copied()
            .filter(|command| command.label.to_ascii_lowercase().contains(&query))
            .collect()
    }

    /// Lookup a command by id.
    pub fn find(&self, id: &str) -> Option<EditorCommand> {
        self.commands
            .iter()
            .copied()
            .find(|command| command.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_filters_commands() {
        let registry = CommandRegistry::default_commands();
        let results = registry.search("undo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "edit.undo");
    }
}
