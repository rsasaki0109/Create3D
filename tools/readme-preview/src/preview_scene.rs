use c3d_asset_db::{AssetDb, AssetKind};
use c3d_asset_pointcloud::PointCloudAsset;
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_import_ply::PlyImportResult;
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::{apply_operations, SceneOperation};
use c3d_scene_schema::{Name, PointCloudRef, Transform};

/// Labels shown in the composed editor chrome.
pub struct PreviewLabels {
    /// Entity name in the hierarchy panel.
    pub entity_name: String,
    /// Formatted point count for the top bar and inspector.
    pub point_count: String,
}

pub fn build_preview_scene(assets: &mut AssetDb) -> (SceneDoc, PreviewLabels) {
    let imported =
        c3d_import_ply::generate_preview_site_scan().expect("preview site scan point cloud");
    let mut ids = UlidGenerator::new();
    let (entity_id, asset_id, point_count) = store_point_cloud_import(assets, imported, &mut ids);

    let mut scene = SceneDoc::new();
    apply_operations(
        &mut scene,
        &[SceneOperation::CreateEntity {
            entity_id,
            parent: None,
            name: Some(Name::new("Site Scan")),
            transform: Transform::IDENTITY,
            mesh_ref: None,
            material_binding: None,
            point_cloud_ref: Some(PointCloudRef::new(asset_id)),
            gaussian_splat_ref: None,
        }],
    )
    .expect("create preview entity");

    (
        scene,
        PreviewLabels {
            entity_name: "Site Scan".into(),
            point_count: format_points(point_count),
        },
    )
}

fn store_point_cloud_import(
    assets: &mut AssetDb,
    mut imported: PlyImportResult,
    ids: &mut UlidGenerator,
) -> (EntityId, AssetId, u64) {
    for (index, chunk) in imported.chunks.iter().enumerate() {
        let bytes = chunk.to_bytes().expect("encode point cloud chunk");
        let chunk_id = ids.next_asset_id();
        assets
            .insert(
                chunk_id,
                AssetKind::PointCloudChunk,
                format!("{}-chunk-{index}", imported.name),
                &bytes,
                Some("application/json".into()),
            )
            .expect("store chunk asset");
        imported.metadata.chunks[index].blob_asset_id = chunk_id;
    }

    let metadata_bytes = PointCloudAsset::encode(&imported.metadata).expect("encode metadata");
    let asset_id = ids.next_asset_id();
    assets
        .insert(
            asset_id,
            AssetKind::PointCloud,
            imported.name.clone(),
            &metadata_bytes,
            Some("application/json".into()),
        )
        .expect("store point cloud asset");

    let entity_id = ids.next_entity_id();
    (entity_id, asset_id, imported.metadata.point_count)
}

fn format_points(point_count: u64) -> String {
    let digits = point_count.to_string();
    let mut formatted = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }
    formatted.chars().rev().collect()
}
