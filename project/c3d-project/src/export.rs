use std::path::Path;

use c3d_export_gltf::{export_scene_glb, GltfExportReport};
use c3d_export_gsplat::{export_scene_gsplat_ply, GsplatExportReport};
use c3d_export_ply::{export_scene_ply, PlyExportOptions, PlyExportReport};
use c3d_export_usd::{export_scene_usda, UsdExportReport};

use crate::error::{ProjectError, ProjectResult};
use crate::Project;

impl Project {
    /// Export mesh entities from the project scene to a binary GLB snapshot.
    pub fn export_gltf(&self, path: impl AsRef<Path>) -> ProjectResult<GltfExportReport> {
        export_scene_glb(
            self.scene(),
            |asset_id| {
                self.mesh_asset(asset_id)
                    .map_err(|err| c3d_export_gltf::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                self.material_asset(asset_id)
                    .map_err(|err| c3d_export_gltf::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                self.texture_export_data(asset_id)
                    .map_err(|err| c3d_export_gltf::ExportError::Asset(err.to_string()))
            },
            path,
        )
        .map_err(|err| ProjectError::Export(err.to_string()))
    }

    /// Export mesh entities from the project scene to an ASCII USD snapshot.
    pub fn export_usd(&self, path: impl AsRef<Path>) -> ProjectResult<UsdExportReport> {
        export_scene_usda(
            self.scene(),
            |asset_id| {
                self.mesh_asset(asset_id)
                    .map_err(|err| c3d_export_usd::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                self.material_asset(asset_id)
                    .map_err(|err| c3d_export_usd::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let data = self
                    .texture_export_data(asset_id)
                    .map_err(|err| c3d_export_usd::ExportError::Asset(err.to_string()))?;
                Ok(c3d_export_usd::TextureExportData {
                    bytes: data.bytes,
                    mime_type: data.mime_type,
                })
            },
            path,
        )
        .map_err(|err| ProjectError::Export(err.to_string()))
    }

    /// Export point cloud entities from the project scene to a PLY snapshot (binary by default).
    pub fn export_ply(&self, path: impl AsRef<Path>) -> ProjectResult<PlyExportReport> {
        self.export_ply_with_options(path, PlyExportOptions::default())
    }

    /// Export point cloud entities from the project scene with explicit PLY options.
    pub fn export_ply_with_options(
        &self,
        path: impl AsRef<Path>,
        options: PlyExportOptions,
    ) -> ProjectResult<PlyExportReport> {
        use c3d_asset_pointcloud::PointCloudChunkPayload;

        export_scene_ply(
            self.scene(),
            |asset_id| {
                self.point_cloud_asset(asset_id)
                    .map_err(|err| c3d_export_ply::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = self
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| c3d_export_ply::ExportError::Asset(err.to_string()))?;
                PointCloudChunkPayload::from_bytes(&bytes)
                    .map_err(|err| c3d_export_ply::ExportError::Asset(err.to_string()))
            },
            path,
            options,
        )
        .map_err(|err| ProjectError::Export(err.to_string()))
    }

    /// Export Gaussian splat entities from the project scene to an ASCII 3DGS PLY snapshot.
    pub fn export_gsplat_ply(&self, path: impl AsRef<Path>) -> ProjectResult<GsplatExportReport> {
        use c3d_asset_gsplat::GaussianSplatChunkPayload;

        export_scene_gsplat_ply(
            self.scene(),
            |asset_id| {
                self.gaussian_splat_asset(asset_id)
                    .map_err(|err| c3d_export_gsplat::ExportError::Asset(err.to_string()))
            },
            |asset_id| {
                let bytes = self
                    .assets()
                    .read_blob(asset_id)
                    .map_err(|err| c3d_export_gsplat::ExportError::Asset(err.to_string()))?;
                GaussianSplatChunkPayload::from_bytes(&bytes)
                    .map_err(|err| c3d_export_gsplat::ExportError::Asset(err.to_string()))
            },
            path,
        )
        .map_err(|err| ProjectError::Export(err.to_string()))
    }
}
