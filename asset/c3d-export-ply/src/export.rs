use std::fs;
use std::path::Path;

use c3d_asset_pointcloud::{PointCloudAssetData, PointCloudChunkPayload};
use c3d_core::math::{Mat4, Vec3};
use c3d_core::{AssetId, EntityId};
use c3d_scene_doc::SceneDoc;
use c3d_scene_schema::Transform;

/// Export failures.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Scene had no exportable point cloud content.
    #[error("scene has no point cloud entities to export")]
    EmptyScene,
    /// Asset lookup failure.
    #[error("asset error: {0}")]
    Asset(String),
}

/// On-disk PLY format for point cloud export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlyExportFormat {
    /// ASCII 1.0 vertices.
    Ascii,
    /// Binary little-endian 1.0 vertices (default).
    #[default]
    BinaryLittleEndian,
}

/// Options controlling PLY snapshot export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlyExportOptions {
    /// Vertex storage format.
    pub format: PlyExportFormat,
}

/// Summary of a PLY export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlyExportReport {
    /// Number of point cloud entities merged into the snapshot.
    pub entity_count: usize,
    /// Total number of points written.
    pub point_count: u64,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export point cloud entities from a scene into a single PLY snapshot.
pub fn export_scene_ply(
    scene: &SceneDoc,
    metadata_loader: impl Fn(AssetId) -> Result<PointCloudAssetData, ExportError>,
    chunk_loader: impl Fn(AssetId) -> Result<PointCloudChunkPayload, ExportError>,
    output: impl AsRef<Path>,
    options: PlyExportOptions,
) -> Result<PlyExportReport, ExportError> {
    let mut merged = MergedPointCloud::default();
    let mut entity_count = 0usize;

    for entity in scene.entities() {
        let Some(point_cloud_ref) = entity.point_cloud_ref.as_ref() else {
            continue;
        };
        let metadata = metadata_loader(point_cloud_ref.asset_id)?;
        let world = entity_world_transform(scene, entity.id);
        let mut wrote_points = false;

        for record in &metadata.chunks {
            let mut payload = chunk_loader(record.blob_asset_id)?;
            if let Some(crop) = point_cloud_ref.crop_filter {
                payload = payload.crop(&crop);
            }
            if payload.point_count() == 0 {
                continue;
            }
            merged.append_payload(&payload, &metadata, world);
            wrote_points = true;
        }

        if wrote_points {
            entity_count += 1;
        }
    }

    if merged.point_count() == 0 {
        return Err(ExportError::EmptyScene);
    }

    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = match options.format {
        PlyExportFormat::Ascii => merged.to_ascii_ply()?,
        PlyExportFormat::BinaryLittleEndian => merged.to_binary_ply()?,
    };
    fs::write(output, body)?;
    let byte_length = fs::metadata(output)?.len();
    Ok(PlyExportReport {
        entity_count,
        point_count: merged.point_count() as u64,
        byte_length,
    })
}

#[derive(Default)]
struct MergedPointCloud {
    positions: Vec<[f32; 3]>,
    colors: Vec<[u8; 3]>,
    intensity: Vec<f32>,
    classification: Vec<u8>,
    has_rgb: bool,
    has_intensity: bool,
    has_classification: bool,
}

impl MergedPointCloud {
    fn point_count(&self) -> usize {
        self.positions.len()
    }

    fn append_payload(
        &mut self,
        payload: &PointCloudChunkPayload,
        metadata: &PointCloudAssetData,
        world: Mat4,
    ) {
        self.has_rgb |= metadata.has_rgb;
        self.has_intensity |= metadata.has_intensity;
        self.has_classification |= metadata.has_classification;

        for (index, position) in payload.positions.iter().enumerate() {
            let local = Vec3::from_array(*position);
            let world_position = world.transform_point3(local);
            self.positions.push(world_position.to_array());

            if metadata.has_rgb {
                let color = payload
                    .colors
                    .get(index)
                    .copied()
                    .unwrap_or([255, 255, 255]);
                self.colors.push(color);
            }
            if metadata.has_intensity {
                let value = payload.intensity.get(index).copied().unwrap_or(0.0);
                self.intensity.push(value);
            }
            if metadata.has_classification {
                let value = payload.classification.get(index).copied().unwrap_or(0);
                self.classification.push(value);
            }
        }
    }

    fn write_header(&self, format_line: &str) -> String {
        let mut header = String::from("ply\n");
        header.push_str(format_line);
        header.push_str(&format!("element vertex {}\n", self.point_count()));
        header.push_str("property float x\nproperty float y\nproperty float z\n");
        if self.has_rgb {
            header.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
        }
        if self.has_intensity {
            header.push_str("property float intensity\n");
        }
        if self.has_classification {
            header.push_str("property uchar classification\n");
        }
        header.push_str("end_header\n");
        header
    }

    fn to_ascii_ply(&self) -> Result<Vec<u8>, ExportError> {
        let header = self.write_header("format ascii 1.0\n");
        let mut body = String::with_capacity(header.len() + self.point_count() * 48);
        body.push_str(&header);
        for index in 0..self.point_count() {
            let position = self.positions[index];
            body.push_str(&format!("{} {} {}", position[0], position[1], position[2]));
            if self.has_rgb {
                let color = self.colors.get(index).copied().unwrap_or([255, 255, 255]);
                body.push_str(&format!(" {} {} {}", color[0], color[1], color[2]));
            }
            if self.has_intensity {
                let value = self.intensity.get(index).copied().unwrap_or(0.0);
                body.push_str(&format!(" {value}"));
            }
            if self.has_classification {
                let value = self.classification.get(index).copied().unwrap_or(0);
                body.push_str(&format!(" {value}"));
            }
            body.push('\n');
        }
        Ok(body.into_bytes())
    }

