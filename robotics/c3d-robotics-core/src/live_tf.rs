use c3d_core::EntityId;
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::{apply_operations, SceneOperation};

use crate::protocol::TfTreeMessage;

/// Node in a live TF tree derived from sidecar snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTfFrameNode {
    /// TF frame name.
    pub frame_name: String,
    /// Child frames in hierarchy order.
    pub children: Vec<LiveTfFrameNode>,
}

/// Build a hierarchical TF tree for UI display from a flat edge list.
pub fn live_tf_tree_from_message(message: &TfTreeMessage) -> LiveTfFrameNode {
    let mut children_by_parent: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for edge in &message.edges {
        children_by_parent
            .entry(edge.parent.clone())
            .or_default()
            .push(edge.child.clone());
    }

    fn build_node(
        frame: &str,
        children_by_parent: &std::collections::HashMap<String, Vec<String>>,
    ) -> LiveTfFrameNode {
        let mut children: Vec<_> = children_by_parent
            .get(frame)
            .into_iter()
            .flat_map(|names| names.iter())
            .map(|child| build_node(child, children_by_parent))
            .collect();
        children.sort_by(|left, right| left.frame_name.cmp(&right.frame_name));
        LiveTfFrameNode {
            frame_name: frame.to_string(),
            children,
        }
    }

    build_node(&message.root_frame, &children_by_parent)
}

/// Apply live TF edges to robot link entities matched by child frame name.
pub fn apply_tf_tree(
    scene: &mut SceneDoc,
    message: &TfTreeMessage,
) -> Result<Vec<SceneOperation>, TfApplyError> {
    let mut operations = Vec::new();
    for edge in &message.edges {
        let Some(entity_id) = find_robot_link(scene, &edge.child) else {
            continue;
        };
        operations.push(SceneOperation::SetTransform {
            entity_id,
            transform: edge.transform,
        });
    }

    apply_operations(scene, &operations).map_err(TfApplyError::Scene)?;
    Ok(operations)
}

fn find_robot_link(scene: &SceneDoc, link_name: &str) -> Option<EntityId> {
    scene.entities().find_map(|entity| {
        entity
            .robot_link
            .as_ref()
            .filter(|link| link.link_name == link_name)
            .map(|_| entity.id)
    })
}

/// Failure while applying live TF snapshots.
#[derive(Debug, thiserror::Error)]
pub enum TfApplyError {
    /// Scene operation failure.
    #[error(transparent)]
    Scene(#[from] c3d_scene_doc::SceneError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{RobotLink, Transform};

    #[test]
    fn live_tf_tree_builds_hierarchy() {
        let message = TfTreeMessage {
            root_frame: "base_link".into(),
            edges: vec![
                crate::protocol::TfEdge {
                    parent: "base_link".into(),
                    child: "upper_arm".into(),
                    transform: Transform::IDENTITY,
                },
                crate::protocol::TfEdge {
                    parent: "upper_arm".into(),
                    child: "tool".into(),
                    transform: Transform::IDENTITY,
                },
            ],
        };

        let tree = live_tf_tree_from_message(&message);
        assert_eq!(tree.frame_name, "base_link");
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].frame_name, "upper_arm");
        assert_eq!(tree.children[0].children[0].frame_name, "tool");
    }

    #[test]
    fn apply_tf_tree_updates_matching_link() {
        let mut scene = SceneDoc::new();
        let link_id = EntityId::new();
        let mut link = Entity::new(link_id);
        link.robot_link = Some(RobotLink::new("upper_arm"));
        scene.insert_entity(link, None).expect("insert link");

        let message = TfTreeMessage {
            root_frame: "base_link".into(),
            edges: vec![crate::protocol::TfEdge {
                parent: "base_link".into(),
                child: "upper_arm".into(),
                transform: Transform {
                    translation: c3d_core::math::Vec3::new(0.0, 0.0, 1.0),
                    rotation: c3d_core::math::Quat::IDENTITY,
                    scale: c3d_core::math::Vec3::ONE,
                },
            }],
        };

        apply_tf_tree(&mut scene, &message).expect("apply tf");
        let updated = scene.get(link_id).expect("entity");
        assert!((updated.transform.translation.z - 1.0).abs() < 1e-6);
    }
}
