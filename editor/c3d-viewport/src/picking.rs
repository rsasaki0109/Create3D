use c3d_asset_db::AssetDb;
use c3d_asset_mesh::MeshAsset;
use c3d_core::math::{Vec2, Vec3, Vec4};
use c3d_core::EntityId;
use c3d_ecs::{RenderMeshKind, SceneDrawable};
use c3d_scene_doc::SceneDoc;

use crate::camera::OrbitCamera;

/// Result of a viewport pick query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickHit {
    /// Selected entity id.
    pub entity_id: EntityId,
    /// Distance along the pick ray.
    pub distance: f32,
}

/// Cast a ray through viewport pixels and return the closest entity hit.
pub fn pick_entity(
    scene: &SceneDoc,
    assets: &AssetDb,
    drawables: &[SceneDrawable],
    camera: &OrbitCamera,
    aspect: f32,
    screen_pos: Vec2,
    viewport_size: Vec2,
) -> Option<PickHit> {
    let (origin, direction) = screen_ray(camera, aspect, screen_pos, viewport_size);
    drawables
        .iter()
        .filter_map(|drawable| {
            let (local_min, local_max) = local_bounds_for_drawable(scene, assets, drawable)?;
            let world = drawable.world;
            let inv_world = world.inverse();
            let local_origin = inv_world.transform_point3(origin);
            let local_direction = inv_world.transform_vector3(direction).normalize();
            let (t_min, t_max) =
                intersect_ray_aabb(local_origin, local_direction, local_min, local_max)?;
            let world_entry = origin + direction * t_min;
            let distance = (world_entry - origin).length();
            (t_min >= 0.0 && t_max >= t_min).then_some(PickHit {
                entity_id: drawable.entity_id,
                distance,
            })
        })
        .min_by(|left, right| {
            left.distance
                .partial_cmp(&right.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn local_bounds_for_drawable(
    _scene: &SceneDoc,
    assets: &AssetDb,
    drawable: &SceneDrawable,
) -> Option<(Vec3, Vec3)> {
    match drawable.mesh {
        RenderMeshKind::Cube => Some((Vec3::splat(-0.5), Vec3::splat(0.5))),
        RenderMeshKind::Asset(asset_id) => {
            let bytes = assets.read_blob(asset_id).ok()?;
            let mesh = MeshAsset::decode(&bytes).ok()?;
            let (min, max) = mesh.local_bounds()?;
            Some((Vec3::from_array(min), Vec3::from_array(max)))
        }
    }
}

pub(crate) fn screen_ray(
    camera: &OrbitCamera,
    aspect: f32,
    screen_pos: Vec2,
    viewport_size: Vec2,
) -> (Vec3, Vec3) {
    let width = viewport_size.x.max(1.0);
    let height = viewport_size.y.max(1.0);
    let ndc_x = (screen_pos.x / width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.y / height) * 2.0;
    let view_proj = camera.view_projection(aspect);
    let inv = view_proj.inverse();
    let near = inv * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let far = inv * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    let near = near.truncate() / near.w;
    let far = far.truncate() / far.w;
    let origin = camera.eye_position();
    let direction = (far - near).normalize();
    (origin, direction)
}

fn intersect_ray_aabb(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<(f32, f32)> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    for axis in 0..3 {
        let origin_axis = origin[axis];
        let direction_axis = direction[axis];
        if direction_axis.abs() < f32::EPSILON {
            if origin_axis < min[axis] || origin_axis > max[axis] {
                return None;
            }
            continue;
        }
        let t1 = (min[axis] - origin_axis) / direction_axis;
        let t2 = (max[axis] - origin_axis) / direction_axis;
        let (near, far) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        t_min = t_min.max(near);
        t_max = t_max.min(far);
        if t_max < t_min {
            return None;
        }
    }
    Some((t_min, t_max))
}

#[cfg(test)]
mod tests {
    use c3d_core::math::Mat4;

    use super::*;

    #[test]
    fn ray_hits_unit_cube_at_origin() {
        let camera = OrbitCamera::default();
        let drawable = SceneDrawable {
            entity_id: EntityId::new(),
            world: Mat4::IDENTITY,
            mesh: RenderMeshKind::Cube,
            material_id: None,
        };
        let scene = SceneDoc::new();
        let assets =
            AssetDb::open(std::env::temp_dir().join("create3d-pick-test")).expect("asset db");
        let hit = pick_entity(
            &scene,
            &assets,
            &[drawable],
            &camera,
            1.0,
            Vec2::new(640.0, 360.0),
            Vec2::new(1280.0, 720.0),
        );
        assert!(hit.is_some());
    }
}
