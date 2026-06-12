use c3d_core::version::C3D_SCENE_SCHEMA_CURRENT;
use c3d_core::EntityId;
use c3d_scene_schema::{
    GaussianSplatRef, MaterialBinding, MeshRef, Name, PointCloudRef, Transform,
};
use serde::{Deserialize, Serialize};

use crate::entity::Entity;
use crate::error::{SceneError, SceneResult};
use crate::SceneDoc;

/// Serialized scene document format (`.c3dscene` JSON v0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    /// Scene schema version.
    pub schema_version: u32,
    /// Flat list of entity records.
    pub entities: Vec<SceneEntityRecord>,
}

/// Serialized entity record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneEntityRecord {
    /// Entity identifier.
    pub id: EntityId,
    /// Optional parent identifier.
    pub parent: Option<EntityId>,
    /// Optional name component.
    pub name: Option<Name>,
    /// Local transform component.
    pub transform: Transform,
    /// Optional mesh reference placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_ref: Option<MeshRef>,
    /// Optional material binding placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_binding: Option<MaterialBinding>,
    /// Optional point cloud reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_cloud_ref: Option<PointCloudRef>,
    /// Optional Gaussian splat reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gaussian_splat_ref: Option<GaussianSplatRef>,
}

impl SceneDocument {
    /// Build a serializable document from a runtime scene.
    pub fn from_scene(scene: &SceneDoc) -> Self {
        let mut entities: Vec<_> = scene
            .entities()
            .map(|entity| SceneEntityRecord {
                id: entity.id,
                parent: entity.parent,
                name: entity.name.clone(),
                transform: entity.transform,
                mesh_ref: entity.mesh_ref.clone(),
                material_binding: entity.material_binding.clone(),
                point_cloud_ref: entity.point_cloud_ref.clone(),
                gaussian_splat_ref: entity.gaussian_splat_ref.clone(),
            })
            .collect();
        entities.sort_by_key(|entity| entity.id);

        Self {
            schema_version: scene.schema_version(),
            entities,
        }
    }

    /// Convert this document into a runtime scene.
    pub fn into_scene(self) -> SceneResult<SceneDoc> {
        let mut scene = SceneDoc::new();
        scene.schema_version = self.schema_version;

        for record in &self.entities {
            if scene.contains(record.id) {
                return Err(SceneError::EntityAlreadyExists(record.id));
            }

            scene.entities.insert(
                record.id,
                Entity {
                    id: record.id,
                    parent: None,
                    children: Vec::new(),
                    name: record.name.clone(),
                    transform: record.transform,
                    mesh_ref: record.mesh_ref.clone(),
                    material_binding: record.material_binding.clone(),
                    point_cloud_ref: record.point_cloud_ref.clone(),
                    gaussian_splat_ref: record.gaussian_splat_ref.clone(),
                },
            );
        }

        for record in self.entities {
            let Some(parent_id) = record.parent else {
                continue;
            };

            if !scene.contains(parent_id) {
                return Err(SceneError::ParentNotFound(parent_id));
            }

            scene
                .entities
                .get_mut(&record.id)
                .expect("entity inserted above")
                .parent = Some(parent_id);

            scene
                .entities
                .get_mut(&parent_id)
                .expect("parent inserted above")
                .children
                .push(record.id);
        }

        Ok(scene)
    }
}

impl Default for SceneDocument {
    fn default() -> Self {
        Self {
            schema_version: C3D_SCENE_SCHEMA_CURRENT,
            entities: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;

    #[test]
    fn empty_document_round_trip() {
        let scene = SceneDoc::new();
        let json = scene.to_json().expect("serialize scene");
        let restored = SceneDoc::from_json(&json).expect("deserialize scene");
        assert_eq!(scene, restored);
    }

    #[test]
    fn parent_child_round_trip() {
        let mut scene = SceneDoc::new();
        let parent_id = EntityId::new();
        let child_id = EntityId::new();

        scene
            .insert_entity(Entity::new(parent_id), None)
            .expect("insert parent");
        scene
            .insert_entity(Entity::new(child_id), Some(parent_id))
            .expect("insert child");

        let json = scene.to_json().expect("serialize scene");
        let restored = SceneDoc::from_json(&json).expect("deserialize scene");
        assert_eq!(scene, restored);
    }

    #[test]
    fn gaussian_splat_ref_round_trip() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = Entity::new(entity_id);
        entity.name = Some(c3d_scene_schema::Name::new("Splat"));
        entity.gaussian_splat_ref = Some(GaussianSplatRef {
            asset_id: c3d_core::AssetId::new(),
            opacity_scale: 0.75,
            size_scale: 1.25,
            crop_filter: Some(c3d_scene_schema::PointCloudCropBox {
                min: [-1.0, 0.0, -1.0],
                max: [1.0, 2.0, 1.0],
            }),
        });
        scene.insert_entity(entity, None).expect("insert entity");

        let json = scene.to_json().expect("serialize scene");
        let restored = SceneDoc::from_json(&json).expect("deserialize scene");
        assert_eq!(scene, restored);
    }
}
