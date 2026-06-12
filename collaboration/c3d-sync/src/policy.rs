use c3d_scene_ops::SceneOperation;

/// Returns true when an operation can be synchronized in Month 11.
pub fn is_sync_supported(operation: &SceneOperation) -> bool {
    !matches!(
        operation,
        SceneOperation::CreateEntity { .. } | SceneOperation::DeleteEntity { .. }
    )
}

/// Filter a transaction down to sync-supported operations.
pub fn filter_syncable_transaction(
    transaction: &c3d_scene_ops::Transaction,
) -> Result<c3d_scene_ops::Transaction, SyncPolicyError> {
    let operations: Vec<_> = transaction
        .operations
        .iter()
        .filter(|operation| is_sync_supported(operation))
        .cloned()
        .collect();
    if operations.is_empty() {
        return Err(SyncPolicyError::EmptyAfterFilter);
    }
    if operations.len() != transaction.operations.len() {
        return Err(SyncPolicyError::UnsupportedOperations);
    }
    Ok(c3d_scene_ops::Transaction {
        id: transaction.id,
        operations,
        provenance: transaction.provenance.clone(),
    })
}

/// Sync policy failures for unsupported collaboration operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncPolicyError {
    /// Transaction contained create/delete operations blocked in Month 11.
    #[error("transaction contains unsupported collaboration operations")]
    UnsupportedOperations,
    /// No supported operations remained after filtering.
    #[error("transaction has no sync-supported operations")]
    EmptyAfterFilter,
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;
    use c3d_scene_schema::{Transform, TransformOp};

    #[test]
    fn transform_ops_are_supported() {
        assert!(is_sync_supported(&SceneOperation::TransformOp {
            entity_id: EntityId::new(),
            op: TransformOp::Translate(c3d_core::math::Vec3::X),
        }));
    }

    #[test]
    fn create_entity_is_blocked() {
        assert!(!is_sync_supported(&SceneOperation::CreateEntity {
            entity_id: EntityId::new(),
            parent: None,
            name: None,
            transform: Transform::IDENTITY,
            mesh_ref: None,
            material_binding: None,
            point_cloud_ref: None,
            gaussian_splat_ref: None,
            robot_root: None,
            robot_link: None,
            robot_joint: None,
        }));
    }
}
