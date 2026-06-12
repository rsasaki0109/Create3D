use c3d_core::AssetId;

/// Summary of assets and entities created by an import operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    /// Mesh assets inserted into AssetDB.
    pub mesh_assets: Vec<AssetId>,
    /// Material assets inserted into AssetDB.
    pub material_assets: Vec<AssetId>,
    /// Texture assets inserted into AssetDB.
    pub texture_assets: Vec<AssetId>,
    /// Point cloud metadata assets inserted into AssetDB.
    pub point_cloud_assets: Vec<AssetId>,
    /// Point cloud chunk payload assets inserted into AssetDB.
    pub chunk_assets: Vec<AssetId>,
    /// Gaussian splat metadata assets inserted into AssetDB.
    pub gaussian_splat_assets: Vec<AssetId>,
    /// Number of scene entities created.
    pub entity_count: usize,
}

impl ImportReport {
    /// Empty import report.
    pub fn empty() -> Self {
        Self {
            mesh_assets: Vec::new(),
            material_assets: Vec::new(),
            texture_assets: Vec::new(),
            point_cloud_assets: Vec::new(),
            chunk_assets: Vec::new(),
            gaussian_splat_assets: Vec::new(),
            entity_count: 0,
        }
    }
}
