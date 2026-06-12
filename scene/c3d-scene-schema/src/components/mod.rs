//! Built-in component schemas.

mod annotation;
mod gaussian_splat_ref;
mod material_binding;
mod mesh_ref;
mod name;
mod point_cloud_ref;
mod transform;

pub use annotation::AnnotationPlaceholder;

pub use gaussian_splat_ref::GaussianSplatRef;
pub use material_binding::MaterialBinding;
pub use mesh_ref::{MeshRef, TopologyMode};
pub use name::Name;
pub use point_cloud_ref::{PointCloudColorMode, PointCloudCropBox, PointCloudRef};
pub use transform::{Transform, TransformOp};
