use std::path::Path;

use c3d_asset_mesh::MeshAssetData;

use crate::{ImportError, ImportResult};

/// Import an STL mesh file from disk.
pub fn import_stl_path(path: impl AsRef<Path>) -> ImportResult<MeshAssetData> {
    let bytes = std::fs::read(path)?;
    import_stl_bytes(&bytes)
}

/// Import an STL mesh from memory.
pub fn import_stl_bytes(bytes: &[u8]) -> ImportResult<MeshAssetData> {
    if is_ascii_stl(bytes) {
        import_ascii_stl(bytes)
    } else {
        import_binary_stl(bytes)
    }
}

fn is_ascii_stl(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    text.starts_with("solid") && text.contains("facet") && text.contains("vertex")
}

fn import_binary_stl(bytes: &[u8]) -> ImportResult<MeshAssetData> {
    if bytes.len() < 84 {
        return Err(ImportError::Invalid("binary STL too short".into()));
    }

    let triangle_count = u32::from_le_bytes(bytes[80..84].try_into().expect("triangle count"));
    let expected_len = 84 + triangle_count as usize * 50;
    if bytes.len() < expected_len {
        return Err(ImportError::Invalid(format!(
            "binary STL truncated: expected at least {expected_len} bytes, got {}",
            bytes.len()
        )));
    }

    let mut positions = Vec::with_capacity(triangle_count as usize * 3);
    let mut normals = Vec::with_capacity(triangle_count as usize * 3);
    let mut indices = Vec::with_capacity(triangle_count as usize * 3);
    let mut offset = 0_u32;

    for triangle_index in 0..triangle_count {
        let start = 84 + triangle_index as usize * 50;
        let chunk = &bytes[start..start + 50];
        let normal = read_f32_triplet(&chunk[0..12]);
        normals.extend([normal, normal, normal]);

        for vertex_index in 0..3 {
            let vertex_start = 12 + vertex_index * 12;
            positions.push(read_f32_triplet(&chunk[vertex_start..vertex_start + 12]));
            indices.push(offset);
            offset += 1;
        }
    }

    Ok(MeshAssetData {
        version: 1,
        positions,
        normals,
        uvs: Vec::new(),
        tangents: Vec::new(),
        indices,
    })
}

fn import_ascii_stl(bytes: &[u8]) -> ImportResult<MeshAssetData> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| ImportError::Invalid(format!("ascii STL is not valid UTF-8: {err}")))?;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut current_normal = [0.0_f32; 3];
    let mut vertex_in_facet = 0_u32;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("facet") => {
                if parts.next() != Some("normal") {
                    return Err(ImportError::Invalid(format!("invalid facet line `{line}`")));
                }
                current_normal = [
                    parse_f32(parts.next(), "normal x")?,
                    parse_f32(parts.next(), "normal y")?,
                    parse_f32(parts.next(), "normal z")?,
                ];
                vertex_in_facet = 0;
            }
            Some("vertex") => {
                let vertex = [
                    parse_f32(parts.next(), "vertex x")?,
                    parse_f32(parts.next(), "vertex y")?,
                    parse_f32(parts.next(), "vertex z")?,
                ];
                positions.push(vertex);
                normals.push(current_normal);
                indices.push(positions.len() as u32 - 1);
                vertex_in_facet += 1;
                if vertex_in_facet > 3 {
                    return Err(ImportError::Invalid(format!(
                        "facet has more than three vertices near `{line}`"
                    )));
                }
            }
            Some("endsolid") | Some("solid") | Some("outer") | Some("loop") | Some("endloop")
            | Some("endfacet") => {}
            Some(other) => {
                return Err(ImportError::Invalid(format!(
                    "unsupported ascii STL token `{other}`"
                )));
            }
            None => {}
        }
    }

    if positions.is_empty() {
        return Err(ImportError::Invalid(
            "ascii STL contains no vertices".into(),
        ));
    }

    Ok(MeshAssetData {
        version: 1,
        positions,
        normals,
        uvs: Vec::new(),
        tangents: Vec::new(),
        indices,
    })
}

fn read_f32_triplet(bytes: &[u8]) -> [f32; 3] {
    [
        f32::from_le_bytes(bytes[0..4].try_into().expect("x")),
        f32::from_le_bytes(bytes[4..8].try_into().expect("y")),
        f32::from_le_bytes(bytes[8..12].try_into().expect("z")),
    ]
}

fn parse_f32(value: Option<&str>, label: &str) -> ImportResult<f32> {
    value
        .ok_or_else(|| ImportError::Invalid(format!("missing {label}")))?
        .parse()
        .map_err(|err| ImportError::Invalid(format!("invalid {label}: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_binary_stl_single_triangle() {
        let mut bytes = vec![0_u8; 84];
        bytes[80..84].copy_from_slice(&1_u32.to_le_bytes());
        let mut triangle = Vec::new();
        for value in [
            0.0_f32, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ] {
            triangle.extend_from_slice(&value.to_le_bytes());
        }
        triangle.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend(triangle);

        let mesh = import_stl_bytes(&bytes).expect("import binary stl");
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
    }

    #[test]
    fn imports_ascii_stl_single_triangle() {
        let stl = r#"solid test
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 0 1 0
    endloop
  endfacet
endsolid test
"#;
        let mesh = import_stl_bytes(stl.as_bytes()).expect("import ascii stl");
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }
}
