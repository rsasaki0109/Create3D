use c3d_asset_db::AssetKind;
use c3d_asset_gsplat::{GaussianSplatAsset, GaussianSplatAssetData, GaussianSplatChunkPayload};
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_import_gsplat::GsplatImportResult;
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{GaussianSplatRef, Name, PointCloudCropBox, Transform};

use crate::error::{ProjectError, ProjectResult};
use crate::import::ImportReport;
use crate::Project;

/// Result of importing a Gaussian splat cloud into the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaussianSplatImportReport {
    /// Metadata asset id stored in AssetDB.
    pub asset_id: AssetId,
    /// Scene entity referencing the splat asset.
    pub entity_id: EntityId,
    /// Chunk payload assets written to AssetDB.
    pub chunk_assets: Vec<AssetId>,
    /// Total splat count across all chunks.
    pub splat_count: u64,
}

impl Project {
    /// Import a 3D Gaussian splat PLY file into the project and create a scene entity.
    pub fn import_gsplat_ply(
        &mut self,
        path: impl AsRef<std::path::Path>,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<GaussianSplatImportReport> {
        let path = path.as_ref();
        let imported = c3d_import_gsplat::import_gsplat_ply_path(path)
            .map_err(|err| ProjectError::import_at_path("3DGS PLY", path, err))?;
        self.store_gsplat_import(imported, ids)
    }

    /// Import a synthetic Gaussian splat cloud for performance/residency testing.
    pub fn import_synthetic_gaussian_splats(
        &mut self,
        splat_count: usize,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<GaussianSplatImportReport> {
        let imported = c3d_import_gsplat::generate_synthetic_gaussian_splats(splat_count, 4_096)
            .map_err(ProjectError::GaussianSplatImport)?;
        self.store_gsplat_import(imported, ids)
    }

    /// Read and decode a Gaussian splat metadata asset.
    pub fn gaussian_splat_asset(&self, asset_id: AssetId) -> ProjectResult<GaussianSplatAssetData> {
        let bytes = self.assets().read_blob(asset_id)?;
        GaussianSplatAsset::decode(&bytes)
            .map_err(|err| ProjectError::GaussianSplat(err.to_string()))
    }

    /// Create a derived Gaussian splat asset by cropping an existing asset.
    pub fn crop_gaussian_splat(
        &mut self,
        source_asset_id: AssetId,
        crop: PointCloudCropBox,
        ids: &mut UlidGenerator,
        name: impl Into<String>,
    ) -> ProjectResult<AssetId> {
        let source = self.gaussian_splat_asset(source_asset_id)?;
        let name = name.into();

        let mut records = Vec::new();
        let mut point_count = 0u64;

        for record in &source.chunks {
            let bytes = self.assets().read_blob(record.blob_asset_id)?;
            let payload = GaussianSplatChunkPayload::from_bytes(&bytes)
                .map_err(|err| ProjectError::GaussianSplat(err.to_string()))?;
            let cropped = payload.crop(&crop);
            if cropped.splat_count() == 0 {
                continue;
            }

            let chunk_bytes = cropped
                .to_bytes()
                .map_err(|err| ProjectError::GaussianSplat(err.to_string()))?;
            let chunk_id = ids.next_asset_id();
            self.assets_mut().insert(
                chunk_id,
                AssetKind::GaussianSplatChunk,
                format!("{name}-chunk-{}", records.len()),
                &chunk_bytes,
                Some("application/json".into()),
            )?;

            let (bounds_min, bounds_max) = payload_bounds(&cropped);
            records.push(c3d_asset_gsplat::GaussianSplatChunkRecord {
                chunk_id: records.len() as u32,
                bounds_min,
                bounds_max,
                splat_count: cropped.splat_count() as u32,
                blob_asset_id: chunk_id,
                lod_stride: record.lod_stride,
            });
            point_count += cropped.splat_count() as u64;
        }

        if records.is_empty() {
            return Err(ProjectError::GaussianSplat(
                "crop removed all splats".into(),
            ));
        }

        let (bounds_min, bounds_max) = metadata_bounds(&records);
        let metadata = GaussianSplatAssetData {
            version: 1,
            splat_count: point_count,
            bounds_min,
            bounds_max,
            sh_degree: source.sh_degree,
            chunks: records,
        };
        let metadata_bytes = GaussianSplatAsset::encode(&metadata)
            .map_err(|err| ProjectError::GaussianSplat(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::GaussianSplat,
            name,
            &metadata_bytes,
            Some("application/json".into()),
        )?;
        Ok(asset_id)
    }

    pub(crate) fn store_gsplat_import(
        &mut self,
        mut imported: GsplatImportResult,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<GaussianSplatImportReport> {
        let mut chunk_assets = Vec::with_capacity(imported.chunks.len());
        for (index, chunk) in imported.chunks.iter().enumerate() {
            let bytes = chunk
                .to_bytes()
                .map_err(|err| ProjectError::GaussianSplat(err.to_string()))?;
            let chunk_id = ids.next_asset_id();
            self.assets_mut().insert(
                chunk_id,
                AssetKind::GaussianSplatChunk,
                format!("{}-chunk-{index}", imported.name),
                &bytes,
                Some("application/json".into()),
            )?;
            chunk_assets.push(chunk_id);
            imported.metadata.chunks[index].blob_asset_id = chunk_id;
        }

        let metadata_bytes = GaussianSplatAsset::encode(&imported.metadata)
            .map_err(|err| ProjectError::GaussianSplat(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::GaussianSplat,
            imported.name.clone(),
            &metadata_bytes,
            Some("application/json".into()),
        )?;

        let entity_id = ids.next_entity_id();
        apply_operations(
            self.scene_mut(),
            &[SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new(imported.name)),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: Some(GaussianSplatRef::new(asset_id)),
                robot_root: None,
                robot_link: None,
                robot_joint: None,
            }],
        )?;

        Ok(GaussianSplatImportReport {
            asset_id,
            entity_id,
            chunk_assets,
            splat_count: imported.metadata.splat_count,
        })
    }
}

impl ImportReport {
    /// Merge a Gaussian splat import into a generic import report.
    pub fn from_gaussian_splat(report: &GaussianSplatImportReport) -> Self {
        Self {
            mesh_assets: Vec::new(),
            material_assets: Vec::new(),
            texture_assets: Vec::new(),
            point_cloud_assets: Vec::new(),
            chunk_assets: report.chunk_assets.clone(),
            gaussian_splat_assets: vec![report.asset_id],
            entity_count: 1,
        }
    }
}

fn payload_bounds(payload: &GaussianSplatChunkPayload) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &payload.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (min, max)
}

fn metadata_bounds(records: &[c3d_asset_gsplat::GaussianSplatChunkRecord]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for record in records {
        for axis in 0..3 {
            min[axis] = min[axis].min(record.bounds_min[axis]);
            max[axis] = max[axis].max(record.bounds_max[axis]);
        }
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_asset_gsplat::{select_resident_chunks, ResidencyConfig};
    use c3d_core::math::Vec3;

    #[test]
    fn synthetic_import_respects_residency_limit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "gsplat").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_synthetic_gaussian_splats(10_000, &mut ids)
            .expect("import synthetic");
        assert!(report.chunk_assets.len() > 1);

        let metadata = project
            .gaussian_splat_asset(report.asset_id)
            .expect("metadata");
        let selected = select_resident_chunks(
            &metadata,
            Vec3::ZERO,
            ResidencyConfig {
                max_resident_chunks: 4,
                ..ResidencyConfig::default()
            },
        );
        assert!(selected.len() < metadata.chunks.len());
    }

    #[test]
    fn project_save_reload_preserves_gsplat_entity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "gsplat").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_synthetic_gaussian_splats(500, &mut ids)
            .expect("import synthetic");
        project.save().expect("save");

        let loaded = Project::open(temp.path()).expect("reload");
        let entity = loaded
            .scene()
            .get(report.entity_id)
            .expect("entity restored");
        let gaussian_splat_ref = entity
            .gaussian_splat_ref
            .as_ref()
            .expect("gaussian splat ref restored");
        assert_eq!(gaussian_splat_ref.asset_id, report.asset_id);
        let metadata = loaded
            .gaussian_splat_asset(report.asset_id)
            .expect("metadata restored");
        assert_eq!(metadata.splat_count, report.splat_count);
    }

    #[test]
    fn crop_creates_derived_asset() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "gsplat").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_synthetic_gaussian_splats(1_000, &mut ids)
            .expect("store");

        let derived = project
            .crop_gaussian_splat(
                report.asset_id,
                PointCloudCropBox {
                    min: [-1.0, -1.0, -1.0],
                    max: [1.0, 1.0, 1.0],
                },
                &mut ids,
                "cropped",
            )
            .expect("crop");
        let metadata = project
            .gaussian_splat_asset(derived)
            .expect("derived metadata");
        assert!(metadata.splat_count > 0);
        assert!(metadata.splat_count <= report.splat_count);
    }
}
