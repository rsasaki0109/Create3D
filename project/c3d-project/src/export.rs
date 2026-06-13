use std::path::Path;

use c3d_export_gltf::{export_scene_glb, GltfExportReport};

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
            path,
        )
        .map_err(|err| ProjectError::Export(err.to_string()))
    }
}
