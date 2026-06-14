use std::fs;
use std::path::Path;

use c3d_asset_pointcloud::{PointCloudAssetData, PointCloudChunkPayload, PointCloudChunkRecord};
use c3d_core::AssetId;

use crate::chunking::{chunk_point_cloud, ParsedPointCloud};
use crate::error::{ImportError, ImportResult};
use crate::ply_header::{parse_ply_header, PlyFormat, PlyHeader, PlyPropertyType};

/// Result of importing a PLY point cloud before AssetDB persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct PlyImportResult {
    /// Point cloud metadata blob payload.
    pub metadata: PointCloudAssetData,
    /// Chunk payloads keyed by stable chunk id order.
    pub chunks: Vec<PointCloudChunkPayload>,
    /// Suggested entity name.
    pub name: String,
}

/// Import a PLY file from disk.
pub fn import_ply_path(path: &Path) -> ImportResult<PlyImportResult> {
    let bytes = fs::read(path).map_err(|err| ImportError::Io(err.to_string()))?;
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("point-cloud")
        .to_string();
    import_ply_bytes(&bytes, &name)
}

/// Import a PLY file from memory.
pub fn import_ply_bytes(bytes: &[u8], name: &str) -> ImportResult<PlyImportResult> {
    let parsed = parse_ply(bytes)?;
    let chunks = chunk_point_cloud(parsed.clone(), 8_192)?;
    let metadata = build_metadata(&parsed, &chunks);
    Ok(PlyImportResult {
        metadata,
        chunks,
        name: name.to_string(),
    })
}

pub(crate) fn build_metadata(
    parsed: &ParsedPointCloud,
    chunks: &[PointCloudChunkPayload],
) -> PointCloudAssetData {
    let (bounds_min, bounds_max) = chunk_bounds(chunks);
    PointCloudAssetData {
        version: 1,
        point_count: parsed.positions.len() as u64,
        bounds_min,
        bounds_max,
        has_rgb: !parsed.colors.is_empty(),
        has_intensity: !parsed.intensity.is_empty(),
        has_classification: !parsed.classification.is_empty(),
        chunks: chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let (min, max) = payload_bounds(chunk);
                PointCloudChunkRecord {
                    chunk_id: index as u32,
                    bounds_min: min,
                    bounds_max: max,
                    point_count: chunk.point_count() as u32,
                    blob_asset_id: AssetId::new(),
                    lod_stride: if chunk.point_count() > 16_384 { 4 } else { 1 },
                }
            })
            .collect(),
    }
}

fn chunk_bounds(chunks: &[PointCloudChunkPayload]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for chunk in chunks {
        let (chunk_min, chunk_max) = payload_bounds(chunk);
        for axis in 0..3 {
            min[axis] = min[axis].min(chunk_min[axis]);
            max[axis] = max[axis].max(chunk_max[axis]);
        }
    }
    (min, max)
}

fn payload_bounds(chunk: &PointCloudChunkPayload) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &chunk.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (min, max)
}

fn parse_ply(bytes: &[u8]) -> ImportResult<ParsedPointCloud> {
    let header = parse_ply_header(bytes)?;
    match header.format {
        PlyFormat::Ascii => parse_ascii_vertices(bytes, &header),
        PlyFormat::BinaryLittleEndian => parse_binary_vertices(bytes, &header),
    }
}

struct PropertyIndices {
    x: usize,
    y: usize,
    z: usize,
    red: Option<usize>,
    green: Option<usize>,
    blue: Option<usize>,
    intensity: Option<usize>,
    classification: Option<usize>,
}

fn property_indices(header: &PlyHeader) -> ImportResult<PropertyIndices> {
    let index = |name: &str| header.properties.iter().position(|prop| prop.name == name);
    Ok(PropertyIndices {
        x: index("x").ok_or_else(|| ImportError::Invalid("missing x property".into()))?,
        y: index("y").ok_or_else(|| ImportError::Invalid("missing y property".into()))?,
        z: index("z").ok_or_else(|| ImportError::Invalid("missing z property".into()))?,
        red: index("red"),
        green: index("green"),
        blue: index("blue"),
        intensity: index("intensity"),
        classification: index("classification").or_else(|| index("class")),
    })
}

