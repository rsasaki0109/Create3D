use std::f64::consts::PI;

use crate::protocol::{
    BridgeEnvelope, BridgeMessage, JointStateMessage, TfEdge, TfTreeMessage, TopicInfo,
};

/// Mock ROS2 bridge that emits joint states and TF snapshots for tests and desktop demos.
#[derive(Debug, Clone)]
pub struct MockBridge {
    tick: u64,
    robot_name: String,
    joint_names: Vec<String>,
}

impl MockBridge {
    /// Create a mock bridge for the named robot and joints.
    pub fn new(robot_name: impl Into<String>, joint_names: Vec<String>) -> Self {
        Self {
            tick: 0,
            robot_name: robot_name.into(),
            joint_names,
        }
    }

    /// Advance the mock bridge and return the next IPC envelopes.
    pub fn next_envelopes(&mut self) -> Vec<BridgeEnvelope> {
        self.tick = self.tick.wrapping_add(1);
        let phase = self.tick as f64 * 0.05;
        let positions: Vec<f64> = self
            .joint_names
            .iter()
            .enumerate()
            .map(|(index, _)| (phase + index as f64 * 0.4).sin() * PI * 0.45)
            .collect();

        vec![
            BridgeEnvelope::new(BridgeMessage::TopicList {
                topics: vec![
                    TopicInfo {
                        name: "/joint_states".into(),
                        message_type: "sensor_msgs/msg/JointState".into(),
                    },
                    TopicInfo {
                        name: "/tf".into(),
                        message_type: "tf2_msgs/msg/TFMessage".into(),
                    },
                ],
            }),
            BridgeEnvelope::new(BridgeMessage::JointState(JointStateMessage {
                topic: "/joint_states".into(),
                joint_names: self.joint_names.clone(),
                positions: positions.clone(),
            })),
            BridgeEnvelope::new(BridgeMessage::TfTree(TfTreeMessage {
                root_frame: format!("{}_root", self.robot_name),
                edges: self
                    .joint_names
                    .iter()
                    .zip(positions)
                    .map(|(joint_name, position)| TfEdge {
                        parent: "base_link".into(),
                        child: joint_name.clone(),
                        transform: c3d_scene_schema::Transform {
                            translation: c3d_core::math::Vec3::new(0.0, 0.0, 0.1),
                            rotation: c3d_core::math::Quat::from_axis_angle(
                                c3d_core::math::Vec3::Z,
                                position as f32,
                            ),
                            scale: c3d_core::math::Vec3::ONE,
                        },
                    })
                    .collect(),
            })),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_bridge_emits_joint_state() {
        let mut bridge = MockBridge::new("preview_arm", vec!["shoulder".into()]);
        let envelopes = bridge.next_envelopes();
        assert!(envelopes
            .iter()
            .any(|envelope| matches!(envelope.message, BridgeMessage::JointState(_))));
    }
}
