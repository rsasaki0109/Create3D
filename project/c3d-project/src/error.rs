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
    /// 3DGS import failure.
    #[error(transparent)]
    GaussianSplatImport(#[from] c3d_import_gsplat::ImportError),
    /// Gaussian splat asset failure.
    #[error("gaussian splat asset error: {0}")]
    GaussianSplat(String),
    /// Mesh asset failure.
    #[error("mesh asset error: {0}")]
    Mesh(String),
    /// Material asset failure.
    #[error("material asset error: {0}")]
    Material(String),
    /// URDF parse failure.
    #[error(transparent)]
    Urdf(#[from] c3d_urdf::UrdfError),
    /// URDF import failure.
    #[error("urdf import error: {0}")]
    UrdfImport(String),
    /// Recovery snapshot failure.
    #[error("recovery error: {0}")]
    Recovery(String),
    /// Import failure with source path context.
    #[error("failed to import {kind} from `{path}`: {message}")]
    ImportAtPath {
        /// Import kind label.
        kind: &'static str,
        /// Source path.
        path: String,
        /// Underlying error message.
        message: String,
    },
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

    /// Wrap an import failure with source path context.
    pub fn import_at_path(
        kind: &'static str,
        path: impl AsRef<std::path::Path>,
        err: impl std::fmt::Display,
    ) -> Self {
        Self::ImportAtPath {
            kind,
            path: path.as_ref().display().to_string(),
            message: err.to_string(),
        }
    }
}
