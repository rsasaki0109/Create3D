use c3d_asset_mesh::MeshAssetData;

/// Built-in primitive mesh kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    /// Axis-aligned unit cube centered at the origin.
    UnitCube,
    /// Horizontal plane on the XZ plane.
    Plane,
}

/// Build a unit cube render mesh.
pub fn unit_cube() -> MeshAssetData {
    let half = 0.5;
    let positions = vec![
        [-half, -half, -half],
        [half, -half, -half],
        [half, half, -half],
        [-half, half, -half],
        [-half, -half, half],
        [half, -half, half],
        [half, half, half],
        [-half, half, half],
    ];
    let indices = vec![
        0, 1, 2, 0, 2, 3, // back
        4, 6, 5, 4, 7, 6, // front
        0, 4, 5, 0, 5, 1, // bottom
        2, 6, 7, 2, 7, 3, // top
        0, 3, 7, 0, 7, 4, // left
        1, 5, 6, 1, 6, 2, // right
    ];
    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    MeshAssetData {
        version: 1,
        positions,
        normals: Vec::new(),
        uvs,
        indices,
        tangents: Vec::new(),
    }
}

/// Build a subdivided plane on the XZ plane centered at the origin.
pub fn plane(width: f32, depth: f32, subdivisions: u32) -> MeshAssetData {
    let segments = subdivisions.max(1);
    let half_w = width * 0.5;
    let half_d = depth * 0.5;
    let verts_x = segments + 1;
    let verts_z = segments + 1;
    let mut positions = Vec::with_capacity((verts_x * verts_z) as usize);
    let mut uvs = Vec::with_capacity((verts_x * verts_z) as usize);

    for z in 0..verts_z {
        for x in 0..verts_x {
            let u = x as f32 / segments as f32;
            let v = z as f32 / segments as f32;
            positions.push([-half_w + width * u, 0.0, -half_d + depth * v]);
            uvs.push([u, v]);
        }
    }

    let mut indices = Vec::new();
    for z in 0..segments {
        for x in 0..segments {
            let i0 = z * verts_x + x;
            let i1 = i0 + 1;
            let i2 = i0 + verts_x;
            let i3 = i2 + 1;
            indices.extend([i0, i2, i1, i1, i2, i3]);
        }
    }

    MeshAssetData {
        version: 1,
        positions,
        normals: Vec::new(),
        uvs,
        indices,
        tangents: Vec::new(),
    }
}

/// Build a primitive mesh by kind.
pub fn primitive(kind: PrimitiveKind) -> MeshAssetData {
    match kind {
        PrimitiveKind::UnitCube => unit_cube(),
        PrimitiveKind::Plane => plane(2.0, 2.0, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::compute_normals;

    #[test]
    fn primitives_validate() {
        for mesh in [unit_cube(), plane(1.0, 1.0, 2)] {
            let mut mesh = mesh;
            compute_normals(&mut mesh);
            mesh.validate().expect("primitive mesh valid");
        }
    }
}
