use c3d_core::math::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::Transform;

/// Supported URDF joint types for Month 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotJointType {
    /// Fixed joint.
    Fixed,
    /// Revolute joint with limits.
    Revolute,
    /// Continuous revolute joint without limits.
    Continuous,
    /// Prismatic joint.
    Prismatic,
}

/// Joint limit metadata from URDF.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RobotJointLimits {
    /// Lower position bound in radians or meters.
    pub lower: f64,
    /// Upper position bound in radians or meters.
    pub upper: f64,
    /// Maximum effort.
    pub effort: f64,
    /// Maximum velocity.
    pub velocity: f64,
}

/// Joint metadata attached to a child link entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotJoint {
    /// URDF joint name.
    pub joint_name: String,
    /// Joint type.
    pub joint_type: RobotJointType,
    /// Parent link name.
    pub parent_link: String,
    /// Child link name.
    pub child_link: String,
    /// Joint axis in parent link frame.
    pub axis: [f64; 3],
    /// Fixed transform from parent link to joint frame.
    pub origin: Transform,
    /// Optional joint limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<RobotJointLimits>,
    /// Current joint position in radians or meters.
    #[serde(default)]
    pub position: f64,
    /// Current joint velocity.
    #[serde(default)]
    pub velocity: f64,
    /// Optional ROS topic binding for live updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ros_topic: Option<String>,
}

impl RobotJoint {
    /// Create a fixed joint component.
    pub fn fixed(
        joint_name: impl Into<String>,
        parent_link: impl Into<String>,
        child_link: impl Into<String>,
        origin: Transform,
    ) -> Self {
        Self {
            joint_name: joint_name.into(),
            joint_type: RobotJointType::Fixed,
            parent_link: parent_link.into(),
            child_link: child_link.into(),
            axis: [0.0, 0.0, 1.0],
            origin,
            limits: None,
            position: 0.0,
            velocity: 0.0,
            ros_topic: None,
        }
    }

    /// Validate that the current position is within joint limits.
    pub fn validate_position(&self) -> Result<(), RobotJointLimitError> {
        validate_joint_position(self)
    }

    /// Compute the child link transform from the joint origin and current position.
    pub fn motion_transform(&self) -> Transform {
        joint_motion_transform(self)
    }
}

/// Joint limit validation failure.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RobotJointLimitError {
    /// Position is outside configured limits.
    #[error("joint `{joint}` position {position} outside [{lower}, {upper}]")]
    OutOfRange {
        /// Joint name.
        joint: String,
        /// Observed position.
        position: f64,
        /// Lower limit.
        lower: f64,
        /// Upper limit.
        upper: f64,
    },
}

/// Validate a joint position against configured limits.
pub fn validate_joint_position(joint: &RobotJoint) -> Result<(), RobotJointLimitError> {
    let Some(limits) = joint.limits else {
        return Ok(());
    };
    if joint.position < limits.lower || joint.position > limits.upper {
        return Err(RobotJointLimitError::OutOfRange {
            joint: joint.joint_name.clone(),
            position: joint.position,
            lower: limits.lower,
            upper: limits.upper,
        });
    }
    Ok(())
}

/// Compute the child link transform from joint origin and current position.
pub fn joint_motion_transform(joint: &RobotJoint) -> Transform {
    match joint.joint_type {
        RobotJointType::Fixed => joint.origin,
        RobotJointType::Revolute | RobotJointType::Continuous => {
            let axis = normalized_axis(joint.axis);
            let rotation = Quat::from_axis_angle(axis, joint.position as f32);
            Transform {
                translation: joint.origin.translation,
                rotation: joint.origin.rotation * rotation,
                scale: joint.origin.scale,
            }
        }
        RobotJointType::Prismatic => {
            let axis = normalized_axis(joint.axis);
            Transform {
                translation: joint.origin.translation + axis * joint.position as f32,
                rotation: joint.origin.rotation,
                scale: joint.origin.scale,
            }
        }
    }
}

fn normalized_axis(axis: [f64; 3]) -> Vec3 {
    let axis = Vec3::new(axis[0] as f32, axis[1] as f32, axis[2] as f32);
    if axis.length_squared() > f32::EPSILON {
        axis.normalize()
    } else {
        Vec3::Z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revolute_motion_rotates_about_axis() {
        let joint = RobotJoint {
            joint_name: "j1".into(),
            joint_type: RobotJointType::Revolute,
            parent_link: "base".into(),
            child_link: "arm".into(),
            axis: [0.0, 0.0, 1.0],
            origin: Transform::IDENTITY,
            limits: Some(RobotJointLimits {
                lower: -2.0,
                upper: 2.0,
                effort: 1.0,
                velocity: 1.0,
            }),
            position: std::f64::consts::FRAC_PI_2,
            velocity: 0.0,
            ros_topic: None,
        };
        assert!(joint.validate_position().is_ok());
        let motion = joint.motion_transform();
        let (_, angle) = motion.rotation.to_axis_angle();
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn limit_validation_rejects_out_of_range() {
        let joint = RobotJoint {
            joint_name: "j1".into(),
            joint_type: RobotJointType::Revolute,
            parent_link: "base".into(),
            child_link: "arm".into(),
            axis: [0.0, 0.0, 1.0],
            origin: Transform::IDENTITY,
            limits: Some(RobotJointLimits {
                lower: -1.0,
                upper: 1.0,
                effort: 1.0,
                velocity: 1.0,
            }),
            position: 2.0,
            velocity: 0.0,
            ros_topic: None,
        };
        assert!(joint.validate_position().is_err());
    }
}
