use c3d_asset_db::AssetKind;
use c3d_asset_pointcloud::{PointCloudAsset, PointCloudAssetData, PointCloudChunkPayload};
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_import_ply::PlyImportResult;
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{Name, PointCloudCropBox, PointCloudRef, Transform};

use crate::error::{ProjectError, ProjectResult};
use crate::import::ImportReport;
use crate::Project;

/// Result of importing a point cloud into the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointCloudImportReport {
    /// Metadata asset id stored in AssetDB.
    pub asset_id: AssetId,
    /// Scene entity referencing the point cloud.
    pub entity_id: EntityId,
    /// Chunk payload assets written to AssetDB.
    pub chunk_assets: Vec<AssetId>,
    /// Total point count across all chunks.
    pub point_count: u64,
}

impl Project {
    /// Import a PLY point cloud file into the project and create a scene entity.
    pub fn import_ply(
        &mut self,
        path: impl AsRef<std::path::Path>,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<PointCloudImportReport> {
        let path = path.as_ref();
        let imported = c3d_import_ply::import_ply_path(path)
            .map_err(|err| ProjectError::import_at_path("PLY point cloud", path, err))?;
        self.store_ply_import(imported, ids)
    }

    /// Import a synthetic point cloud for performance/residency testing.
    pub fn import_synthetic_point_cloud(
        &mut self,
        point_count: usize,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<PointCloudImportReport> {
        let imported = c3d_import_ply::generate_synthetic_point_cloud(point_count, 8_192)
            .map_err(ProjectError::PointCloudImport)?;
        self.store_ply_import(imported, ids)
    }

    /// Read and decode a point cloud metadata asset.
    pub fn point_cloud_asset(&self, asset_id: AssetId) -> ProjectResult<PointCloudAssetData> {
        let bytes = self.assets().read_blob(asset_id)?;
        PointCloudAsset::decode(&bytes).map_err(|err| ProjectError::PointCloud(err.to_string()))
    }

    /// Create a derived point cloud asset by cropping an existing asset.
    pub fn crop_point_cloud(
        &mut self,
        source_asset_id: AssetId,
        crop: PointCloudCropBox,
        ids: &mut UlidGenerator,
        name: impl Into<String>,
    ) -> ProjectResult<AssetId> {
        let source = self.point_cloud_asset(source_asset_id)?;
        let name = name.into();

        let mut chunks = Vec::new();
        let mut records = Vec::new();
        let mut point_count = 0u64;

        for record in &source.chunks {
            let bytes = self.assets().read_blob(record.blob_asset_id)?;
            let payload = PointCloudChunkPayload::from_bytes(&bytes)
                .map_err(|err| ProjectError::PointCloud(err.to_string()))?;
            let cropped = payload.crop(&crop);
            if cropped.point_count() == 0 {
                continue;
            }

            let chunk_bytes = cropped
                .to_bytes()
                .map_err(|err| ProjectError::PointCloud(err.to_string()))?;
            let chunk_id = ids.next_asset_id();
            self.assets_mut().insert(
                chunk_id,
                AssetKind::PointCloudChunk,
                format!("{name}-chunk-{}", records.len()),
                &chunk_bytes,
                Some("application/json".into()),
            )?;

            let (bounds_min, bounds_max) = payload_bounds(&cropped);
            records.push(c3d_asset_pointcloud::PointCloudChunkRecord {
                chunk_id: records.len() as u32,
                bounds_min,
                bounds_max,
                point_count: cropped.point_count() as u32,
                blob_asset_id: chunk_id,
                lod_stride: record.lod_stride,
            });
            point_count += cropped.point_count() as u64;
            chunks.push(chunk_id);
        }

        if records.is_empty() {
            return Err(ProjectError::PointCloud("crop removed all points".into()));
        }

        let (bounds_min, bounds_max) = metadata_bounds(&records);
        let metadata = PointCloudAssetData {
            version: 1,
            point_count,
            bounds_min,
            bounds_max,
            has_rgb: source.has_rgb,
            has_intensity: source.has_intensity,
            has_classification: source.has_classification,
            chunks: records,
        };
        let metadata_bytes = PointCloudAsset::encode(&metadata)
            .map_err(|err| ProjectError::PointCloud(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::PointCloud,
            name,
            &metadata_bytes,
            Some("application/json".into()),
        )?;

        let _ = chunks;
        Ok(asset_id)
    }

    fn store_ply_import(
        &mut self,
        mut imported: PlyImportResult,
        ids: &mut UlidGenerator,
    ) -> ProjectResult<PointCloudImportReport> {
        let mut chunk_assets = Vec::with_capacity(imported.chunks.len());
        for (index, chunk) in imported.chunks.iter().enumerate() {
            let bytes = chunk
                .to_bytes()
                .map_err(|err| ProjectError::PointCloud(err.to_string()))?;
            let chunk_id = ids.next_asset_id();
            self.assets_mut().insert(
                chunk_id,
                AssetKind::PointCloudChunk,
                format!("{}-chunk-{index}", imported.name),
                &bytes,
                Some("application/json".into()),
            )?;
            chunk_assets.push(chunk_id);
            imported.metadata.chunks[index].blob_asset_id = chunk_id;
        }

        let metadata_bytes = PointCloudAsset::encode(&imported.metadata)
            .map_err(|err| ProjectError::PointCloud(err.to_string()))?;
        let asset_id = ids.next_asset_id();
        self.assets_mut().insert(
            asset_id,
            AssetKind::PointCloud,
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
                point_cloud_ref: Some(PointCloudRef::new(asset_id)),
                gaussian_splat_ref: None,
                robot_root: None,
                robot_link: None,
                robot_joint: None,
            }],
        )?;

        Ok(PointCloudImportReport {
            asset_id,
            entity_id,
            chunk_assets,
            point_count: imported.metadata.point_count,
        })
    }
}

impl ImportReport {
    /// Merge a point cloud import into a generic import report.
    pub fn from_point_cloud(report: &PointCloudImportReport) -> Self {
        Self {
            mesh_assets: Vec::new(),
            material_assets: Vec::new(),
            texture_assets: Vec::new(),
            point_cloud_assets: vec![report.asset_id],
            chunk_assets: report.chunk_assets.clone(),
            gaussian_splat_assets: Vec::new(),
            entity_count: 1,
        }
    }
}

fn payload_bounds(payload: &PointCloudChunkPayload) -> ([f32; 3], [f32; 3]) {
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

fn metadata_bounds(
    records: &[c3d_asset_pointcloud::PointCloudChunkRecord],
) -> ([f32; 3], [f32; 3]) {
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
    use c3d_asset_pointcloud::{select_resident_chunks, ResidencyConfig};
    use c3d_core::math::Vec3;

    #[test]
    fn synthetic_import_respects_residency_limit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "pointcloud").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_synthetic_point_cloud(20_000, &mut ids)
            .expect("import synthetic");
        assert!(report.chunk_assets.len() > 1);

        let metadata = project
            .point_cloud_asset(report.asset_id)
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
    fn crop_creates_derived_asset() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "pointcloud").expect("project");
        let mut ids = UlidGenerator::default();
        let report = project
            .import_synthetic_point_cloud(1_000, &mut ids)
            .expect("store");

        let derived = project
            .crop_point_cloud(
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
            .point_cloud_asset(derived)
            .expect("derived metadata");
        assert!(metadata.point_count > 0);
        assert!(metadata.point_count <= report.point_count);
    }
}
