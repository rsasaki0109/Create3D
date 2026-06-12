use c3d_core::math::{Quat, Vec3};
use c3d_scene_schema::{RobotJoint, RobotJointLimits, RobotJointType, Transform};

use crate::error::UrdfError;

/// URDF origin expressed as xyz/rpy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UrdfOrigin {
    /// Translation xyz.
    pub xyz: [f64; 3],
    /// Roll-pitch-yaw in radians.
    pub rpy: [f64; 3],
}

impl Default for UrdfOrigin {
    fn default() -> Self {
        Self {
            xyz: [0.0, 0.0, 0.0],
            rpy: [0.0, 0.0, 0.0],
        }
    }
}

impl UrdfOrigin {
    /// Convert URDF origin to a scene transform.
    pub fn to_transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.xyz[0] as f32, self.xyz[1] as f32, self.xyz[2] as f32),
            rotation: Quat::from_euler(
                glam::EulerRot::XYZ,
                self.rpy[0] as f32,
                self.rpy[1] as f32,
                self.rpy[2] as f32,
            ),
            scale: Vec3::ONE,
        }
    }
}

/// Supported URDF geometry kinds for Month 10 import.
#[derive(Debug, Clone, PartialEq)]
pub enum UrdfGeometry {
    /// Axis-aligned box with full size extents.
    Box {
        /// Box size xyz.
        size: [f64; 3],
    },
    /// Cylinder aligned with local Z.
    Cylinder {
        /// Cylinder radius.
        radius: f64,
        /// Cylinder length.
        length: f64,
    },
    /// Sphere geometry.
    Sphere {
        /// Sphere radius.
        radius: f64,
    },
    /// External mesh reference.
    Mesh {
        /// Mesh filename from URDF.
        filename: String,
        /// Optional mesh scale.
        scale: [f64; 3],
    },
}

/// Visual geometry attached to a link.
#[derive(Debug, Clone, PartialEq)]
pub struct UrdfVisualSpec {
    /// Visual name or geometry label.
    pub name: String,
    /// Visual origin relative to the link frame.
    pub origin: UrdfOrigin,
    /// Geometry definition.
    pub geometry: UrdfGeometry,
    /// RGBA color when material color is present.
    pub color: [f32; 4],
}

/// Link import specification.
#[derive(Debug, Clone, PartialEq)]
pub struct UrdfLinkSpec {
    /// URDF link name.
    pub link_name: String,
    /// Visual geometries attached to the link.
    pub visuals: Vec<UrdfVisualSpec>,
}

/// Joint import specification.
#[derive(Debug, Clone, PartialEq)]
pub struct UrdfJointSpec {
    /// Joint metadata stored on the child link entity.
    pub joint: RobotJoint,
}

/// Parsed import plan used by the project importer.
#[derive(Debug, Clone, PartialEq)]
pub struct UrdfImportPlan {
    /// Robot name from the URDF root.
    pub robot_name: String,
    /// Root link name used as kinematic tree root.
    pub root_link: String,
    /// Link specifications keyed by link order in URDF.
    pub links: Vec<UrdfLinkSpec>,
    /// Joint specifications keyed by child link.
    pub joints: Vec<UrdfJointSpec>,
}

impl UrdfImportPlan {
    /// Find a link spec by name.
    pub fn link(&self, link_name: &str) -> Option<&UrdfLinkSpec> {
        self.links.iter().find(|link| link.link_name == link_name)
    }

    /// Find a joint spec whose child link matches `link_name`.
    pub fn joint_for_child(&self, link_name: &str) -> Option<&UrdfJointSpec> {
        self.joints
            .iter()
            .find(|joint| joint.joint.child_link == link_name)
    }
}

/// Parse a URDF joint type string.
pub fn parse_joint_type(value: &str) -> Result<RobotJointType, UrdfError> {
    match value {
        "fixed" => Ok(RobotJointType::Fixed),
        "revolute" => Ok(RobotJointType::Revolute),
        "continuous" => Ok(RobotJointType::Continuous),
        "prismatic" => Ok(RobotJointType::Prismatic),
        other => Err(UrdfError::Invalid(format!(
            "unsupported joint type `{other}`"
        ))),
    }
}

/// Build a [`RobotJoint`] from parsed URDF fields.
pub fn build_robot_joint(
    name: &str,
    joint_type: RobotJointType,
    parent: &str,
    child: &str,
    origin: UrdfOrigin,
    axis: [f64; 3],
    limits: Option<RobotJointLimits>,
) -> RobotJoint {
    RobotJoint {
        joint_name: name.to_string(),
        joint_type,
        parent_link: parent.to_string(),
        child_link: child.to_string(),
        axis,
        origin: origin.to_transform(),
        limits,
        position: 0.0,
        velocity: 0.0,
        ros_topic: Some("/joint_states".into()),
    }
}
