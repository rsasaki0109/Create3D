use c3d_scene_schema::Transform;
use serde::{Deserialize, Serialize};

/// Sidecar bridge protocol version.
pub const BRIDGE_PROTOCOL_VERSION: u32 = 1;

/// Envelope for newline-delimited JSON IPC messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeEnvelope {
    /// Protocol version.
    pub version: u32,
    /// Message payload.
    pub message: BridgeMessage,
}

impl BridgeEnvelope {
    /// Wrap a message in the current protocol version.
    pub fn new(message: BridgeMessage) -> Self {
        Self {
            version: BRIDGE_PROTOCOL_VERSION,
            message,
        }
    }

    /// Serialize to a single JSON line.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse a JSON line into an envelope.
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

/// Messages exchanged between Create3D and a ROS2 sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeMessage {
    /// Client hello handshake.
    Hello {
        /// Client identifier.
        client_id: String,
    },
    /// Available topic list from the bridge.
    TopicList {
        /// Topics exposed by the bridge.
        topics: Vec<TopicInfo>,
    },
    /// Joint state update from `/joint_states` or mock bridge.
    JointState(JointStateMessage),
    /// TF tree snapshot.
    TfTree(TfTreeMessage),
}

/// Topic metadata exposed to the robotics panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicInfo {
    /// ROS topic name.
    pub name: String,
    /// Topic type string.
    pub message_type: String,
}

/// Joint state payload mapped onto scene joints by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointStateMessage {
    /// Source topic name.
    pub topic: String,
    /// Joint names in message order.
    pub joint_names: Vec<String>,
    /// Joint positions in message order.
    pub positions: Vec<f64>,
}

/// TF edge in a tree snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TfEdge {
    /// Parent frame name.
    pub parent: String,
    /// Child frame name.
    pub child: String,
    /// Transform from parent to child.
    pub transform: Transform,
}

/// TF tree snapshot for visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TfTreeMessage {
    /// Robot or TF root frame.
    pub root_frame: String,
    /// Edges in the TF tree.
    pub edges: Vec<TfEdge>,
}