fn property_offset(header: &PlyHeader, property_index: usize) -> usize {
    header.properties[..property_index]
        .iter()
        .map(|property| property.byte_size())
        .sum()
}

fn read_property_f32(record: &[u8], header: &PlyHeader, property_index: usize) -> f32 {
    let offset = property_offset(header, property_index);
    match header.properties[property_index].property_type {
        PlyPropertyType::Float => {
            f32::from_le_bytes(record[offset..offset + 4].try_into().expect("float"))
        }
        PlyPropertyType::UChar => record[offset] as f32,
    }
}

fn append_vertex(parsed: &mut ParsedPointCloud, indices: &PropertyIndices, values: &[f32]) {
    parsed
        .positions
        .push([values[indices.x], values[indices.y], values[indices.z]]);
    if let (Some(r), Some(g), Some(b)) = (indices.red, indices.green, indices.blue) {
        parsed.colors.push([
            values[r].clamp(0.0, 255.0) as u8,
            values[g].clamp(0.0, 255.0) as u8,
            values[b].clamp(0.0, 255.0) as u8,
        ]);
    }
    if let Some(index) = indices.intensity {
        parsed.intensity.push(values[index]);
    }
    if let Some(index) = indices.classification {
        parsed.classification.push(values[index] as u8);
    }
}

fn parse_ascii_vertices(bytes: &[u8], header: &PlyHeader) -> ImportResult<ParsedPointCloud> {
    let text = std::str::from_utf8(&bytes[header.data_offset..])
        .map_err(|err| ImportError::Invalid(err.to_string()))?;
    let indices = property_indices(header)?;
    let mut parsed = ParsedPointCloud::default();

    for line in text.lines().take(header.vertex_count) {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<f32> = line
            .split_whitespace()
            .map(|value| value.parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|_| ImportError::Invalid("invalid vertex numeric data".into()))?;
        if values.len() < header.properties.len() {
            return Err(ImportError::Invalid(
                "ascii vertex has fewer fields than properties".into(),
            ));
        }
        append_vertex(&mut parsed, &indices, &values);
    }

    if parsed.positions.len() != header.vertex_count {
        return Err(ImportError::Invalid(format!(
            "expected {} ascii vertices, got {}",
            header.vertex_count,
            parsed.positions.len()
        )));
    }

    Ok(parsed)
}

fn parse_binary_vertices(bytes: &[u8], header: &PlyHeader) -> ImportResult<ParsedPointCloud> {
    let stride = header.vertex_stride();
    let expected_len = header.data_offset + header.vertex_count * stride;
    if bytes.len() < expected_len {
        return Err(ImportError::Invalid(format!(
            "binary ply truncated: expected at least {expected_len} bytes, got {}",
            bytes.len()
        )));
    }

    let indices = property_indices(header)?;
    let mut parsed = ParsedPointCloud::default();
    for vertex_index in 0..header.vertex_count {
        let start = header.data_offset + vertex_index * stride;
        let record = &bytes[start..start + stride];
        let values: Vec<f32> = (0..header.properties.len())
            .map(|property_index| read_property_f32(record, header, property_index))
            .collect();
        append_vertex(&mut parsed, &indices, &values);
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_ascii_ply() {
        let ply = b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n0 0 0 255 0 0\n1 0 0 0 255 0\n0 1 0 0 0 255\n";
        let imported = import_ply_bytes(ply, "triangle").expect("import ply");
        assert_eq!(imported.metadata.point_count, 3);
        assert!(imported.metadata.has_rgb);
    }

    #[test]
    fn imports_binary_ply() {
        let mut ply = b"ply\nformat binary_little_endian 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n".to_vec();
        for (x, y, z, rgb) in [
            (0.0f32, 0.0f32, 0.0f32, [255u8, 0, 0]),
            (1.0f32, 0.0f32, 0.0f32, [0, 255, 0]),
        ] {
            ply.extend_from_slice(&x.to_le_bytes());
            ply.extend_from_slice(&y.to_le_bytes());
            ply.extend_from_slice(&z.to_le_bytes());
            ply.extend_from_slice(&rgb);
        }
        let imported = import_ply_bytes(&ply, "binary").expect("import binary ply");
        assert_eq!(imported.metadata.point_count, 2);
        assert!(imported.metadata.has_rgb);
    }
}
