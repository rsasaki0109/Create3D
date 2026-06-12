//! ECS runtime wrapper and SceneDB projection for `Create3D`.

#![warn(missing_docs)]

mod components;
mod projection;

pub use components::{
    RenderGaussianSplat, RenderMaterial, RenderMeshKind, RenderPointCloud, SceneEntity,
    SceneTransform,
};
pub use projection::{
    project_scene_to_ecs, RuntimeWorld, SceneDrawable, SceneGaussianSplatDrawable,
    ScenePointCloudDrawable,
};