    fn to_binary_ply(&self) -> Result<Vec<u8>, ExportError> {
        let header = self.write_header("format binary_little_endian 1.0\n");
        let mut body = header.into_bytes();
        let vertex_stride = 12
            + usize::from(self.has_rgb) * 3
            + usize::from(self.has_intensity) * 4
            + usize::from(self.has_classification);
        body.reserve(self.point_count() * vertex_stride);

        for index in 0..self.point_count() {
            let position = self.positions[index];
            for component in position {
                body.extend_from_slice(&component.to_le_bytes());
            }
            if self.has_rgb {
                let color = self.colors.get(index).copied().unwrap_or([255, 255, 255]);
                body.extend_from_slice(&color);
            }
            if self.has_intensity {
                let value = self.intensity.get(index).copied().unwrap_or(0.0);
                body.extend_from_slice(&value.to_le_bytes());
            }
            if self.has_classification {
                let value = self.classification.get(index).copied().unwrap_or(0);
                body.push(value);
            }
        }

        Ok(body)
    }
}

fn entity_world_transform(scene: &SceneDoc, entity_id: EntityId) -> Mat4 {
    let mut chain = Vec::new();
    let mut current = Some(entity_id);
    while let Some(id) = current {
        let Some(entity) = scene.get(id) else {
            break;
        };
        chain.push(entity.transform);
        current = entity.parent;
    }
    chain.reverse();
    chain.iter().fold(Mat4::IDENTITY, |accumulator, transform| {
        accumulator * transform_to_mat4(transform)
    })
}

fn transform_to_mat4(transform: &Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        transform.scale,
        transform.rotation,
        transform.translation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_import_ply::import_ply_bytes;
    use c3d_project::Project;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::PointCloudRef;

    #[test]
    fn exports_point_cloud_scene_sample_and_round_trips() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/point-cloud-scene");
        if !sample.join("manifest.c3d.toml").is_file() {
            return;
        }

        let project = Project::open(&sample).expect("open sample");
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("point-cloud-scene.ply");

        let report = export_scene_ply(
            project.scene(),
            |asset_id| {
                project
                    .point_cloud_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = project
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                PointCloudChunkPayload::from_bytes(&bytes)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            &output,
            PlyExportOptions::default(),
        )
        .expect("export");

        assert!(report.entity_count >= 1);
        assert!(report.point_count > 0);
        let exported = fs::read(&output).expect("read ply");
        assert!(exported.starts_with(b"ply\n"));
        let header = String::from_utf8_lossy(&exported);
        assert!(header.contains("format binary_little_endian 1.0"));

        let reimported = import_ply_bytes(&exported, "round-trip").expect("re-import");
        assert_eq!(reimported.metadata.point_count, report.point_count);
    }

    #[test]
    fn applies_entity_transform_to_exported_points() {
        let metadata_asset_id = AssetId::new();
        let chunk_blob_id = AssetId::new();
        let metadata = PointCloudAssetData {
            version: 1,
            point_count: 1,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [0.0, 0.0, 0.0],
            has_rgb: false,
            has_intensity: false,
            has_classification: false,
            chunks: vec![c3d_asset_pointcloud::PointCloudChunkRecord {
                chunk_id: 0,
                bounds_min: [0.0, 0.0, 0.0],
                bounds_max: [0.0, 0.0, 0.0],
                point_count: 1,
                blob_asset_id: chunk_blob_id,
                lod_stride: 1,
            }],
        };
        let chunk = PointCloudChunkPayload {
            positions: vec![[0.0, 0.0, 0.0]],
            colors: Vec::new(),
            intensity: Vec::new(),
            classification: Vec::new(),
        };

        let entity_id = EntityId::new();
        let mut scene = SceneDoc::new();
        let mut entity = Entity::new(entity_id);
        entity.transform.translation = Vec3::new(2.0, 0.0, 0.0);
        entity.point_cloud_ref = Some(PointCloudRef::new(metadata_asset_id));
        scene.insert_entity(entity, None).expect("insert entity");

        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("transformed.ply");
        let report = export_scene_ply(
            &scene,
            |asset_id| {
                if asset_id == metadata_asset_id {
                    Ok(metadata.clone())
                } else {
                    Err(ExportError::Asset("missing metadata".into()))
                }
            },
            |asset_id| {
                if asset_id == chunk_blob_id {
                    Ok(chunk.clone())
                } else {
                    Err(ExportError::Asset("missing chunk".into()))
                }
            },
            &output,
            PlyExportOptions::default(),
        )
        .expect("export");
        assert_eq!(report.point_count, 1);

        let contents = fs::read(&output).expect("read ply");
        let reimported = import_ply_bytes(&contents, "transformed").expect("re-import");
        assert_eq!(reimported.metadata.point_count, 1);
        assert!((reimported.chunks[0].positions[0][0] - 2.0).abs() < 1e-5);
    }
}
