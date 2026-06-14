use std::fs;
use std::path::Path;

use c3d_asset_gsplat::{GaussianSplatAssetData, GaussianSplatChunkPayload};
use c3d_core::math::{Mat4, Quat, Vec3};
use c3d_core::{AssetId, EntityId};
use c3d_scene_doc::SceneDoc;
use c3d_scene_schema::Transform;

const SH_C0: f32 = 0.282_095;

/// Export failures.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Scene had no exportable Gaussian splat content.
    #[error("scene has no gaussian splat entities to export")]
    EmptyScene,
    /// Asset lookup failure.
    #[error("asset error: {0}")]
    Asset(String),
}

/// Summary of a 3DGS PLY export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GsplatExportReport {
    /// Number of Gaussian splat entities merged into the snapshot.
    pub entity_count: usize,
    /// Total number of splats written.
    pub splat_count: u64,
    /// Output file size in bytes.
    pub byte_length: u64,
}

/// Export Gaussian splat entities from a scene into a single ASCII 3DGS PLY snapshot.
pub fn export_scene_gsplat_ply(
    scene: &SceneDoc,
    metadata_loader: impl Fn(AssetId) -> Result<GaussianSplatAssetData, ExportError>,
    chunk_loader: impl Fn(AssetId) -> Result<GaussianSplatChunkPayload, ExportError>,
    output: impl AsRef<Path>,
) -> Result<GsplatExportReport, ExportError> {
    let mut merged = MergedGaussianSplats::default();
    let mut entity_count = 0usize;

    for entity in scene.entities() {
        let Some(gaussian_splat_ref) = entity.gaussian_splat_ref.as_ref() else {
            continue;
        };
        let metadata = metadata_loader(gaussian_splat_ref.asset_id)?;
        let world = entity_world_transform(scene, entity.id);
        let mut wrote_splats = false;

        for record in &metadata.chunks {
            let mut payload = chunk_loader(record.blob_asset_id)?;
            if let Some(crop) = gaussian_splat_ref.crop_filter {
                payload = payload.crop(&crop);
            }
            if payload.splat_count() == 0 {
                continue;
            }
            merged.append_payload(
                &payload,
                world,
                gaussian_splat_ref.opacity_scale,
                gaussian_splat_ref.size_scale,
            );
            wrote_splats = true;
        }

        if wrote_splats {
            entity_count += 1;
        }
    }

    if merged.splat_count() == 0 {
        return Err(ExportError::EmptyScene);
    }

    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = merged.to_ascii_gsplat_ply()?;
    fs::write(output, body)?;
    let byte_length = fs::metadata(output)?.len();
    Ok(GsplatExportReport {
        entity_count,
        splat_count: merged.splat_count() as u64,
        byte_length,
    })
}

#[derive(Default)]
struct MergedGaussianSplats {
    positions: Vec<[f32; 3]>,
    f_dc: Vec<[f32; 3]>,
    opacities: Vec<f32>,
    scales: Vec<[f32; 3]>,
    rotations: Vec<[f32; 4]>,
}

impl MergedGaussianSplats {
    fn splat_count(&self) -> usize {
        self.positions.len()
    }

    fn append_payload(
        &mut self,
        payload: &GaussianSplatChunkPayload,
        world: Mat4,
        opacity_scale: f32,
        size_scale: f32,
    ) {
        let (world_scale, world_rotation, _) = world.to_scale_rotation_translation();
        for index in 0..payload.splat_count() {
            let local_position = Vec3::from_array(payload.positions[index]);
            let world_position = world.transform_point3(local_position);

            let local_rotation = payload
                .rotations
                .get(index)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let local_quat = Quat::from_xyzw(
                local_rotation[0],
                local_rotation[1],
                local_rotation[2],
                local_rotation[3],
            );
            let world_quat = (world_rotation * local_quat).normalize();

            let local_scale = payload
                .scales
                .get(index)
                .copied()
                .unwrap_or([0.05, 0.05, 0.05]);
            let world_scale_linear = [
                local_scale[0] * world_scale.x * size_scale,
                local_scale[1] * world_scale.y * size_scale,
                local_scale[2] * world_scale.z * size_scale,
            ];

            let opacity = payload.opacities.get(index).copied().unwrap_or(1.0) * opacity_scale;
            let color = payload
                .colors
                .get(index)
                .copied()
                .unwrap_or([1.0, 1.0, 1.0]);

            self.positions.push(world_position.to_array());
            self.f_dc.push(rgb_to_sh_dc(color));
            self.opacities.push(logit(opacity.clamp(1e-6, 1.0 - 1e-6)));
            self.scales.push([
                world_scale_linear[0].ln(),
                world_scale_linear[1].ln(),
                world_scale_linear[2].ln(),
            ]);
            self.rotations
                .push([world_quat.x, world_quat.y, world_quat.z, world_quat.w]);
        }
    }

