use c3d_scene_doc::SceneDoc;

use crate::tf_tree::robot_tf_trees;

/// Robot metadata used to configure mock or sidecar bridges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotBridgeTarget {
    /// Robot name from the scene root.
    pub robot_name: String,
    /// Joint names present in the scene.
    pub joint_names: Vec<String>,
}

/// Return bridge targets for all robot roots in a scene.
pub fn robot_bridge_targets(scene: &SceneDoc) -> Vec<RobotBridgeTarget> {
    robot_tf_trees(scene)
        .into_iter()
        .map(|tree| RobotBridgeTarget {
            robot_name: tree.robot_name,
            joint_names: scene
                .entities()
                .filter_map(|entity| {
                    entity
                        .robot_joint
                        .as_ref()
                        .map(|joint| joint.joint_name.clone())
                })
                .collect(),
        })
        .collect()
}

/// Return the first robot bridge target, if any.
pub fn primary_robot_bridge_target(scene: &SceneDoc) -> Option<RobotBridgeTarget> {
    robot_bridge_targets(scene).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{RobotJoint, RobotJointType, RobotLink, RobotRoot, Transform};

    #[test]
    fn primary_target_collects_joint_names() {
        let mut scene = SceneDoc::new();
        let root_id = EntityId::new();
        let joint_id = EntityId::new();

        let mut root = Entity::new(root_id);
        root.robot_root = Some(RobotRoot::new("preview_arm"));
        scene.insert_entity(root, None).expect("insert root");

        let mut joint_entity = Entity::new(joint_id);
        joint_entity.robot_link = Some(RobotLink::new("upper_arm"));
        joint_entity.robot_joint = Some(RobotJoint {
            joint_name: "shoulder".into(),
            joint_type: RobotJointType::Revolute,
            parent_link: "base_link".into(),
            child_link: "upper_arm".into(),
            axis: [0.0, 0.0, 1.0],
            origin: Transform::IDENTITY,
            limits: None,
            position: 0.0,
            velocity: 0.0,
            ros_topic: None,
        });
        scene
            .insert_entity(joint_entity, Some(root_id))
            .expect("insert joint");

        let target = primary_robot_bridge_target(&scene).expect("target");
        assert_eq!(target.robot_name, "preview_arm");
        assert_eq!(target.joint_names, vec!["shoulder".to_string()]);
    }
}
