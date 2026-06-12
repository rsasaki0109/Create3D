use c3d_core::math::{Mat4, Vec2, Vec3, Vec4};

use crate::camera::OrbitCamera;
use crate::mesh::Vertex;
use crate::picking::screen_ray;

/// Gizmo axis identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    /// X axis.
    X,
    /// Y axis.
    Y,
    /// Z axis.
    Z,
}

/// Active gizmo drag state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoDragState {
    /// Dragged axis.
    pub axis: GizmoAxis,
    /// Screen position where the drag started.
    pub start_screen: Vec2,
    /// World translation accumulated during the drag.
    pub accumulated: Vec3,
}

impl GizmoDragState {
    /// Create a new drag state.
    pub fn new(axis: GizmoAxis, start_screen: Vec2) -> Self {
        Self {
            axis,
            start_screen,
            accumulated: Vec3::ZERO,
        }
    }
}

/// Pick a translate gizmo axis from a viewport click.
pub fn pick_gizmo_axis(
    origin: Vec3,
    camera: &OrbitCamera,
    aspect: f32,
    screen_pos: Vec2,
    viewport_size: Vec2,
    pick_radius: f32,
) -> Option<GizmoAxis> {
    let (ray_origin, ray_direction) = screen_ray(camera, aspect, screen_pos, viewport_size);
    let axes = [
        (GizmoAxis::X, Vec3::X, [1.0, 0.2, 0.2, 1.0]),
        (GizmoAxis::Y, Vec3::Y, [0.2, 1.0, 0.2, 1.0]),
        (GizmoAxis::Z, Vec3::Z, [0.2, 0.4, 1.0, 1.0]),
    ];

    let mut best: Option<(GizmoAxis, f32)> = None;
    for (axis, direction, _color) in axes {
        if let Some(distance) =
            pick_axis_line(origin, direction, ray_origin, ray_direction, pick_radius)
        {
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((axis, distance));
            }
        }
    }
    best.map(|(axis, _)| axis)
}

/// Update drag accumulation from the current pointer position.
pub fn gizmo_drag_delta(
    _origin: Vec3,
    axis: GizmoAxis,
    camera: &OrbitCamera,
    _aspect: f32,
    start_screen: Vec2,
    current_screen: Vec2,
    _viewport_size: Vec2,
) -> Vec3 {
    let axis_vector = match axis {
        GizmoAxis::X => Vec3::X,
        GizmoAxis::Y => Vec3::Y,
        GizmoAxis::Z => Vec3::Z,
    };
    let view = camera.view_matrix();
    let right = view.row(0).truncate().normalize();
    let up = view.row(1).truncate().normalize();
    let screen_delta = current_screen - start_screen;
    let scale = camera.distance * 0.002;
    let motion = right * screen_delta.x - up * screen_delta.y;
    axis_vector * motion.dot(axis_vector) * scale
}

/// Build line vertices for a translate gizmo at the given origin.
pub(crate) fn gizmo_vertices(origin: Vec3, length: f32) -> Vec<Vertex> {
    let axes = [
        (Vec3::X * length, [1.0, 0.25, 0.25, 1.0]),
        (Vec3::Y * length, [0.25, 1.0, 0.25, 1.0]),
        (Vec3::Z * length, [0.25, 0.45, 1.0, 1.0]),
    ];
    axes.into_iter()
        .flat_map(|(offset, color)| {
            [
                Vertex {
                    position: origin.to_array(),
                    color,
                },
                Vertex {
                    position: (origin + offset).to_array(),
                    color,
                },
            ]
        })
        .collect()
}

fn pick_axis_line(
    origin: Vec3,
    axis: Vec3,
    ray_origin: Vec3,
    ray_direction: Vec3,
    radius: f32,
) -> Option<f32> {
    let end = origin + axis;
    let segment = end - origin;
    let segment_length_sq = segment.length_squared().max(f32::EPSILON);
    let _t = ((ray_origin - origin).cross(ray_direction).length_squared()).min(segment_length_sq);
    let projection = ((ray_origin - origin).dot(segment) / segment_length_sq).clamp(0.0, 1.0);
    let closest = origin + segment * projection;
    let distance = (closest - ray_origin).cross(ray_direction).length();
    (distance <= radius).then_some(distance)
}

#[allow(dead_code)]
fn world_to_screen(world: Vec3, view_proj: Mat4, viewport_size: Vec2) -> Option<Vec2> {
    let clip = view_proj * Vec4::from((world, 1.0));
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Vec2::new(
        (ndc.x * 0.5 + 0.5) * viewport_size.x,
        (1.0 - (ndc.y * 0.5 + 0.5)) * viewport_size.y,
    ))
}
