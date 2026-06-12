//! Scene component schemas for `Create3D`.

#![warn(missing_docs)]

pub mod components;
/// Component schema versioning and migration registry.
pub mod registry;

pub use components::{
    joint_motion_transform, validate_joint_position, AnnotationPlaceholder, GaussianSplatRef,
    MaterialBinding, MeshRef, Name, PointCloudColorMode, PointCloudCropBox, PointCloudRef,
    RobotJoint, RobotJointLimitError, RobotJointLimits, RobotJointType, RobotLink, RobotRoot,
    TopologyMode, Transform, TransformOp,
};
pub use registry::{ComponentSchemaVersion, SchemaError, SchemaRegistry};
