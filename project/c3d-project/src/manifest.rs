use serde::{Deserialize, Serialize};

/// Project manifest stored at `manifest.c3d.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Manifest schema version.
    pub version: u32,
    /// Human-readable project name.
    pub name: String,
    /// Relative path to the main scene document.
    pub main_scene: String,
}

impl ProjectManifest {
    /// Create a new default manifest.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: 1,
            name: name.into(),
            main_scene: "scenes/main.c3dscene.json".into(),
        }
    }
}
