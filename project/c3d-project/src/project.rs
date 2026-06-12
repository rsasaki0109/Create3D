use std::fs;
use std::path::{Path, PathBuf};

use c3d_asset_db::{AssetDb, AssetKind};
use c3d_asset_material::{MaterialAsset, MaterialAssetData};
use c3d_asset_mesh::{MeshAsset, MeshAssetData};
use c3d_core::{AssetId, UlidGenerator};
use c3d_import_gltf::{import_result_to_scene_operations, GltfImportResult};
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::apply_operations;

use crate::error::{ProjectError, ProjectResult};
use crate::manifest::ProjectManifest;
use crate::ImportReport;

/// On-disk Create3D project.
#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    manifest: ProjectManifest,
    assets: AssetDb,
    scene_path: PathBuf,
    scene: SceneDoc,
}

impl Project {
    /// Create a new project directory.
    pub fn create(root: impl AsRef<Path>, name: impl Into<String>) -> ProjectResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("scenes"))?;

        let manifest = ProjectManifest::new(name);
        let scene_path = root.join(&manifest.main_scene);
        let scene = SceneDoc::new();
        scene
            .to_json()
            .map_err(ProjectError::Scene)
            .and_then(|json| {
                fs::write(&scene_path, json)?;
                Ok(())
            })?;

        let assets = AssetDb::open(&root)?;
        let project = Self {
            root,
            manifest,
            assets,
            scene_path,
            scene,
        };
        project.save_manifest()?;
        project.assets.save()?;
        Ok(project)
    }

    /// Open an existing project directory.
    pub fn open(root: impl AsRef<Path>) -> ProjectResult<Self> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("manifest.c3d.toml");
        if !manifest_path.is_file() {
            return Err(ProjectError::NotFound(root.display().to_string()));
        }

        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest: ProjectManifest = toml::from_str(&manifest_text)
            .map_err(|err| ProjectError::Manifest(err.to_string()))?;
        let scene_path = root.join(&manifest.main_scene);
        let scene = if scene_path.is_file() {
            let json = fs::read_to_string(&scene_path)?;
            SceneDoc::from_json(&json)?
        } else {
            SceneDoc::new()
        };

        let assets = AssetDb::open(&root)?;
        Ok(Self {
            root,
            manifest,
            assets,
            scene_path,
            scene,
        })
    }

    /// Returns the project root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Borrow the project manifest.
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    /// Borrow the asset database.
    pub fn assets(&self) -> &AssetDb {
        &self.assets
    }

    /// Mutable access to the asset database.
    pub fn assets_mut(&mut self) -> &mut AssetDb {
        &mut self.assets
    }

    /// Borrow the authoritative scene document.
    pub fn scene(&self) -> &SceneDoc {
        &self.scene
    }

    /// Mutable access to the scene document.
    pub fn scene_mut(&mut self) -> &mut SceneDoc {
        &mut self.scene
    }

    /// Read and decode a mesh asset.
    pub fn mesh_asset(&self, asset_id: AssetId) -> ProjectResult<MeshAssetData> {
        let bytes = self.assets.read_blob(asset_id)?;
        MeshAsset::decode(&bytes).map_err(|err| ProjectError::Mesh(err.to_string()))
    }

    /// Read and decode a material asset.
    pub fn material_asset(&self, asset_id: AssetId) -> ProjectResult<MaterialAssetData> {
        let bytes = self.assets.read_blob(asset_id)?;
        MaterialAsset::decode(&bytes).map_err(|err| ProjectError::Material(err.to_string()))
    }

    /// Read raw texture bytes for an asset id.
    pub fn texture_bytes(&self, asset_id: AssetId) -> ProjectResult<Vec<u8>> {
        Ok(self.assets.read_blob(asset_id)?)
    }

    /// Import a glTF/GLB file into the project and append scene entities.
    pub fn import_gltf(
        &mut self,
        path: impl AsRef<Path>,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<ImportReport> {
        let import = c3d_import_gltf::import_gltf_path(path.as_ref())?;
        self.store_import(&import, ids)
    }

    /// Persist manifest, scene, and asset index.
    pub fn save(&self) -> ProjectResult<()> {
        self.save_manifest()?;
        self.save_scene()?;
        self.assets.save()?;
        Ok(())
    }

    fn store_import(
        &mut self,
        import: &GltfImportResult,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<ImportReport> {
        let mut texture_ids = Vec::new();
        for texture in &import.textures {
            let asset_id = ids.next_asset_id();
            self.assets.insert(
                asset_id,
                AssetKind::Texture,
                texture.name.clone(),
                &texture.bytes,
                Some(texture.mime_type.clone()),
            )?;
            texture_ids.push(asset_id);
        }

        let mut material_ids = Vec::new();
        for material in &import.materials {
            let mut data = material.data.clone();
            if let Some(texture_index) = material.texture_index {
                data.base_color_texture = texture_ids.get(texture_index).copied();
            }
            let bytes = MaterialAsset::encode(&data)
                .map_err(|err| ProjectError::Material(err.to_string()))?;
            let asset_id = ids.next_asset_id();
            self.assets.insert(
                asset_id,
                AssetKind::Material,
                material.name.clone(),
                &bytes,
                Some("application/json".into()),
            )?;
            material_ids.push(asset_id);
        }

        let mut mesh_ids = Vec::new();
        for mesh in &import.meshes {
            let bytes =
                MeshAsset::encode(&mesh.data).map_err(|err| ProjectError::Mesh(err.to_string()))?;
            let asset_id = ids.next_asset_id();
            self.assets.insert(
                asset_id,
                AssetKind::Mesh,
                mesh.name.clone(),
                &bytes,
                Some("application/json".into()),
            )?;
            mesh_ids.push(asset_id);
        }

        let operations = import_result_to_scene_operations(import, &mesh_ids, &material_ids, ids);
        apply_operations(&mut self.scene, &operations)?;

        Ok(ImportReport {
            mesh_assets: mesh_ids,
            material_assets: material_ids,
            texture_assets: texture_ids,
            point_cloud_assets: Vec::new(),
            chunk_assets: Vec::new(),
            entity_count: operations
                .iter()
                .filter(|op| matches!(op, c3d_scene_ops::SceneOperation::CreateEntity { .. }))
                .count(),
        })
    }

    fn save_manifest(&self) -> ProjectResult<()> {
        let text = toml::to_string_pretty(&self.manifest)
            .map_err(|err| ProjectError::Manifest(err.to_string()))?;
        fs::write(self.root.join("manifest.c3d.toml"), text)?;
        Ok(())
    }

    fn save_scene(&self) -> ProjectResult<()> {
        if let Some(parent) = self.scene_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = self.scene.to_json()?;
        fs::write(&self.scene_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::{EntityId, TransactionId};
    use c3d_scene_ops::{SceneOperation, Transaction, TransactionManager};
    use c3d_scene_schema::Name;

    #[test]
    fn create_and_reload_project() {
        let temp = tempfile::tempdir().expect("temp dir");
        {
            let mut project = Project::create(temp.path(), "demo").expect("create project");
            let mut manager = TransactionManager::new(project.scene().clone());
            let entity_id = EntityId::new();
            manager
                .apply(Transaction::new(
                    TransactionId::new(),
                    vec![SceneOperation::CreateEntity {
                        entity_id,
                        parent: None,
                        name: Some(Name::new("Root")),
                        transform: Default::default(),
                        mesh_ref: None,
                        material_binding: None,
                        point_cloud_ref: None,
                    }],
                ))
                .expect("apply transaction");
            *project.scene_mut() = manager.scene().clone();
            project.save().expect("save project");
        }

        let project = Project::open(temp.path()).expect("open project");
        assert_eq!(project.manifest().name, "demo");
        assert_eq!(project.scene().entity_count(), 1);
    }
}
