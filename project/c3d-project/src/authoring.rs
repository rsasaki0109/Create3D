use std::path::PathBuf;

use c3d_asset_db::AssetKind;
use c3d_asset_material::{MaterialAsset, MaterialAssetData, MaterialGraphData};
use c3d_asset_mesh::{MeshAsset, MeshAssetData};
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_mesh_authoring::{
    compute_normals, compute_tangents, primitive, render_mesh_thumbnail_png, AuthoringMesh,
    PrimitiveKind,
};
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{MaterialBinding, MeshRef, Name, Transform};

use crate::error::{ProjectError, ProjectResult};
use crate::Project;

/// Result of creating a primitive entity in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveCreateReport {
    /// Created scene entity id.
    pub entity_id: EntityId,
    /// Stored mesh asset id.
    pub mesh_id: AssetId,
    /// Stored material asset id.
    pub material_id: AssetId,
}

impl Project {
    /// Store a mesh asset blob in the project database.
    pub fn store_mesh(
        &mut self,
        ids: &mut UlidGenerator,
        name: impl Into<String>,
        mesh: MeshAssetData,
    ) -> ProjectResult<AssetId> {
        AuthoringMesh::from_render_mesh(mesh.clone())
            .map_err(|err| ProjectError::Mesh(err.to_string()))?;
        let bytes = MeshAsset::encode(&mesh).map_err(|err| ProjectError::Mesh(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::Mesh,
            name,
            &bytes,
            Some("application/json".into()),
        )?;
        Ok(asset_id)
    }

    /// Store a material asset blob in the project database.
    pub fn store_material(
        &mut self,
        ids: &mut UlidGenerator,
        name: impl Into<String>,
        material: MaterialAssetData,
    ) -> ProjectResult<AssetId> {
        let bytes = MaterialAsset::encode(&material)
            .map_err(|err| ProjectError::Material(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::Material,
            name,
            &bytes,
            Some("application/json".into()),
        )?;
        Ok(asset_id)
    }

    /// Replace an existing material asset blob.
    pub fn update_material(
        &mut self,
        material_id: AssetId,
        material: MaterialAssetData,
    ) -> ProjectResult<()> {
        let bytes = MaterialAsset::encode(&material)
            .map_err(|err| ProjectError::Material(err.to_string()))?;
        self.assets_mut()
            .replace_blob(material_id, &bytes)
            .map_err(ProjectError::from)?;
        Ok(())
    }

    /// Create a primitive mesh/material pair and scene entity.
    pub fn create_primitive(
        &mut self,
        ids: &mut UlidGenerator,
        kind: PrimitiveKind,
        name: impl Into<String>,
    ) -> ProjectResult<PrimitiveCreateReport> {
        let name = name.into();
        let mut mesh = primitive(kind);
        compute_normals(&mut mesh);
        compute_tangents(&mut mesh);
        AuthoringMesh::from_render_mesh(mesh.clone())
            .map_err(|err| ProjectError::Mesh(err.to_string()))?;

        let mesh_id = self.store_mesh(ids, format!("{name}-mesh"), mesh)?;
        let material = MaterialAssetData {
            version: 1,
            base_color: match kind {
                PrimitiveKind::UnitCube => [0.75, 0.55, 0.25, 1.0],
                PrimitiveKind::Plane => [0.35, 0.65, 0.45, 1.0],
            },
            base_color_texture: None,
            graph: Some(MaterialGraphData::from_base_color(match kind {
                PrimitiveKind::UnitCube => [0.75, 0.55, 0.25, 1.0],
                PrimitiveKind::Plane => [0.35, 0.65, 0.45, 1.0],
            })),
        };
        let material_id = self.store_material(ids, format!("{name}-material"), material)?;

        let entity_id = ids.next_entity_id();
        apply_operations(
            self.scene_mut(),
            &[SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new(name)),
                transform: Transform::IDENTITY,
                mesh_ref: Some(MeshRef::new(mesh_id)),
                material_binding: Some(MaterialBinding::new(material_id)),
                point_cloud_ref: None,
            }],
        )?;

        let _ = self.write_mesh_thumbnail(mesh_id, material_id)?;

        Ok(PrimitiveCreateReport {
            entity_id,
            mesh_id,
            material_id,
        })
    }

    /// Generate and persist a PNG thumbnail for a mesh/material pair.
    pub fn write_mesh_thumbnail(
        &self,
        mesh_id: AssetId,
        material_id: AssetId,
    ) -> ProjectResult<PathBuf> {
        let mesh = self.mesh_asset(mesh_id)?;
        let material = self.material_asset(material_id)?;
        let png = render_mesh_thumbnail_png(&mesh, &material, 128).map_err(ProjectError::Mesh)?;
        let dir = self.root().join("thumbnails");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{mesh_id}.png"));
        std::fs::write(&path, png)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_mesh_authoring::unit_cube;

    #[test]
    fn create_primitive_writes_thumbnail() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "primitive-test").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .create_primitive(&mut ids, PrimitiveKind::UnitCube, "Cube")
            .expect("create primitive");
        let path = project
            .write_mesh_thumbnail(report.mesh_id, report.material_id)
            .expect("thumbnail path");
        assert!(path.is_file());
        let mesh = unit_cube();
        assert!(project
            .mesh_asset(report.mesh_id)
            .expect("mesh")
            .validate()
            .is_ok());
        let _ = mesh;
    }
}
