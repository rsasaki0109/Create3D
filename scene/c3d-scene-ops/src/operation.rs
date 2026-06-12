use c3d_core::EntityId;
use c3d_scene_doc::EntitySnapshot;
use c3d_scene_schema::{
    GaussianSplatRef, MaterialBinding, MeshRef, Name, PointCloudRef, Transform, TransformOp,
};
use serde::{Deserialize, Serialize};

/// Typed scene mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SceneOperation {
    /// Create a new entity.
    CreateEntity {
        /// Entity identifier to create.
        entity_id: EntityId,
        /// Optional parent entity.
        parent: Option<EntityId>,
        /// Optional initial name.
        name: Option<Name>,
        /// Initial transform.
        transform: Transform,
        /// Optional mesh reference component.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mesh_ref: Option<MeshRef>,
        /// Optional material binding component.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        material_binding: Option<MaterialBinding>,
        /// Optional point cloud reference component.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        point_cloud_ref: Option<PointCloudRef>,
        /// Optional Gaussian splat reference component.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        gaussian_splat_ref: Option<GaussianSplatRef>,
    },
    /// Delete an existing entity.
    DeleteEntity {
        /// Snapshot used to restore the entity on undo.
        snapshot: EntitySnapshot,
    },
    /// Replace an entity transform.
    SetTransform {
        /// Target entity.
        entity_id: EntityId,
        /// Transform value to apply.
        transform: Transform,
    },
    /// Apply a typed transform edit operation.
    TransformOp {
        /// Target entity.
        entity_id: EntityId,
        /// Typed transform edit.
        op: TransformOp,
    },
    /// Set or replace the name component.
    SetName {
        /// Target entity.
        entity_id: EntityId,
        /// New name value.
        name: Name,
    },
    /// Restore a previous name value, or clear it when `None`.
    RestoreName {
        /// Target entity.
        entity_id: EntityId,
        /// Previous name value.
        name: Option<Name>,
    },
    /// Set mesh reference placeholder component.
    SetMeshRef {
        /// Target entity.
        entity_id: EntityId,
        /// Mesh reference value.
        mesh_ref: MeshRef,
    },
    /// Restore a previous mesh reference, or clear it when `None`.
    RestoreMeshRef {
        /// Target entity.
        entity_id: EntityId,
        /// Previous mesh reference value.
        mesh_ref: Option<MeshRef>,
    },
    /// Set material binding placeholder component.
    SetMaterialBinding {
        /// Target entity.
        entity_id: EntityId,
        /// Material binding value.
        material_binding: MaterialBinding,
    },
    /// Restore a previous material binding, or clear it when `None`.
    RestoreMaterialBinding {
        /// Target entity.
        entity_id: EntityId,
        /// Previous material binding value.
        material_binding: Option<MaterialBinding>,
    },
    /// Set point cloud reference placeholder component.
    SetPointCloudRef {
        /// Target entity.
        entity_id: EntityId,
        /// Point cloud reference value.
        point_cloud_ref: PointCloudRef,
    },
    /// Restore a previous point cloud reference, or clear it when `None`.
    RestorePointCloudRef {
        /// Target entity.
        entity_id: EntityId,
        /// Previous point cloud reference value.
        point_cloud_ref: Option<PointCloudRef>,
    },
    /// Set Gaussian splat reference placeholder component.
    SetGaussianSplatRef {
        /// Target entity.
        entity_id: EntityId,
        /// Gaussian splat reference value.
        gaussian_splat_ref: GaussianSplatRef,
    },
    /// Restore a previous Gaussian splat reference, or clear it when `None`.
    RestoreGaussianSplatRef {
        /// Target entity.
        entity_id: EntityId,
        /// Previous Gaussian splat reference value.
        gaussian_splat_ref: Option<GaussianSplatRef>,
    },
}
