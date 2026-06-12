use thiserror::Error;

use c3d_asset_db::AssetError;
use c3d_core::AssetId;
use c3d_import_gltf::ImportError;
use c3d_scene_doc::SceneError;

/// Project error type.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Manifest parse failure.
    #[error("manifest error: {0}")]
    Manifest(String),
    /// Asset database failure.
    #[error(transparent)]
    Asset(#[from] AssetError),
    /// Scene failure.
    #[error(transparent)]
    Scene(#[from] SceneError),
    /// Import failure.
    #[error(transparent)]
    Import(#[from] ImportError),
    /// PLY import failure.
    #[error(transparent)]
    PointCloudImport(#[from] c3d_import_ply::ImportError),
    /// Point cloud asset failure.
    #[error("point cloud asset error: {0}")]
    PointCloud(String),
    /// Mesh asset failure.
    #[error("mesh asset error: {0}")]
    Mesh(String),
    /// Material asset failure.
    #[error("material asset error: {0}")]
    Material(String),
    /// Project was not found.
    #[error("project not found at {0}")]
    NotFound(String),
}

/// Result alias for project operations.
pub type ProjectResult<T> = Result<T, ProjectError>;

/// Convenience helper for missing assets.
impl ProjectError {
    /// Build a missing asset error.
    pub fn missing_asset(asset_id: AssetId) -> Self {
        ProjectError::Asset(AssetError::NotFound(asset_id))
    }
}
