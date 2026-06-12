use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Mesh asset error type.
#[derive(Debug, Error)]
pub enum MeshAssetError {
    /// JSON serialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Mesh data is invalid.
    #[error("invalid mesh: {0}")]
    Invalid(String),
}

/// Result alias for mesh asset operations.
pub type MeshAssetResult<T> = Result<T, MeshAssetError>;

/// CPU-side mesh asset payload stored in AssetDB blobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshAssetData {
    /// Mesh blob schema version.
    pub version: u32,
    /// Vertex positions in object space.
    pub positions: Vec<[f32; 3]>,
    /// Optional per-vertex normals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normals: Vec<[f32; 3]>,
    /// Optional per-vertex UV coordinates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uvs: Vec<[f32; 2]>,
    /// Optional per-vertex tangents as XYZ + handedness.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tangents: Vec<[f32; 4]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl Default for MeshAssetData {
    fn default() -> Self {
        Self {
            version: 1,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl MeshAssetData {
    /// Validate mesh topology and attribute lengths.
    pub fn validate(&self) -> MeshAssetResult<()> {
        if self.positions.is_empty() {
            return Err(MeshAssetError::Invalid("mesh has no positions".into()));
        }
        if !self.normals.is_empty() && self.normals.len() != self.positions.len() {
            return Err(MeshAssetError::Invalid(
                "normal count must match position count".into(),
            ));
        }
        if !self.uvs.is_empty() && self.uvs.len() != self.positions.len() {
            return Err(MeshAssetError::Invalid(
                "uv count must match position count".into(),
            ));
        }
        if !self.tangents.is_empty() && self.tangents.len() != self.positions.len() {
            return Err(MeshAssetError::Invalid(
                "tangent count must match position count".into(),
            ));
        }
        if !self.indices.len().is_multiple_of(3) {
            return Err(MeshAssetError::Invalid(
                "index count must be a multiple of three".into(),
            ));
        }
        let vertex_count = self.positions.len() as u32;
        for (face, triangle) in self.indices.chunks_exact(3).enumerate() {
            if triangle[0] == triangle[1]
                || triangle[1] == triangle[2]
                || triangle[2] == triangle[0]
            {
                return Err(MeshAssetError::Invalid(format!(
                    "face {face} is degenerate"
                )));
            }
            for index in triangle {
                if *index >= vertex_count {
                    return Err(MeshAssetError::Invalid(format!(
                        "face {face} references out-of-range index {index}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Serialize to JSON bytes for blob storage.
    pub fn to_bytes(&self) -> MeshAssetResult<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|err| MeshAssetError::Serialization(err.to_string()))
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> MeshAssetResult<Self> {
        let mesh: Self = serde_json::from_slice(bytes)
            .map_err(|err| MeshAssetError::Serialization(err.to_string()))?;
        mesh.validate()?;
        Ok(mesh)
    }
}

/// Helper for reading and writing mesh assets.
#[derive(Debug, Clone, Default)]
pub struct MeshAsset;

impl MeshAsset {
    /// Decode mesh asset bytes.
    pub fn decode(bytes: &[u8]) -> MeshAssetResult<MeshAssetData> {
        MeshAssetData::from_bytes(bytes)
    }

    /// Encode mesh asset bytes.
    pub fn encode(mesh: &MeshAssetData) -> MeshAssetResult<Vec<u8>> {
        mesh.to_bytes()
    }
}

impl MeshAssetData {
    /// Returns axis-aligned bounds in local object space.
    pub fn local_bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        (min[0].is_finite() && max[0].is_finite()).then_some((min, max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cube_round_trip() {
        let mesh = MeshAssetData {
            version: 1,
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Vec::new(),
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let bytes = MeshAsset::encode(&mesh).expect("encode mesh");
        let restored = MeshAsset::decode(&bytes).expect("decode mesh");
        assert_eq!(mesh, restored);
    }

    #[test]
    fn rejects_out_of_range_index_in_validate() {
        let mesh = MeshAssetData {
            version: 1,
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: vec![0, 1, 99],
        };
        assert!(mesh.validate().is_err());
    }
}
