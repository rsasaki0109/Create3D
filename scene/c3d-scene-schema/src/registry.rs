use c3d_core::version::{C3D_SCENE_SCHEMA_CURRENT, C3D_SCENE_SCHEMA_MIN};

/// Version metadata for a component schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSchemaVersion {
    /// Stable schema version number.
    pub version: u32,
}

impl ComponentSchemaVersion {
    /// Create a schema version marker.
    pub const fn new(version: u32) -> Self {
        Self { version }
    }
}

/// Registry of component schema versions and migration hooks (stub for Month 2).
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    scene_schema_version: u32,
}

impl SchemaRegistry {
    /// Create a registry for the current scene schema.
    pub fn current() -> Self {
        Self {
            scene_schema_version: C3D_SCENE_SCHEMA_CURRENT,
        }
    }

    /// Returns the active scene schema version.
    pub fn scene_schema_version(&self) -> u32 {
        self.scene_schema_version
    }

    /// Validate that a serialized schema version can be loaded.
    pub fn validate_scene_schema(&self, version: u32) -> Result<(), SchemaError> {
        if !(C3D_SCENE_SCHEMA_MIN..=self.scene_schema_version).contains(&version) {
            return Err(SchemaError::UnsupportedSceneSchema {
                found: version,
                min: C3D_SCENE_SCHEMA_MIN,
                max: self.scene_schema_version,
            });
        }
        Ok(())
    }
}

/// Schema validation failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    /// Serialized scene schema is outside supported bounds.
    #[error("unsupported scene schema version {found} (supported {min}..={max})")]
    UnsupportedSceneSchema {
        /// Version found in serialized data.
        found: u32,
        /// Minimum supported version.
        min: u32,
        /// Maximum supported version.
        max: u32,
    },
}
