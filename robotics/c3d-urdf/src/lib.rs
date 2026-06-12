//! URDF parsing and import planning.

#![warn(missing_docs)]

mod error;
mod model;
mod parse;
mod synthetic;

pub use error::{UrdfError, UrdfResult};
pub use model::{
    UrdfGeometry, UrdfImportPlan, UrdfJointSpec, UrdfLinkSpec, UrdfOrigin, UrdfVisualSpec,
};
pub use parse::{parse_urdf, parse_urdf_file};
pub use synthetic::preview_arm_urdf;
