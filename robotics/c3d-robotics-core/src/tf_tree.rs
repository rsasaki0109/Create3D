use c3d_core::EntityId;
use c3d_scene_doc::SceneDoc;

/// Node in a robot TF tree derived from scene hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfTreeNode {
    /// Scene entity id.
    pub entity_id: EntityId,
    /// Frame name shown in the TF panel.
    pub frame_name: String,
    /// Child frames in hierarchy order.
    pub children: Vec<TfTreeNode>,
}

/// TF tree for a robot root entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotTfTree {
    /// Robot root entity id.
    pub root_entity_id: EntityId,
    /// Robot name.
    pub robot_name: String,
    /// Root TF node.
    pub root: TfTreeNode,
}

/// Build TF trees for all robot roots in a scene.
pub fn robot_tf_trees(scene: &SceneDoc) -> Vec<RobotTfTree> {
    scene
        .entities()
        .filter_map(|entity| entity.robot_root.as_ref().map(|root| (entity.id, root)))
        .map(|(root_entity_id, robot_root)| RobotTfTree {
            robot_name: robot_root.robot_name.clone(),
            root: build_tf_node(scene, root_entity_id, robot_root.robot_name.clone()),
            root_entity_id,
        })
        .collect()
}

fn build_tf_node(scene: &SceneDoc, entity_id: EntityId, frame_name: String) -> TfTreeNode {
    let entity = scene.get(entity_id).expect("entity exists");
    let children = entity
        .children
        .iter()
        .map(|child_id| {
            let child = scene.get(*child_id).expect("child exists");
            let child_frame = child
                .robot_link
                .as_ref()
                .map(|link| link.link_name.clone())
                .or_else(|| child.name.as_ref().map(|name| name.value.clone()))
                .unwrap_or_else(|| format!("entity-{child_id}"));
            build_tf_node(scene, *child_id, child_frame)
        })
        .collect();

    TfTreeNode {
        entity_id,
        frame_name,
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{Name, RobotLink, RobotRoot};

    #[test]
    fn tf_tree_includes_link_children() {
        let mut scene = SceneDoc::new();
        let root_id = EntityId::new();
        let link_id = EntityId::new();

        let mut root = Entity::new(root_id);
        root.robot_root = Some(RobotRoot::new("preview_arm"));
        scene.insert_entity(root, None).expect("insert root");

        let mut link = Entity::new(link_id);
        link.robot_link = Some(RobotLink::new("base_link"));
        link.name = Some(Name::new("base_link"));
        scene
            .insert_entity(link, Some(root_id))
            .expect("insert link");

        let trees = robot_tf_trees(&scene);
        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].root.children.len(), 1);
        assert_eq!(trees[0].root.children[0].frame_name, "base_link");
    }
}
