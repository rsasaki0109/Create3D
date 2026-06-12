use std::collections::HashMap;
use std::path::Path;

use c3d_asset_material::{MaterialAssetData, MaterialGraphData};
use c3d_core::{EntityId, UlidGenerator};
use c3d_mesh_authoring::{compute_normals, compute_tangents, unit_cube, AuthoringMesh};
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{MaterialBinding, MeshRef, Name, RobotLink, RobotRoot, Transform};
use c3d_urdf::{parse_urdf, parse_urdf_file, UrdfGeometry, UrdfImportPlan};

use crate::error::{ProjectError, ProjectResult};
use crate::import::ImportReport;
use crate::Project;

/// Result of importing a URDF robot into the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrdfImportReport {
    /// Robot root scene entity id.
    pub root_entity_id: EntityId,
    /// Imported robot name.
    pub robot_name: String,
    /// Link entity ids keyed by URDF link name.
    pub link_entities: HashMap<String, EntityId>,
    /// Revolute/prismatic joint names available for live updates.
    pub joint_names: Vec<String>,
}

impl Project {
    /// Import a URDF file into the project and create a kinematic robot hierarchy.
    pub fn import_urdf(
        &mut self,
        path: impl AsRef<Path>,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<UrdfImportReport> {
        let path = path.as_ref();
        let plan =
            parse_urdf_file(path).map_err(|err| ProjectError::import_at_path("URDF", path, err))?;
        let package_path = path
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned());
        self.store_urdf_import(plan, package_path, ids)
    }

