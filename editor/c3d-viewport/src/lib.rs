//! Editor viewport camera, grid, and placeholder mesh rendering.

#![warn(missing_docs)]

mod camera;
mod gizmo;
mod mesh;
mod mesh_cache;
mod mode;
mod picking;
mod point_cloud_cache;
mod renderer;
mod shaders;

pub use camera::OrbitCamera;
pub use gizmo::{gizmo_drag_delta, pick_gizmo_axis, GizmoAxis, GizmoDragState};
pub use mesh_cache::MeshGpuCache;
pub use mode::ViewportShadingMode;
pub use picking::{pick_entity, PickHit};
pub use point_cloud_cache::{CachedPointCloudDraw, PointCloudGpuCache};
pub use renderer::ViewportRenderer;
