//! Built-in component schemas.

mod annotation;
mod gaussian_splat_ref;
mod material_binding;
mod mesh_ref;
mod name;
mod point_cloud_ref;
mod robot_joint;
mod robot_link;
mod robot_root;
mod transform;

pub use annotation::AnnotationPlaceholder;

pub use gaussian_splat_ref::GaussianSplatRef;
pub use material_binding::MaterialBinding;
pub use mesh_ref::{MeshRef, TopologyMode};
pub use name::Name;
pub use point_cloud_ref::{PointCloudColorMode, PointCloudCropBox, PointCloudRef};
pub use robot_joint::{
    joint_motion_transform, validate_joint_position, RobotJoint, RobotJointLimitError,
    RobotJointLimits, RobotJointType,
};
pub use robot_link::RobotLink;
pub use robot_root::RobotRoot;
pub use transform::{Transform, TransformOp};
