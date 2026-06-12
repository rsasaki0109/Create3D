use serde::{Deserialize, Serialize};

/// Root marker for an imported robot model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotRoot {
    /// URDF or MJCF robot name.
    pub robot_name: String,
    /// Optional package root used to resolve relative mesh paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_path: Option<String>,
}

impl RobotRoot {
    /// Create a robot root component.
    pub fn new(robot_name: impl Into<String>) -> Self {
        Self {
            robot_name: robot_name.into(),
            package_path: None,
        }
    }
}
