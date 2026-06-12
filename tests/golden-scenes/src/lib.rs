//! Golden scene replay harness for Create3D SceneDB.

use std::path::{Path, PathBuf};

use c3d_scene_doc::SceneDoc;
use c3d_scene_schema::SchemaRegistry;
use serde::{Deserialize, Serialize};

/// Metadata for a golden scene fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenSceneManifest {
    /// Human-readable scene name.
    pub name: String,
    /// Expected Create3D API version.
    pub api_version: u32,
    /// Expected product version when the fixture was captured.
    pub product_version: String,
    /// Relative path to serialized scene payload.
    pub scene_path: String,
}

impl GoldenSceneManifest {
    /// Load a manifest from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Load a golden scene fixture directory into a runtime scene document.
pub fn load_fixture(dir: impl AsRef<Path>) -> Result<SceneDoc, Box<dyn std::error::Error>> {
    let dir = dir.as_ref();
    let manifest = GoldenSceneManifest::load(dir.join("manifest.json"))?;
    let scene_path = dir.join(manifest.scene_path);
    let content = std::fs::read_to_string(scene_path)?;
    let registry = SchemaRegistry::current();
    Ok(SceneDoc::from_json_validated(&content, &registry)?)
}

/// Locate the golden scenes fixture directory.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::{C3D_API_VERSION, C3D_VERSION};

    #[test]
    fn sample_manifest_round_trip() {
        let manifest = GoldenSceneManifest {
            name: "empty-scene".to_string(),
            api_version: C3D_API_VERSION,
            product_version: C3D_VERSION.to_string(),
            scene_path: "empty.c3dscene.json".to_string(),
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
        let parsed: GoldenSceneManifest = serde_json::from_str(&json).expect("parse manifest");
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn fixtures_dir_exists() {
        assert!(fixtures_dir().is_dir());
    }

    #[test]
    fn empty_fixture_loads_and_round_trips() {
        let scene = load_fixture(fixtures_dir().join("empty")).expect("load empty fixture");
        assert_eq!(scene.entity_count(), 0);

        let json = scene.to_json().expect("serialize scene");
        let restored = SceneDoc::from_json(&json).expect("deserialize scene");
        assert_eq!(scene, restored);
    }

    #[test]
    fn gaussian_splat_entity_round_trips() {
        use c3d_core::EntityId;
        use c3d_scene_doc::Entity;
        use c3d_scene_schema::GaussianSplatRef;

        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = Entity::new(entity_id);
        entity.name = Some(c3d_scene_schema::Name::new("GoldenSplat"));
        entity.gaussian_splat_ref = Some(GaussianSplatRef::new(c3d_core::AssetId::new()));
        scene.insert_entity(entity, None).expect("insert entity");

        let json = scene.to_json().expect("serialize scene");
        let restored = SceneDoc::from_json(&json).expect("deserialize scene");
        assert_eq!(scene, restored);
    }
}
