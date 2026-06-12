use serde::{Deserialize, Serialize};

/// Link metadata attached to a robot link entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotLink {
    /// URDF link name.
    pub link_name: String,
}

impl RobotLink {
    /// Create a robot link component.
    pub fn new(link_name: impl Into<String>) -> Self {
        Self {
            link_name: link_name.into(),
        }
    }
}
