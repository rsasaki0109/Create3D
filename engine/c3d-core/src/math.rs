//! Math types and helpers for `Create3D`.
//!
//! The engine uses `glam` as the baseline linear algebra library. Higher-level crates
//! should depend on this module rather than importing `glam` directly when possible.

pub use glam::{
    Affine2, Affine3A, DAffine2, DAffine3, DMat2, DMat3, DMat4, DQuat, DVec2, DVec3, DVec4, IVec2,
    IVec3, IVec4, Mat2, Mat3, Mat3A, Mat4, Quat, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4,
};

/// Default world-space units are meters.
pub const DEFAULT_LENGTH_UNIT_METERS: f64 = 1.0;

/// Returns true when two vectors are approximately equal within `epsilon`.
pub fn approx_eq_vec3(a: Vec3, b: Vec3, epsilon: f32) -> bool {
    (a - b).length() <= epsilon
}

/// Returns true when two transforms are approximately equal within `epsilon`.
pub fn approx_eq_affine3(a: Affine3A, b: Affine3A, epsilon: f32) -> bool {
    approx_eq_vec3(a.translation.into(), b.translation.into(), epsilon)
        && a.matrix3.abs_diff_eq(b.matrix3, epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_eq_vec3_works() {
        assert!(approx_eq_vec3(Vec3::ZERO, Vec3::new(1e-7, 0.0, 0.0), 1e-6));
        assert!(!approx_eq_vec3(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1e-6));
    }
}
