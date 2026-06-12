use c3d_asset_mesh::MeshAssetData;

/// Recompute per-vertex normals from triangle faces.
pub fn compute_normals(mesh: &mut MeshAssetData) {
    let mut normals = vec![[0.0_f32; 3]; mesh.positions.len()];
    for triangle in mesh.indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;
        let p0 = mesh.positions[i0];
        let p1 = mesh.positions[i1];
        let p2 = mesh.positions[i2];
        let edge_a = sub(p1, p0);
        let edge_b = sub(p2, p0);
        let face_normal = cross(edge_a, edge_b);
        for index in [i0, i1, i2] {
            normals[index] = add(normals[index], face_normal);
        }
    }

    for normal in &mut normals {
        *normal = normalize(*normal);
    }
    mesh.normals = normals;
}

/// Recompute per-vertex tangents from positions, normals, and UVs.
pub fn compute_tangents(mesh: &mut MeshAssetData) {
    if mesh.uvs.len() != mesh.positions.len() {
        mesh.tangents.clear();
        return;
    }
    if mesh.normals.len() != mesh.positions.len() {
        compute_normals(mesh);
    }

    let mut tan1 = vec![[0.0_f32; 3]; mesh.positions.len()];
    let mut tan2 = vec![[0.0_f32; 3]; mesh.positions.len()];

    for triangle in mesh.indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;
        let p0 = mesh.positions[i0];
        let p1 = mesh.positions[i1];
        let p2 = mesh.positions[i2];
        let uv0 = mesh.uvs[i0];
        let uv1 = mesh.uvs[i1];
        let uv2 = mesh.uvs[i2];

        let edge1 = sub(p1, p0);
        let edge2 = sub(p2, p0);
        let delta_uv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
        let delta_uv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];
        let denom = delta_uv1[0] * delta_uv2[1] - delta_uv2[0] * delta_uv1[1];
        if denom.abs() <= f32::EPSILON {
            continue;
        }
        let r = 1.0 / denom;
        let tangent = [
            (edge1[0] * delta_uv2[1] - edge2[0] * delta_uv1[1]) * r,
            (edge1[1] * delta_uv2[1] - edge2[1] * delta_uv1[1]) * r,
            (edge1[2] * delta_uv2[1] - edge2[2] * delta_uv1[1]) * r,
        ];
        let bitangent = [
            (edge2[0] * delta_uv1[0] - edge1[0] * delta_uv2[0]) * r,
            (edge2[1] * delta_uv1[0] - edge1[1] * delta_uv2[0]) * r,
            (edge2[2] * delta_uv1[0] - edge1[2] * delta_uv2[0]) * r,
        ];
        for index in [i0, i1, i2] {
            tan1[index] = add(tan1[index], tangent);
            tan2[index] = add(tan2[index], bitangent);
        }
    }

    let mut tangents = Vec::with_capacity(mesh.positions.len());
    for index in 0..mesh.positions.len() {
        let normal = mesh.normals[index];
        let tangent = tan1[index];
        let tangent = orthogonalize(tangent, normal);
        let handedness = if dot(cross(normal, tangent), tan2[index]) < 0.0 {
            -1.0
        } else {
            1.0
        };
        tangents.push([tangent[0], tangent[1], tangent[2], handedness]);
    }
    mesh.tangents = tangents;
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length_squared(v: [f32; 3]) -> f32 {
    dot(v, v)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = length_squared(v).sqrt();
    if len <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn orthogonalize(v: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
    let n = normalize(normal);
    let projection = dot(v, n);
    normalize(sub(
        v,
        [n[0] * projection, n[1] * projection, n[2] * projection],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::unit_cube;

    #[test]
    fn normals_match_vertex_count() {
        let mut mesh = unit_cube();
        compute_normals(&mut mesh);
        assert_eq!(mesh.normals.len(), mesh.positions.len());
    }
}
