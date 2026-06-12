use c3d_core::EntityId;
use c3d_scene_schema::{
    GaussianSplatRef, MaterialBinding, MeshRef, Name, PointCloudRef, RobotJoint, RobotLink,
    RobotRoot, Transform,
};
use serde::{Deserialize, Serialize};

/// Runtime entity stored in [`SceneDoc`](crate::SceneDoc).
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Stable entity identifier.
    pub id: EntityId,
    /// Optional parent entity.
    pub parent: Option<EntityId>,
    /// Ordered child entity identifiers.
    pub children: Vec<EntityId>,
    /// Optional display name.
    pub name: Option<Name>,
    /// Local transform.
    pub transform: Transform,
    /// Optional mesh reference placeholder.
    pub mesh_ref: Option<MeshRef>,
    /// Optional material binding placeholder.
    pub material_binding: Option<MaterialBinding>,
    /// Optional point cloud reference.
    pub point_cloud_ref: Option<PointCloudRef>,
    /// Optional Gaussian splat reference.
    pub gaussian_splat_ref: Option<GaussianSplatRef>,
    /// Optional robot root marker.
    pub robot_root: Option<RobotRoot>,
    /// Optional robot link metadata.
    pub robot_link: Option<RobotLink>,
    /// Optional robot joint metadata.
    pub robot_joint: Option<RobotJoint>,
}

impl Entity {
    /// Create a new entity with identity transform.
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            parent: None,
            children: Vec::new(),
            name: None,
            transform: Transform::IDENTITY,
            mesh_ref: None,
            material_binding: None,
            point_cloud_ref: None,
            gaussian_splat_ref: None,
            robot_root: None,
            robot_link: None,
            robot_joint: None,
        }
    }

    /// Snapshot this entity for undo/delete operations.
    pub fn snapshot(&self) -> EntitySnapshot {
        EntitySnapshot {
            id: self.id,
            parent: self.parent,
            name: self.name.clone(),
            transform: self.transform,
            mesh_ref: self.mesh_ref.clone(),
            material_binding: self.material_binding.clone(),
            point_cloud_ref: self.point_cloud_ref.clone(),
            gaussian_splat_ref: self.gaussian_splat_ref.clone(),
            robot_root: self.robot_root.clone(),
            robot_link: self.robot_link.clone(),
            robot_joint: self.robot_joint.clone(),
        }
    }
}

/// Serializable entity state used by delete/create undo operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    /// Stable entity identifier.
    pub id: EntityId,
    /// Parent at time of snapshot.
    pub parent: Option<EntityId>,
    /// Name component at time of snapshot.
    pub name: Option<Name>,
    /// Transform at time of snapshot.
    pub transform: Transform,
    /// Mesh reference at time of snapshot.
    pub mesh_ref: Option<MeshRef>,
    /// Material binding at time of snapshot.
    pub material_binding: Option<MaterialBinding>,
    /// Point cloud reference at time of snapshot.
    pub point_cloud_ref: Option<PointCloudRef>,
    /// Gaussian splat reference at time of snapshot.
    pub gaussian_splat_ref: Option<GaussianSplatRef>,
    /// Robot root marker at time of snapshot.
    pub robot_root: Option<RobotRoot>,
    /// Robot link metadata at time of snapshot.
    pub robot_link: Option<RobotLink>,
    /// Robot joint metadata at time of snapshot.
    pub robot_joint: Option<RobotJoint>,
}

impl EntitySnapshot {
    /// Reconstruct a runtime entity from a snapshot.
    pub fn into_entity(self) -> Entity {
        Entity {
            id: self.id,
            parent: self.parent,
            children: Vec::new(),
            name: self.name,
            transform: self.transform,
            mesh_ref: self.mesh_ref,
            material_binding: self.material_binding,
            point_cloud_ref: self.point_cloud_ref,
            gaussian_splat_ref: self.gaussian_splat_ref,
            robot_root: self.robot_root,
            robot_link: self.robot_link,
            robot_joint: self.robot_joint,
        }
    }
}
