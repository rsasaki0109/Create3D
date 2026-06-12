use c3d_core::EntityId;
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{joint_motion_transform, RobotJoint};

/// Joint position update mapped by joint name.
#[derive(Debug, Clone, PartialEq)]
pub struct JointStateUpdate {
    /// URDF joint name.
    pub joint_name: String,
    /// New joint position in radians or meters.
    pub position: f64,
}

/// Apply a single joint state update to the scene.
pub fn apply_joint_state(
    scene: &mut SceneDoc,
    update: &JointStateUpdate,
) -> Result<Vec<SceneOperation>, JointApplyError> {
    apply_joint_states(scene, std::slice::from_ref(update))
}

/// Apply multiple joint state updates to matching robot joint components.
pub fn apply_joint_states(
    scene: &mut SceneDoc,
    updates: &[JointStateUpdate],
) -> Result<Vec<SceneOperation>, JointApplyError> {
    let mut operations = Vec::new();

    for update in updates {
        let Some((entity_id, mut joint)) = find_joint(scene, &update.joint_name) else {
            continue;
        };
        joint.position = update.position;
        joint.validate_position().map_err(JointApplyError::Limit)?;

        let transform = joint_motion_transform(&joint);
        operations.push(SceneOperation::SetRobotJoint {
            entity_id,
            robot_joint: joint.clone(),
        });
        operations.push(SceneOperation::SetTransform {
            entity_id,
            transform,
        });
    }

    apply_operations(scene, &operations).map_err(JointApplyError::Scene)?;
    Ok(operations)
}

fn find_joint(scene: &SceneDoc, joint_name: &str) -> Option<(EntityId, RobotJoint)> {
    scene.entities().find_map(|entity| {
        entity
            .robot_joint
            .as_ref()
            .filter(|joint| joint.joint_name == joint_name)
            .map(|joint| (entity.id, joint.clone()))
    })
}

/// Failure while applying live joint states.
#[derive(Debug, thiserror::Error)]
pub enum JointApplyError {
    /// Scene operation failure.
    #[error(transparent)]
    Scene(#[from] c3d_scene_doc::SceneError),
    /// Joint limit validation failure.
    #[error(transparent)]
    Limit(#[from] c3d_scene_schema::RobotJointLimitError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{RobotJoint, RobotJointLimits, RobotJointType, RobotLink, Transform};

    #[test]
    fn apply_joint_state_updates_child_transform() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = Entity::new(entity_id);
        entity.robot_link = Some(RobotLink::new("upper_arm"));
        entity.robot_joint = Some(RobotJoint {
            joint_name: "shoulder".into(),
            joint_type: RobotJointType::Revolute,
            parent_link: "base_link".into(),
            child_link: "upper_arm".into(),
            axis: [0.0, 0.0, 1.0],
            origin: Transform::IDENTITY,
            limits: Some(RobotJointLimits {
                lower: -1.57,
                upper: 1.57,
                effort: 10.0,
                velocity: 1.0,
            }),
            position: 0.0,
            velocity: 0.0,
            ros_topic: None,
        });
        scene.insert_entity(entity, None).expect("insert entity");

        apply_joint_state(
            &mut scene,
            &JointStateUpdate {
                joint_name: "shoulder".into(),
                position: 0.5,
            },
        )
        .expect("apply joint state");

        let updated = scene.get(entity_id).expect("entity");
        let joint = updated.robot_joint.as_ref().expect("joint");
        assert!((joint.position - 0.5).abs() < 1e-6);
    }
}
