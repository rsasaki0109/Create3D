use bevy_ecs::prelude::*;
use c3d_core::math::Mat4;
use c3d_core::EntityId;
use c3d_scene_doc::SceneDoc;

use crate::components::{
    world_affine_from_scene, RenderGaussianSplat, RenderMaterial, RenderMeshKind, RenderPointCloud,
    SceneEntity, SceneTransform,
};

/// ECS world used by editor runtime systems.
#[derive(Default)]
pub struct RuntimeWorld {
    /// Underlying Bevy ECS world.
    pub world: World,
}

/// Drawable mesh instance extracted from the runtime ECS world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneDrawable {
    /// Authoritative scene entity id.
    pub entity_id: EntityId,
    /// World transform matrix.
    pub world: Mat4,
    /// Mesh kind to render.
    pub mesh: RenderMeshKind,
    /// Optional material asset for imported meshes.
    pub material_id: Option<c3d_core::AssetId>,
}

/// Drawable point cloud instance extracted from the runtime ECS world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePointCloudDrawable {
    /// Authoritative scene entity id.
    pub entity_id: EntityId,
    /// World transform matrix.
    pub world: Mat4,
    /// Point cloud render parameters.
    pub point_cloud: RenderPointCloud,
}

/// Drawable Gaussian splat instance extracted from the runtime ECS world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneGaussianSplatDrawable {
    /// Authoritative scene entity id.
    pub entity_id: EntityId,
    /// World transform matrix.
    pub world: Mat4,
    /// Gaussian splat render parameters.
    pub gaussian_splat: RenderGaussianSplat,
}

impl RuntimeWorld {
    /// Create an empty runtime world.
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    /// Collect world matrices for cube placeholders in the runtime ECS.
    pub fn cube_transforms(&mut self) -> Vec<Mat4> {
        self.drawables()
            .into_iter()
            .filter_map(|drawable| {
                matches!(drawable.mesh, RenderMeshKind::Cube).then_some(drawable.world)
            })
            .collect()
    }

    /// Collect drawable mesh instances from the runtime ECS world.
    pub fn drawables(&mut self) -> Vec<SceneDrawable> {
        let mut query = self.world.query::<(
            Entity,
            &SceneEntity,
            &SceneTransform,
            &RenderMeshKind,
            Option<&RenderMaterial>,
        )>();
        query
            .iter(&self.world)
            .map(
                |(_entity, scene_entity, transform, mesh, material)| SceneDrawable {
                    entity_id: scene_entity.id,
                    world: transform.world,
                    mesh: *mesh,
                    material_id: material.map(|value| value.material_id),
                },
            )
            .collect()
    }

    /// Collect drawable point cloud instances from the runtime ECS world.
    pub fn point_cloud_drawables(&mut self) -> Vec<ScenePointCloudDrawable> {
        let mut query = self
            .world
            .query::<(Entity, &SceneEntity, &SceneTransform, &RenderPointCloud)>();
        query
            .iter(&self.world)
            .map(
                |(_entity, scene_entity, transform, point_cloud)| ScenePointCloudDrawable {
                    entity_id: scene_entity.id,
                    world: transform.world,
                    point_cloud: *point_cloud,
                },
            )
            .collect()
    }

    /// Collect drawable Gaussian splat instances from the runtime ECS world.
    pub fn gaussian_splat_drawables(&mut self) -> Vec<SceneGaussianSplatDrawable> {
        let mut query = self
            .world
            .query::<(Entity, &SceneEntity, &SceneTransform, &RenderGaussianSplat)>();
        query
            .iter(&self.world)
            .map(
                |(_entity, scene_entity, transform, gaussian_splat)| SceneGaussianSplatDrawable {
                    entity_id: scene_entity.id,
                    world: transform.world,
                    gaussian_splat: *gaussian_splat,
                },
            )
            .collect()
    }
}

