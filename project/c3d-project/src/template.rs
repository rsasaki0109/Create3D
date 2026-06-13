use std::path::Path;

use c3d_core::UlidGenerator;
use c3d_mesh_authoring::PrimitiveKind;
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::Transform;
use c3d_urdf::preview_arm_urdf;

use crate::error::ProjectResult;
use crate::Project;

/// Built-in sample project templates shipped with Create3D Alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    /// Empty scene with no entities.
    Empty,
    /// Floor plane and unit cube primitives.
    MeshScene,
    /// Synthetic chunked point cloud for viewport residency testing.
    PointCloudScene,
    /// Synthetic Gaussian splat cloud.
    GaussianSplatScene,
    /// Preview URDF arm hierarchy with mock bridge joints.
    UrdfRobotScene,
    /// Named entity setup for Copilot move/rename demos.
    AiEditingDemo,
}

impl ProjectTemplate {
    /// Parse a template id from CLI input.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "empty" => Some(Self::Empty),
            "mesh" | "mesh-scene" => Some(Self::MeshScene),
            "point-cloud" | "pointcloud" | "point-cloud-scene" => Some(Self::PointCloudScene),
            "gsplat" | "gaussian-splat" | "gaussian-splat-scene" => Some(Self::GaussianSplatScene),
            "urdf" | "robot" | "urdf-robot-scene" => Some(Self::UrdfRobotScene),
            "ai" | "ai-editing" | "ai-editing-demo" => Some(Self::AiEditingDemo),
            _ => None,
        }
    }

    /// Stable template id for CLI and docs.
    pub fn id(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::MeshScene => "mesh-scene",
            Self::PointCloudScene => "point-cloud-scene",
            Self::GaussianSplatScene => "gaussian-splat-scene",
            Self::UrdfRobotScene => "urdf-robot-scene",
            Self::AiEditingDemo => "ai-editing-demo",
        }
    }

    /// Human-readable description.
    pub fn description(self) -> &'static str {
        match self {
            Self::Empty => "Empty SceneDB document with no entities.",
            Self::MeshScene => "Floor plane and unit cube mesh primitives.",
            Self::PointCloudScene => "Synthetic chunked point cloud (5k points).",
            Self::GaussianSplatScene => "Synthetic Gaussian splat cloud (2k splats).",
            Self::UrdfRobotScene => "Preview URDF arm with link/joint hierarchy.",
            Self::AiEditingDemo => "Single Lamp entity for Copilot move/rename demos.",
        }
    }

    /// All templates in display order.
    pub fn all() -> &'static [Self] {
        &[
            Self::Empty,
            Self::MeshScene,
            Self::PointCloudScene,
            Self::GaussianSplatScene,
            Self::UrdfRobotScene,
            Self::AiEditingDemo,
        ]
    }
}

impl Project {
    /// Create a new project populated from a built-in template.
    pub fn create_from_template(
        root: impl AsRef<Path>,
        name: impl Into<String>,
        template: ProjectTemplate,
    ) -> ProjectResult<Self> {
        let mut project = Self::create(root, name)?;
        let mut ids = UlidGenerator::new();
        match template {
            ProjectTemplate::Empty => {}
            ProjectTemplate::MeshScene => {
                project.create_primitive(&mut ids, PrimitiveKind::Plane, "Floor")?;
                project.create_primitive(&mut ids, PrimitiveKind::UnitCube, "Cube")?;
            }
            ProjectTemplate::PointCloudScene => {
                project.import_synthetic_point_cloud(5_000, &mut ids)?;
            }
            ProjectTemplate::GaussianSplatScene => {
                project.import_synthetic_gaussian_splats(2_000, &mut ids)?;
            }
            ProjectTemplate::UrdfRobotScene => {
                project.import_urdf_xml(preview_arm_urdf(), &mut ids)?;
            }
            ProjectTemplate::AiEditingDemo => {
                let report = project.create_primitive(&mut ids, PrimitiveKind::UnitCube, "Lamp")?;
                apply_operations(
                    project.scene_mut(),
                    &[SceneOperation::SetTransform {
                        entity_id: report.entity_id,
                        transform: Transform {
                            translation: c3d_core::math::Vec3::new(0.0, 0.5, 0.0),
                            ..Transform::IDENTITY
                        },
                    }],
                )?;
            }
        }
        project.save()?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_template_creates_primitives() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project =
            Project::create_from_template(temp.path(), "mesh", ProjectTemplate::MeshScene)
                .expect("create");
        assert!(project.scene().entity_count() >= 2);
    }

    #[test]
    fn ai_demo_names_lamp_entity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project =
            Project::create_from_template(temp.path(), "ai", ProjectTemplate::AiEditingDemo)
                .expect("create");
        let has_lamp = project.scene().entities().any(|entity| {
            entity
                .name
                .as_ref()
                .is_some_and(|name| name.value == "Lamp")
        });
        assert!(has_lamp);
    }
}
