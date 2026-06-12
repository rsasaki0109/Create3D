use std::collections::HashMap;

use c3d_asset_mesh::MeshAssetData;
use thiserror::Error;

/// Topology validation failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TopologyError {
    /// Generic invalid topology message.
    #[error("invalid topology: {0}")]
    Invalid(String),
}

/// Lightweight triangle topology used for validation and edit ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriangleTopology {
    edge_faces: HashMap<(u32, u32), Vec<u32>>,
    face_count: u32,
}

impl TriangleTopology {
    /// Build topology metadata from a triangle mesh.
    pub fn from_mesh(mesh: &MeshAssetData) -> Result<Self, TopologyError> {
        mesh.validate()
            .map_err(|err| TopologyError::Invalid(err.to_string()))?;
        validate_indices(mesh)?;
        let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        let face_count = (mesh.indices.len() / 3) as u32;
        for (face, triangle) in mesh.indices.chunks_exact(3).enumerate() {
            for (a, b) in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                let edge = undirected_edge(a, b);
                edge_faces.entry(edge).or_default().push(face as u32);
            }
        }
        Ok(Self {
            edge_faces,
            face_count,
        })
    }

    /// Returns the number of triangle faces.
    pub fn face_count(&self) -> u32 {
        self.face_count
    }

    /// Validate manifold triangle topology.
    pub fn validate(&self) -> Result<(), TopologyError> {
        for (edge, faces) in &self.edge_faces {
            if faces.len() > 2 {
                return Err(TopologyError::Invalid(format!(
                    "edge {:?} belongs to more than two faces",
                    edge
                )));
            }
        }
        Ok(())
    }
}

/// Half-edge prototype mesh backed by triangle topology metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoringMesh {
    /// Render-ready mesh payload.
    pub mesh: MeshAssetData,
    /// Derived topology metadata.
    pub topology: TriangleTopology,
}

impl AuthoringMesh {
    /// Create an authoring mesh from render mesh data.
    pub fn from_render_mesh(mesh: MeshAssetData) -> Result<Self, TopologyError> {
        let topology = TriangleTopology::from_mesh(&mesh)?;
        topology.validate()?;
        Ok(Self { mesh, topology })
    }

    /// Validate positions, indices, and topology together.
    pub fn validate(&self) -> Result<(), TopologyError> {
        self.mesh
            .validate()
            .map_err(|err| TopologyError::Invalid(err.to_string()))?;
        validate_indices(&self.mesh)?;
        self.topology.validate()
    }

    /// Export the render mesh payload.
    pub fn into_render_mesh(self) -> MeshAssetData {
        self.mesh
    }
}

fn validate_indices(mesh: &MeshAssetData) -> Result<(), TopologyError> {
    let vertex_count = mesh.positions.len() as u32;
    for (face, triangle) in mesh.indices.chunks_exact(3).enumerate() {
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(TopologyError::Invalid(format!("face {face} is degenerate")));
        }
        for index in triangle {
            if *index >= vertex_count {
                return Err(TopologyError::Invalid(format!(
                    "face {face} references out-of-range index {index}"
                )));
            }
        }
        let area = triangle_area(
            mesh.positions[triangle[0] as usize],
            mesh.positions[triangle[1] as usize],
            mesh.positions[triangle[2] as usize],
        );
        if area <= f32::EPSILON {
            return Err(TopologyError::Invalid(format!("face {face} has zero area")));
        }
    }
    Ok(())
}

fn undirected_edge(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn triangle_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::unit_cube;

    #[test]
    fn cube_topology_is_valid() {
        let mesh = unit_cube();
        let authoring = AuthoringMesh::from_render_mesh(mesh).expect("authoring mesh");
        authoring.validate().expect("valid topology");
    }

    #[test]
    fn rejects_out_of_range_index() {
        let mesh = MeshAssetData {
            version: 1,
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: vec![0, 1, 99],
            tangents: Vec::new(),
        };
        let err = AuthoringMesh::from_render_mesh(mesh).expect_err("invalid mesh");
        assert!(matches!(err, TopologyError::Invalid(_)));
    }
}