    /// Import URDF XML already loaded in memory.
    pub fn import_urdf_xml(
        &mut self,
        xml: &str,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<UrdfImportReport> {
        let plan = parse_urdf(xml).map_err(ProjectError::Urdf)?;
        self.store_urdf_import(plan, None, ids)
    }

    fn store_urdf_import(
        &mut self,
        plan: UrdfImportPlan,
        package_path: Option<String>,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<UrdfImportReport> {
        let mut operations = Vec::new();
        let mut link_entities = HashMap::new();
        let root_entity_id = ids.next_entity_id();

        let mut robot_root = RobotRoot::new(plan.robot_name.clone());
        robot_root.package_path = package_path;

        operations.push(SceneOperation::CreateEntity {
            entity_id: root_entity_id,
            parent: None,
            name: Some(Name::new(plan.robot_name.clone())),
            transform: Transform::IDENTITY,
            mesh_ref: None,
            material_binding: None,
            point_cloud_ref: None,
            gaussian_splat_ref: None,
            robot_root: Some(robot_root),
            robot_link: None,
            robot_joint: None,
        });

        build_link_branch(
            self,
            &plan,
            &plan.root_link,
            root_entity_id,
            &mut link_entities,
            &mut operations,
            ids,
        )?;

        apply_operations(self.scene_mut(), &operations)?;

        let joint_names = plan
            .joints
            .iter()
            .map(|joint| joint.joint.joint_name.clone())
            .collect();

        Ok(UrdfImportReport {
            root_entity_id,
            robot_name: plan.robot_name,
            link_entities,
            joint_names,
        })
    }
}

impl ImportReport {
    /// Merge a URDF import into a generic import report.
    pub fn from_urdf(report: &UrdfImportReport) -> Self {
        Self {
            mesh_assets: Vec::new(),
            material_assets: Vec::new(),
            texture_assets: Vec::new(),
            point_cloud_assets: Vec::new(),
            chunk_assets: Vec::new(),
            gaussian_splat_assets: Vec::new(),
            entity_count: 1 + report.link_entities.len(),
        }
    }
}

fn build_link_branch(
    project: &mut Project,
    plan: &UrdfImportPlan,
    link_name: &str,
    parent_entity: EntityId,
    link_entities: &mut HashMap<String, EntityId>,
    operations: &mut Vec<SceneOperation>,
    ids: &mut UlidGenerator,
) -> ProjectResult<()> {
    let link_spec = plan
        .link(link_name)
        .ok_or_else(|| ProjectError::UrdfImport(format!("missing link `{link_name}`")))?;

    let link_entity_id = ids.next_entity_id();
    let joint_spec = plan.joint_for_child(link_name);
    let (robot_joint, transform) = if let Some(joint_spec) = joint_spec {
        let joint = joint_spec.joint.clone();
        let transform = joint.motion_transform();
        (Some(joint), transform)
    } else {
        (None, Transform::IDENTITY)
    };

    operations.push(SceneOperation::CreateEntity {
        entity_id: link_entity_id,
        parent: Some(parent_entity),
        name: Some(Name::new(link_name)),
        transform,
        mesh_ref: None,
        material_binding: None,
        point_cloud_ref: None,
        gaussian_splat_ref: None,
        robot_root: None,
        robot_link: Some(RobotLink::new(link_name)),
        robot_joint,
    });

    for visual in &link_spec.visuals {
        let mesh_id = project.store_urdf_mesh(ids, &visual.name, &visual.geometry)?;
        let material_id = project.store_urdf_material(ids, &visual.name, visual.color)?;
        let visual_entity_id = ids.next_entity_id();
        let mut visual_transform = visual.origin.to_transform();
        visual_transform.scale = geometry_scale(&visual.geometry);

        operations.push(SceneOperation::CreateEntity {
            entity_id: visual_entity_id,
            parent: Some(link_entity_id),
            name: Some(Name::new(format!("{}_{}", link_name, visual.name))),
            transform: visual_transform,
            mesh_ref: Some(MeshRef::new(mesh_id)),
            material_binding: Some(MaterialBinding::new(material_id)),
            point_cloud_ref: None,
            gaussian_splat_ref: None,
            robot_root: None,
            robot_link: None,
            robot_joint: None,
        });
    }

    link_entities.insert(link_name.to_string(), link_entity_id);

    for joint in plan
        .joints
        .iter()
        .filter(|joint| joint.joint.parent_link == link_name)
    {
        build_link_branch(
            project,
            plan,
            &joint.joint.child_link,
            link_entity_id,
            link_entities,
            operations,
            ids,
        )?;
    }

    Ok(())
}

impl Project {
    fn store_urdf_mesh(
        &mut self,
        ids: &mut UlidGenerator,
        name: &str,
        geometry: &UrdfGeometry,
    ) -> ProjectResult<c3d_core::AssetId> {
        match geometry {
            UrdfGeometry::Mesh { filename, .. } => Err(ProjectError::UrdfImport(format!(
                "external mesh `{filename}` import not implemented in Month 10"
            ))),
            UrdfGeometry::Box { .. }
            | UrdfGeometry::Cylinder { .. }
            | UrdfGeometry::Sphere { .. } => {
                let mut mesh = unit_cube();
                compute_normals(&mut mesh);
                compute_tangents(&mut mesh);
                AuthoringMesh::from_render_mesh(mesh.clone())
                    .map_err(|err| ProjectError::Mesh(err.to_string()))?;
                self.store_mesh(ids, format!("{name}-mesh"), mesh)
            }
        }
    }

    fn store_urdf_material(
        &mut self,
        ids: &mut UlidGenerator,
        name: &str,
        color: [f32; 4],
    ) -> ProjectResult<c3d_core::AssetId> {
        let material = MaterialAssetData {
            version: 1,
            base_color: color,
            base_color_texture: None,
            graph: Some(MaterialGraphData::from_base_color(color)),
        };
        self.store_material(ids, format!("{name}-material"), material)
    }
}

fn geometry_scale(geometry: &UrdfGeometry) -> c3d_core::math::Vec3 {
    match geometry {
        UrdfGeometry::Box { size } => {
            c3d_core::math::Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32)
        }
        UrdfGeometry::Cylinder { radius, length } => {
            c3d_core::math::Vec3::new(*radius as f32 * 2.0, *radius as f32 * 2.0, *length as f32)
        }
        UrdfGeometry::Sphere { radius } => {
            let diameter = *radius as f32 * 2.0;
            c3d_core::math::Vec3::splat(diameter)
        }
        UrdfGeometry::Mesh { scale, .. } => {
            c3d_core::math::Vec3::new(scale[0] as f32, scale[1] as f32, scale[2] as f32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_robotics_core::{apply_joint_state, JointStateUpdate};
    use c3d_urdf::preview_arm_urdf;

    #[test]
    fn urdf_import_creates_robot_hierarchy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "robot").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_urdf_xml(preview_arm_urdf(), &mut ids)
            .expect("import urdf");
        assert_eq!(report.robot_name, "preview_arm");
        assert_eq!(report.link_entities.len(), 2);
        assert!(report.joint_names.contains(&"shoulder".to_string()));

        let root = project
            .scene()
            .get(report.root_entity_id)
            .expect("root entity");
        assert!(root.robot_root.is_some());
    }

    #[test]
    fn mock_joint_state_updates_scene() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "robot").expect("project");
        let mut ids = UlidGenerator::default();
        project
            .import_urdf_xml(preview_arm_urdf(), &mut ids)
            .expect("import urdf");

        apply_joint_state(
            project.scene_mut(),
            &JointStateUpdate {
                joint_name: "shoulder".into(),
                position: 0.4,
            },
        )
        .expect("apply joint state");

        let upper_arm = project
            .scene()
            .entities()
            .find(|entity| {
                entity
                    .robot_link
                    .as_ref()
                    .is_some_and(|link| link.link_name == "upper_arm")
            })
            .expect("upper arm");
        let joint = upper_arm.robot_joint.as_ref().expect("joint");
        assert!((joint.position - 0.4).abs() < 1e-6);
    }

    #[test]
    fn project_save_reload_preserves_robot() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "robot").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_urdf_xml(preview_arm_urdf(), &mut ids)
            .expect("import urdf");
        project.save().expect("save");

        let loaded = Project::open(temp.path()).expect("reload");
        let root = loaded
            .scene()
            .get(report.root_entity_id)
            .expect("root entity");
        assert_eq!(
            root.robot_root
                .as_ref()
                .map(|root| root.robot_name.as_str()),
            Some("preview_arm")
        );
    }
}