/// Synchronize SceneDB entities into the runtime ECS world.
pub fn project_scene_to_ecs(scene: &SceneDoc, runtime: &mut RuntimeWorld) {
    let mut seen = Vec::new();
    for entity in scene.entities() {
        seen.push(entity.id);
        let world_affine =
            world_affine_from_scene(scene, entity.id).expect("entity exists during iteration");
        let world = SceneTransform::from_affine(world_affine);

        if let Some(ecs_entity) = find_ecs_entity(&mut runtime.world, entity.id) {
            let mut entity_mut = runtime.world.entity_mut(ecs_entity);
            entity_mut.insert(world);
            if let Some(gaussian_splat_ref) = entity.gaussian_splat_ref.as_ref() {
                entity_mut.insert(RenderGaussianSplat {
                    asset_id: gaussian_splat_ref.asset_id,
                    opacity_scale: gaussian_splat_ref.opacity_scale,
                    size_scale: gaussian_splat_ref.size_scale,
                    crop_filter: gaussian_splat_ref.crop_filter,
                });
                entity_mut.remove::<RenderPointCloud>();
                entity_mut.remove::<RenderMeshKind>();
                entity_mut.remove::<RenderMaterial>();
            } else if let Some(point_cloud_ref) = entity.point_cloud_ref.as_ref() {
                entity_mut.insert(RenderPointCloud {
                    asset_id: point_cloud_ref.asset_id,
                    color_mode: point_cloud_ref.color_mode,
                    crop_filter: point_cloud_ref.crop_filter,
                });
                entity_mut.remove::<RenderGaussianSplat>();
                entity_mut.remove::<RenderMeshKind>();
                entity_mut.remove::<RenderMaterial>();
            } else {
                entity_mut.remove::<RenderGaussianSplat>();
                entity_mut.remove::<RenderPointCloud>();
                let mesh_kind = entity
                    .mesh_ref
                    .as_ref()
                    .map(|mesh_ref| RenderMeshKind::Asset(mesh_ref.asset_id))
                    .unwrap_or(RenderMeshKind::Cube);
                entity_mut.insert(mesh_kind);
                if let Some(binding) = entity.material_binding.as_ref() {
                    entity_mut.insert(RenderMaterial {
                        material_id: binding.material_id,
                    });
                } else {
                    entity_mut.remove::<RenderMaterial>();
                }
            }
        } else if let Some(gaussian_splat_ref) = entity.gaussian_splat_ref.as_ref() {
            runtime.world.spawn((
                SceneEntity { id: entity.id },
                world,
                RenderGaussianSplat {
                    asset_id: gaussian_splat_ref.asset_id,
                    opacity_scale: gaussian_splat_ref.opacity_scale,
                    size_scale: gaussian_splat_ref.size_scale,
                    crop_filter: gaussian_splat_ref.crop_filter,
                },
            ));
        } else if let Some(point_cloud_ref) = entity.point_cloud_ref.as_ref() {
            runtime.world.spawn((
                SceneEntity { id: entity.id },
                world,
                RenderPointCloud {
                    asset_id: point_cloud_ref.asset_id,
                    color_mode: point_cloud_ref.color_mode,
                    crop_filter: point_cloud_ref.crop_filter,
                },
            ));
        } else {
            let mesh_kind = entity
                .mesh_ref
                .as_ref()
                .map(|mesh_ref| RenderMeshKind::Asset(mesh_ref.asset_id))
                .unwrap_or(RenderMeshKind::Cube);
            let mut entity_cmd =
                runtime
                    .world
                    .spawn((SceneEntity { id: entity.id }, world, mesh_kind));
            if let Some(binding) = entity.material_binding.as_ref() {
                entity_cmd.insert(RenderMaterial {
                    material_id: binding.material_id,
                });
            }
        }
    }

    despawn_missing(&mut runtime.world, &seen);
}

fn find_ecs_entity(world: &mut World, entity_id: EntityId) -> Option<Entity> {
    let mut query = world.query::<(Entity, &SceneEntity)>();
    query
        .iter(world)
        .find_map(|(entity, scene_entity)| (scene_entity.id == entity_id).then_some(entity))
}

fn despawn_missing(world: &mut World, keep: &[EntityId]) {
    let mut to_despawn = Vec::new();
    let mut query = world.query::<(Entity, &SceneEntity)>();
    for (entity, scene_entity) in query.iter(world) {
        if !keep.contains(&scene_entity.id) {
            to_despawn.push(entity);
        }
    }
    for entity in to_despawn {
        world.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::{GaussianSplatRef, PointCloudRef};

    #[test]
    fn projection_spawns_scene_entities() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        scene
            .insert_entity(Entity::new(entity_id), None)
            .expect("insert entity");

        let mut runtime = RuntimeWorld::new();
        project_scene_to_ecs(&scene, &mut runtime);

        let mut query = runtime.world.query::<&SceneEntity>();
        assert_eq!(query.iter(&runtime.world).count(), 1);
    }

    #[test]
    fn point_cloud_entities_skip_default_cube() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = Entity::new(entity_id);
        entity.point_cloud_ref = Some(PointCloudRef::new(c3d_core::AssetId::new()));
        scene.insert_entity(entity, None).expect("insert entity");

        let mut runtime = RuntimeWorld::new();
        project_scene_to_ecs(&scene, &mut runtime);

        assert!(runtime.drawables().is_empty());
        assert_eq!(runtime.point_cloud_drawables().len(), 1);
    }

    #[test]
    fn gaussian_splat_entities_skip_default_cube() {
        let mut scene = SceneDoc::new();
        let entity_id = EntityId::new();
        let mut entity = Entity::new(entity_id);
        entity.gaussian_splat_ref = Some(GaussianSplatRef::new(c3d_core::AssetId::new()));
        scene.insert_entity(entity, None).expect("insert entity");

        let mut runtime = RuntimeWorld::new();
        project_scene_to_ecs(&scene, &mut runtime);

        assert!(runtime.drawables().is_empty());
        assert_eq!(runtime.gaussian_splat_drawables().len(), 1);
    }
}