    fn to_ascii_gsplat_ply(&self) -> Result<String, ExportError> {
        let mut header = String::from("ply\nformat ascii 1.0\n");
        header.push_str(&format!("element vertex {}\n", self.splat_count()));
        header.push_str("property float x\nproperty float y\nproperty float z\n");
        header.push_str("property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\n");
        header.push_str("property float opacity\n");
        header.push_str("property float scale_0\nproperty float scale_1\nproperty float scale_2\n");
        header.push_str("property float rot_0\nproperty float rot_1\nproperty float rot_2\nproperty float rot_3\n");
        header.push_str("end_header\n");

        let mut body = String::with_capacity(header.len() + self.splat_count() * 96);
        body.push_str(&header);
        for index in 0..self.splat_count() {
            let position = self.positions[index];
            let sh = self.f_dc[index];
            let opacity = self.opacities[index];
            let scale = self.scales[index];
            let rotation = self.rotations[index];
            body.push_str(&format!(
                "{} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
                position[0],
                position[1],
                position[2],
                sh[0],
                sh[1],
                sh[2],
                opacity,
                scale[0],
                scale[1],
                scale[2],
                rotation[0],
                rotation[1],
                rotation[2],
                rotation[3],
            ));
        }
        Ok(body)
    }
}

fn rgb_to_sh_dc(rgb: [f32; 3]) -> [f32; 3] {
    [
        (rgb[0] - 0.5) / SH_C0,
        (rgb[1] - 0.5) / SH_C0,
        (rgb[2] - 0.5) / SH_C0,
    ]
}

fn logit(probability: f32) -> f32 {
    (probability / (1.0 - probability)).ln()
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
    use c3d_import_gsplat::{import_gsplat_ply_bytes, looks_like_gsplat_ply};
    use c3d_project::Project;
    use c3d_scene_doc::Entity;
    use c3d_scene_schema::GaussianSplatRef;

    #[test]
    fn exports_gaussian_splat_scene_sample_and_round_trips() {
        let sample =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gaussian-splat-scene");
        if !sample.join("manifest.c3d.toml").is_file() {
            return;
        }

        let project = Project::open(&sample).expect("open sample");
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("gaussian-splat-scene.ply");

        let report = export_scene_gsplat_ply(
            project.scene(),
            |asset_id| {
                project
                    .gaussian_splat_asset(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = project
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| ExportError::Asset(err.to_string()))?;
                GaussianSplatChunkPayload::from_bytes(&bytes)
                    .map_err(|err| ExportError::Asset(err.to_string()))
            },
            &output,
        )
        .expect("export");

        assert!(report.entity_count >= 1);
        assert!(report.splat_count > 0);
        let exported = fs::read(&output).expect("read ply");
        assert!(looks_like_gsplat_ply(&exported));

        let reimported =
            import_gsplat_ply_bytes(&exported, "round-trip").expect("re-import gsplat");
        assert_eq!(reimported.metadata.splat_count, report.splat_count);
    }

    #[test]
    fn applies_entity_transform_to_exported_splats() {
        let metadata_asset_id = AssetId::new();
        let chunk_blob_id = AssetId::new();
        let metadata = GaussianSplatAssetData {
            version: 1,
            splat_count: 1,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [0.0, 0.0, 0.0],
            sh_degree: 0,
            chunks: vec![c3d_asset_gsplat::GaussianSplatChunkRecord {
                chunk_id: 0,
                bounds_min: [0.0, 0.0, 0.0],
                bounds_max: [0.0, 0.0, 0.0],
                splat_count: 1,
                blob_asset_id: chunk_blob_id,
                lod_stride: 1,
            }],
        };
        let chunk = GaussianSplatChunkPayload {
            positions: vec![[0.0, 0.0, 0.0]],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]],
            scales: vec![[1.0, 1.0, 1.0]],
            opacities: vec![1.0],
            colors: vec![[1.0, 0.0, 0.0]],
        };

        let entity_id = EntityId::new();
        let mut scene = SceneDoc::new();
        let mut entity = Entity::new(entity_id);
        entity.transform.translation = Vec3::new(2.0, 0.0, 0.0);
        entity.gaussian_splat_ref = Some(GaussianSplatRef::new(metadata_asset_id));
        scene.insert_entity(entity, None).expect("insert entity");

        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("transformed.ply");
        let report = export_scene_gsplat_ply(
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
        )
        .expect("export");
        assert_eq!(report.splat_count, 1);

        let contents = fs::read_to_string(&output).expect("read ply");
        assert!(contents.contains("2 0 0"));
    }
}
