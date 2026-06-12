use c3d_scene_doc::{Entity, SceneDoc, SceneError, SceneResult};

use crate::SceneOperation;

/// Apply a scene operation and return its inverse operation.
pub fn apply_operation(
    scene: &mut SceneDoc,
    operation: &SceneOperation,
) -> SceneResult<SceneOperation> {
    match operation {
        SceneOperation::CreateEntity {
            entity_id,
            parent,
            name,
            transform,
            mesh_ref,
            material_binding,
            point_cloud_ref,
            gaussian_splat_ref,
        } => {
            let mut entity = Entity::new(*entity_id);
            entity.transform = *transform;
            entity.name = name.clone();
            entity.mesh_ref = mesh_ref.clone();
            entity.material_binding = material_binding.clone();
            entity.point_cloud_ref = point_cloud_ref.clone();
            entity.gaussian_splat_ref = gaussian_splat_ref.clone();
            scene.insert_entity(entity, *parent)?;
            let snapshot = scene.get(*entity_id).expect("entity inserted").snapshot();
            Ok(SceneOperation::DeleteEntity { snapshot })
        }
        SceneOperation::DeleteEntity { snapshot } => {
            let removed = scene.remove_entity(snapshot.id)?;
            Ok(SceneOperation::CreateEntity {
                entity_id: removed.id,
                parent: removed.parent,
                name: removed.name,
                transform: removed.transform,
                mesh_ref: removed.mesh_ref,
                material_binding: removed.material_binding,
                point_cloud_ref: removed.point_cloud_ref,
                gaussian_splat_ref: removed.gaussian_splat_ref,
            })
        }
        SceneOperation::SetTransform {
            entity_id,
            transform,
        } => {
            let before = scene.set_transform(*entity_id, *transform)?;
            Ok(SceneOperation::SetTransform {
                entity_id: *entity_id,
                transform: before,
            })
        }
        SceneOperation::TransformOp { entity_id, op } => {
            let current = scene
                .get(*entity_id)
                .ok_or(SceneError::EntityNotFound(*entity_id))?
                .transform;
            let mut next = current;
            op.apply_to(&mut next);
            let before = scene.set_transform(*entity_id, next)?;
            Ok(SceneOperation::SetTransform {
                entity_id: *entity_id,
                transform: before,
            })
        }
        SceneOperation::SetName { entity_id, name } => {
            let before = scene.set_name(*entity_id, name.clone())?;
            Ok(SceneOperation::RestoreName {
                entity_id: *entity_id,
                name: before,
            })
        }
        SceneOperation::RestoreName { entity_id, name } => {
            let before = scene.restore_name(*entity_id, name.clone())?;
            Ok(SceneOperation::RestoreName {
                entity_id: *entity_id,
                name: before,
            })
        }
        SceneOperation::SetMeshRef {
            entity_id,
            mesh_ref,
        } => {
            let before = scene.set_mesh_ref(*entity_id, mesh_ref.clone())?;
            Ok(SceneOperation::RestoreMeshRef {
                entity_id: *entity_id,
                mesh_ref: before,
            })
        }
        SceneOperation::RestoreMeshRef {
            entity_id,
            mesh_ref,
        } => {
            let before = scene.restore_mesh_ref(*entity_id, mesh_ref.clone())?;
            Ok(SceneOperation::RestoreMeshRef {
                entity_id: *entity_id,
                mesh_ref: before,
            })
        }
        SceneOperation::SetMaterialBinding {
            entity_id,
            material_binding,
        } => {
            let before = scene.set_material_binding(*entity_id, material_binding.clone())?;
            Ok(SceneOperation::RestoreMaterialBinding {
                entity_id: *entity_id,
                material_binding: before,
            })
        }
        SceneOperation::RestoreMaterialBinding {
            entity_id,
            material_binding,
        } => {
            let before = scene.restore_material_binding(*entity_id, material_binding.clone())?;
            Ok(SceneOperation::RestoreMaterialBinding {
                entity_id: *entity_id,
                material_binding: before,
            })
        }
        SceneOperation::SetPointCloudRef {
            entity_id,
            point_cloud_ref,
        } => {
            let before = scene.set_point_cloud_ref(*entity_id, point_cloud_ref.clone())?;
            Ok(SceneOperation::RestorePointCloudRef {
                entity_id: *entity_id,
                point_cloud_ref: before,
            })
        }
        SceneOperation::RestorePointCloudRef {
            entity_id,
            point_cloud_ref,
        } => {
            let before = scene.restore_point_cloud_ref(*entity_id, point_cloud_ref.clone())?;
            Ok(SceneOperation::RestorePointCloudRef {
                entity_id: *entity_id,
                point_cloud_ref: before,
            })
        }
        SceneOperation::SetGaussianSplatRef {
            entity_id,
            gaussian_splat_ref,
        } => {
            let before = scene.set_gaussian_splat_ref(*entity_id, gaussian_splat_ref.clone())?;
            Ok(SceneOperation::RestoreGaussianSplatRef {
                entity_id: *entity_id,
                gaussian_splat_ref: before,
            })
        }
        SceneOperation::RestoreGaussianSplatRef {
            entity_id,
            gaussian_splat_ref,
        } => {
            let before =
                scene.restore_gaussian_splat_ref(*entity_id, gaussian_splat_ref.clone())?;
            Ok(SceneOperation::RestoreGaussianSplatRef {
                entity_id: *entity_id,
                gaussian_splat_ref: before,
            })
        }
    }
}

/// Apply a list of operations and collect inverse operations in reverse order.
pub fn apply_operations(
    scene: &mut SceneDoc,
    operations: &[SceneOperation],
) -> SceneResult<Vec<SceneOperation>> {
    let mut inverse = Vec::with_capacity(operations.len());
    for operation in operations {
        inverse.push(apply_operation(scene, operation)?);
    }
    inverse.reverse();
    Ok(inverse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;
    use c3d_scene_schema::{Name, Transform};

    #[test]
    fn create_delete_are_inverse() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();

        let inverse = apply_operation(
            &mut scene,
            &SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new("Cube")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
            },
        )
        .expect("apply create");

        assert_eq!(scene.entity_count(), 1);
        apply_operation(&mut scene, &inverse).expect("apply inverse");
        assert_eq!(scene.entity_count(), 0);
    }
}
