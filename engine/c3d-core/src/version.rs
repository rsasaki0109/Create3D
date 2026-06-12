//! Version constants for `Create3D` binaries and serialized formats.

/// Human-readable product version.
pub const C3D_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable API version for serialized scene and tool schemas.
pub const C3D_API_VERSION: u32 = 1;

/// Minimum supported scene document schema version.
pub const C3D_SCENE_SCHEMA_MIN: u32 = 0;

/// Current scene document schema version.
pub const C3D_SCENE_SCHEMA_CURRENT: u32 = 0;
