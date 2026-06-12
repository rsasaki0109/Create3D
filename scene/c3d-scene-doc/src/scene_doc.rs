use std::collections::HashMap;

use c3d_core::version::C3D_SCENE_SCHEMA_CURRENT;
use c3d_core::EntityId;
use c3d_scene_schema::{
    GaussianSplatRef, MaterialBinding, MeshRef, Name, PointCloudRef, SchemaRegistry, Transform,
};

use crate::entity::{Entity, EntitySnapshot};
use crate::error::{SceneError, SceneResult};
use crate::serialize::SceneDocument;

/// Authoritative scene database.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SceneDoc {
    pub(crate) schema_version: u32,
    pub(crate) entities: HashMap<EntityId, Entity>,
}

impl SceneDoc {
    /// Create an empty scene document.
    pub fn new() -> Self {
        Self {
            schema_version: C3D_SCENE_SCHEMA_CURRENT,
            entities: HashMap::new(),
        }
    }

    /// Returns the scene schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the number of entities in the scene.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns true when the entity exists.
    pub fn contains(&self, entity_id: EntityId) -> bool {
        self.entities.contains_key(&entity_id)
    }

    /// Borrow an entity by id.
    pub fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(&entity_id)
    }

    /// Iterate entities in stable id order.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        let mut entities: Vec<_> = self.entities.values().collect();
        entities.sort_by_key(|entity| entity.id);
        entities.into_iter()
    }

    /// Insert a newly created entity and attach it to an optional parent.
    pub fn insert_entity(
        &mut self,
        mut entity: Entity,
        parent: Option<EntityId>,
    ) -> SceneResult<()> {
        if self.entities.contains_key(&entity.id) {
            return Err(SceneError::EntityAlreadyExists(entity.id));
        }

        if let Some(parent_id) = parent {
            if parent_id == entity.id {
                return Err(SceneError::SelfParent(entity.id));
            }
            let parent_entity = self
                .entities
                .get_mut(&parent_id)
                .ok_or(SceneError::ParentNotFound(parent_id))?;
            parent_entity.children.push(entity.id);
        }

        entity.parent = parent;
        self.entities.insert(entity.id, entity);
        Ok(())
    }

    /// Remove an entity and detach it from its parent.
    pub fn remove_entity(&mut self, entity_id: EntityId) -> SceneResult<EntitySnapshot> {
        let entity = self
            .entities
            .get(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;

        if !entity.children.is_empty() {
            return Err(SceneError::EntityHasChildren(entity_id));
        }

        if let Some(parent_id) = entity.parent {
            if let Some(parent) = self.entities.get_mut(&parent_id) {
                parent.children.retain(|child| *child != entity_id);
            }
        }

        let entity = self
            .entities
            .remove(&entity_id)
            .expect("entity existence checked above");

        Ok(entity.snapshot())
    }

    /// Restore an entity from a snapshot.
    pub fn restore_entity(&mut self, snapshot: EntitySnapshot) -> SceneResult<()> {
        let parent = snapshot.parent;
        self.insert_entity(snapshot.into_entity(), parent)
    }

    /// Set the transform component for an entity.
    pub fn set_transform(
        &mut self,
        entity_id: EntityId,
        transform: Transform,
    ) -> SceneResult<Transform> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.transform;
        entity.transform = transform;
        Ok(before)
    }

    /// Set or replace the name component.
    pub fn set_name(&mut self, entity_id: EntityId, name: Name) -> SceneResult<Option<Name>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.name.clone();
        entity.name = Some(name);
        Ok(before)
    }

    /// Set mesh reference placeholder component.
    pub fn set_mesh_ref(
        &mut self,
        entity_id: EntityId,
        mesh_ref: MeshRef,
    ) -> SceneResult<Option<MeshRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.mesh_ref.clone();
        entity.mesh_ref = Some(mesh_ref);
        Ok(before)
    }

    /// Set material binding placeholder component.
    pub fn set_material_binding(
        &mut self,
        entity_id: EntityId,
        material_binding: MaterialBinding,
    ) -> SceneResult<Option<MaterialBinding>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.material_binding.clone();
        entity.material_binding = Some(material_binding);
        Ok(before)
    }

    /// Set point cloud reference placeholder component.
    pub fn set_point_cloud_ref(
        &mut self,
        entity_id: EntityId,
        point_cloud_ref: PointCloudRef,
    ) -> SceneResult<Option<PointCloudRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.point_cloud_ref.clone();
        entity.point_cloud_ref = Some(point_cloud_ref);
        Ok(before)
    }

    /// Set Gaussian splat reference placeholder component.
    pub fn set_gaussian_splat_ref(
        &mut self,
        entity_id: EntityId,
        gaussian_splat_ref: GaussianSplatRef,
    ) -> SceneResult<Option<GaussianSplatRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.gaussian_splat_ref.clone();
        entity.gaussian_splat_ref = Some(gaussian_splat_ref);
        Ok(before)
    }

    /// Restore the name component to a previous value.
    pub fn restore_name(
        &mut self,
        entity_id: EntityId,
        name: Option<Name>,
    ) -> SceneResult<Option<Name>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.name.clone();
        entity.name = name;
        Ok(before)
    }

    /// Restore the mesh reference component to a previous value.
    pub fn restore_mesh_ref(
        &mut self,
        entity_id: EntityId,
        mesh_ref: Option<MeshRef>,
    ) -> SceneResult<Option<MeshRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.mesh_ref.clone();
        entity.mesh_ref = mesh_ref;
        Ok(before)
    }

    /// Restore the material binding component to a previous value.
    pub fn restore_material_binding(
        &mut self,
        entity_id: EntityId,
        material_binding: Option<MaterialBinding>,
    ) -> SceneResult<Option<MaterialBinding>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.material_binding.clone();
        entity.material_binding = material_binding;
        Ok(before)
    }

    /// Restore the point cloud reference component to a previous value.
    pub fn restore_point_cloud_ref(
        &mut self,
        entity_id: EntityId,
        point_cloud_ref: Option<PointCloudRef>,
    ) -> SceneResult<Option<PointCloudRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.point_cloud_ref.clone();
        entity.point_cloud_ref = point_cloud_ref;
        Ok(before)
    }

    /// Restore the Gaussian splat reference component to a previous value.
    pub fn restore_gaussian_splat_ref(
        &mut self,
        entity_id: EntityId,
        gaussian_splat_ref: Option<GaussianSplatRef>,
    ) -> SceneResult<Option<GaussianSplatRef>> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(SceneError::EntityNotFound(entity_id))?;
        let before = entity.gaussian_splat_ref.clone();
        entity.gaussian_splat_ref = gaussian_splat_ref;
        Ok(before)
    }

    /// Serialize the scene to JSON.
    pub fn to_json(&self) -> SceneResult<String> {
        let document = SceneDocument::from_scene(self);
        serde_json::to_string_pretty(&document)
            .map_err(|err| SceneError::Serialization(err.to_string()))
    }

    /// Deserialize a scene from JSON.
    pub fn from_json(value: &str) -> SceneResult<Self> {
        let document: SceneDocument = serde_json::from_str(value)
            .map_err(|err| SceneError::Serialization(err.to_string()))?;
        document.into_scene()
    }

    /// Deserialize a scene from JSON with schema validation.
    pub fn from_json_validated(value: &str, registry: &SchemaRegistry) -> SceneResult<Self> {
        let document: SceneDocument = serde_json::from_str(value)
            .map_err(|err| SceneError::Serialization(err.to_string()))?;
        registry.validate_scene_schema(document.schema_version)?;
        document.into_scene()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::EntityId;

    #[test]
    fn insert_and_remove_entity() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        scene
            .insert_entity(Entity::new(entity_id), None)
            .expect("insert entity");

        assert_eq!(scene.entity_count(), 1);

        let snapshot = scene.remove_entity(entity_id).expect("remove entity");
        assert_eq!(snapshot.id, entity_id);
        assert_eq!(scene.entity_count(), 0);
    }

    #[test]
    fn cannot_delete_entity_with_children() {
        let mut scene = SceneDoc::new();
        let parent_id = EntityId::new();
        let child_id = EntityId::new();
        scene
            .insert_entity(Entity::new(parent_id), None)
            .expect("insert parent");
        scene
            .insert_entity(Entity::new(child_id), Some(parent_id))
            .expect("insert child");

        assert!(matches!(
            scene.remove_entity(parent_id),
            Err(SceneError::EntityHasChildren(id)) if id == parent_id
        ));
    }
}
