use bevy_ecs::prelude::*;
use c3d_core::math::{Affine3A, Mat4};
use c3d_core::{AssetId, EntityId};
use c3d_scene_schema::{PointCloudColorMode, PointCloudCropBox, Transform};

/// Links an ECS entity to a SceneDB entity id.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneEntity {
    /// Authoritative scene entity identifier.
    pub id: EntityId,
}

/// World-space transform extracted from SceneDB.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SceneTransform {
    /// World matrix used by the viewport renderer.
    pub world: Mat4,
}

/// Placeholder render primitive attached to scene entities.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMeshKind {
    /// Unit cube placeholder mesh.
    Cube,
    /// Imported mesh asset from AssetDB.
    Asset(AssetId),
}

/// Material asset used when rendering imported meshes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderMaterial {
    /// Material asset identifier when bound.
    pub material_id: AssetId,
}

/// Point cloud asset rendered as GPU point sprites.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct RenderPointCloud {
    /// Point cloud metadata asset identifier.
    pub asset_id: AssetId,
    /// Viewport color mode for attribute mapping.
    pub color_mode: PointCloudColorMode,
    /// Optional scene-level crop filter.
    pub crop_filter: Option<PointCloudCropBox>,
}

impl SceneTransform {
    /// Build a transform from a SceneDB local transform and optional parent world matrix.
    pub fn from_local(local: Transform, parent_world: Option<Mat4>) -> Self {
        let local_matrix =
            Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation);
        let world = parent_world.map_or(local_matrix, |parent| parent * local_matrix);
        Self { world }
    }

    /// Build from an affine world transform.
    pub fn from_affine(world: Affine3A) -> Self {
        Self {
            world: Mat4::from(world),
        }
    }
}

/// Compute world affine transform by walking SceneDB parents.
pub fn world_affine_from_scene(
    scene: &c3d_scene_doc::SceneDoc,
    entity_id: EntityId,
) -> Option<Affine3A> {
    let entity = scene.get(entity_id)?;
    let local = Affine3A::from_scale_rotation_translation(
        entity.transform.scale,
        entity.transform.rotation,
        entity.transform.translation,
    );
    if let Some(parent_id) = entity.parent {
        let parent = world_affine_from_scene(scene, parent_id)?;
        Some(parent * local)
    } else {
        Some(local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::math::Vec3;
    use c3d_core::EntityId;
    use c3d_scene_doc::{Entity, SceneDoc};

    #[test]
    fn child_inherits_parent_translation() {
        let mut scene = SceneDoc::new();
        let parent_id = EntityId::new();
        let child_id = EntityId::new();

        let mut parent = Entity::new(parent_id);
        parent.transform.translation = Vec3::new(2.0, 0.0, 0.0);
        scene.insert_entity(parent, None).expect("insert parent");

        let mut child = Entity::new(child_id);
        child.transform.translation = Vec3::new(0.0, 1.0, 0.0);
        scene
            .insert_entity(child, Some(parent_id))
            .expect("insert child");

        let world = world_affine_from_scene(&scene, child_id).expect("world transform");
        assert!((world.translation.x - 2.0).abs() < 1e-5);
        assert!((world.translation.y - 1.0).abs() < 1e-5);
    }
}
