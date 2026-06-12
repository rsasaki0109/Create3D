use c3d_core::math::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Local transform component stored in SceneDB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Local translation in meters.
    pub translation: Vec3,
    /// Local rotation.
    pub rotation: Quat,
    /// Local scale.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Apply a translation delta in local space.
    pub fn translate(&mut self, delta: Vec3) {
        self.translation += delta;
    }

    /// Replace rotation.
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
    }

    /// Replace uniform or non-uniform scale.
    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
    }
}

/// Typed transform edit operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransformOp {
    /// Add a translation delta.
    Translate(Vec3),
    /// Set absolute rotation.
    SetRotation(Quat),
    /// Set absolute scale.
    SetScale(Vec3),
    /// Replace the full transform.
    SetTransform(Transform),
}

impl TransformOp {
    /// Apply this operation to a transform value.
    pub fn apply_to(&self, transform: &mut Transform) {
        match self {
            Self::Translate(delta) => transform.translate(*delta),
            Self::SetRotation(rotation) => transform.set_rotation(*rotation),
            Self::SetScale(scale) => transform.set_scale(*scale),
            Self::SetTransform(value) => *transform = *value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_updates_translation() {
        let mut transform = Transform::IDENTITY;
        TransformOp::Translate(Vec3::new(1.0, 2.0, 3.0)).apply_to(&mut transform);
        assert_eq!(transform.translation, Vec3::new(1.0, 2.0, 3.0));
    }
}
