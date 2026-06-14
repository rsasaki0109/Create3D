//! Robotics bridge protocol, kinematics, and TF helpers.

#![warn(missing_docs)]

mod bridge_targets;
mod kinematics;
mod live_tf;
mod mock_bridge;
mod protocol;
mod sidecar_client;
mod tf_tree;

pub use bridge_targets::{primary_robot_bridge_target, robot_bridge_targets, RobotBridgeTarget};
pub use kinematics::{apply_joint_state, apply_joint_states, JointStateUpdate};
pub use live_tf::{apply_tf_tree, live_tf_tree_from_message, LiveTfFrameNode, TfApplyError};
pub use mock_bridge::MockBridge;
pub use protocol::{
    BridgeEnvelope, BridgeMessage, JointStateMessage, TfEdge, TfTreeMessage, TopicInfo,
    BRIDGE_PROTOCOL_VERSION,
};
pub use sidecar_client::{
    SidecarClient, SidecarClientConfig, SidecarClientError, DEFAULT_SIDECAR_ADDR,
};
pub use tf_tree::{robot_tf_trees, RobotTfTree, TfTreeNode};
