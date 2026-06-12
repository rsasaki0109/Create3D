use c3d_core::math::{Mat4, Quat, Vec2, Vec3};

/// Orbit camera used by the editor viewport.
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    /// Orbit target point.
    pub target: Vec3,
    /// Yaw in radians.
    pub yaw: f32,
    /// Pitch in radians.
    pub pitch: f32,
    /// Distance from target.
    pub distance: f32,
    /// Vertical field of view in radians.
    pub fov_y: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.8,
            pitch: 0.5,
            distance: 6.0,
            fov_y: 45.0_f32.to_radians(),
        }
    }
}

impl OrbitCamera {
    /// Build a view matrix for the current orbit parameters.
    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_position();
        Mat4::look_at_rh(eye, self.target, Vec3::Y)
    }

    /// Build a perspective projection matrix for the given aspect ratio.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect.max(0.01), 0.05, 500.0)
    }

    /// Combined view-projection matrix.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Current camera eye position in world space.
    pub fn eye_position(&self) -> Vec3 {
        let rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch);
        self.target + rotation * Vec3::new(0.0, 0.0, self.distance)
    }

    /// Orbit around the target using pixel delta.
    pub fn orbit(&mut self, delta_pixels: Vec2) {
        self.yaw -= delta_pixels.x * 0.01;
        self.pitch = (self.pitch - delta_pixels.y * 0.01).clamp(0.05, 1.5);
    }

    /// Pan the target in camera space.
    pub fn pan(&mut self, delta_pixels: Vec2) {
        let view = self.view_matrix();
        let right = view.row(0).truncate().normalize();
        let up = view.row(1).truncate().normalize();
        let scale = self.distance * 0.002;
        self.target += (-right * delta_pixels.x + up * delta_pixels.y) * scale;
    }

    /// Zoom by adjusting camera distance.
    pub fn zoom(&mut self, scroll_delta: f32) {
        self.distance = (self.distance - scroll_delta * 0.5).clamp(1.0, 100.0);
    }

    /// Move the orbit target to frame a world-space point.
    pub fn focus_on(&mut self, point: Vec3) {
        self.target = point;
        self.distance = self.distance.clamp(2.0, 30.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_distance() {
        let mut camera = OrbitCamera::default();
        camera.zoom(100.0);
        assert!(camera.distance >= 1.0);
    }
}
