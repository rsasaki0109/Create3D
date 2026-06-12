//! Scene component schemas for `Create3D`.

#![warn(missing_docs)]

pub mod components;
/// Component schema versioning and migration registry.
pub mod registry;

pub use components::{
    AnnotationPlaceholder, GaussianSplatRef, MaterialBinding, MeshRef, Name, PointCloudColorMode,
    PointCloudCropBox, PointCloudRef, TopologyMode, Transform, TransformOp,
};
pub use registry::{ComponentSchemaVersion, SchemaError, SchemaRegistry};
