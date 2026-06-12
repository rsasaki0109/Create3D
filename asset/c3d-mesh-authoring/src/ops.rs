use c3d_asset_mesh::MeshAssetData;

use crate::processing::compute_normals;
use crate::topology::{AuthoringMesh, TopologyError};

/// Subdivide every triangle into four triangles using edge midpoints.
pub fn subdivide_triangles(mesh: &mut MeshAssetData) -> Result<(), TopologyError> {
    AuthoringMesh::from_render_mesh(mesh.clone())?;

    let mut next_index = mesh.positions.len() as u32;
    let mut edge_midpoints: std::collections::HashMap<(u32, u32), u32> =
        std::collections::HashMap::new();
    let mut new_indices = Vec::with_capacity(mesh.indices.len() * 4);

    let midpoint = |mesh: &mut MeshAssetData,
                    edge_midpoints: &mut std::collections::HashMap<(u32, u32), u32>,
                    next_index: &mut u32,
                    a: u32,
                    b: u32|
     -> u32 {
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(index) = edge_midpoints.get(&key) {
            return *index;
        }
        let pa = mesh.positions[a as usize];
        let pb = mesh.positions[b as usize];
        let uv_a = mesh.uvs.get(a as usize).copied().unwrap_or([0.0, 0.0]);
        let uv_b = mesh.uvs.get(b as usize).copied().unwrap_or([0.0, 0.0]);
        let index = *next_index;
        *next_index += 1;
        mesh.positions.push([
            (pa[0] + pb[0]) * 0.5,
            (pa[1] + pb[1]) * 0.5,
            (pa[2] + pb[2]) * 0.5,
        ]);
        if !mesh.uvs.is_empty() {
            mesh.uvs
                .push([(uv_a[0] + uv_b[0]) * 0.5, (uv_a[1] + uv_b[1]) * 0.5]);
        }
        edge_midpoints.insert(key, index);
        index
    };

    let triangles: Vec<[u32; 3]> = mesh
        .indices
        .chunks_exact(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect();
    for [i0, i1, i2] in triangles {
        let m01 = midpoint(mesh, &mut edge_midpoints, &mut next_index, i0, i1);
        let m12 = midpoint(mesh, &mut edge_midpoints, &mut next_index, i1, i2);
        let m20 = midpoint(mesh, &mut edge_midpoints, &mut next_index, i2, i0);
        new_indices.extend([i0, m01, m20, i1, m12, m01, i2, m20, m12, m01, m12, m20]);
    }

    mesh.indices = new_indices;
    mesh.normals.clear();
    mesh.tangents.clear();
    compute_normals(mesh);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::unit_cube;

    #[test]
    fn subdivide_quadruples_triangle_count() {
        let mut mesh = unit_cube();
        let before = mesh.indices.len() / 3;
        subdivide_triangles(&mut mesh).expect("subdivided mesh valid");
        assert_eq!(mesh.indices.len() / 3, before * 4);
        mesh.validate().expect("subdivided mesh valid");
    }
}
