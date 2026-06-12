use std::fs;
use std::path::Path;

use c3d_asset_pointcloud::{PointCloudAssetData, PointCloudChunkPayload, PointCloudChunkRecord};
use c3d_core::AssetId;

use crate::chunking::{chunk_point_cloud, ParsedPointCloud};
use crate::error::{ImportError, ImportResult};

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
    let parsed = parse_ascii_ply(bytes)?;
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

fn parse_ascii_ply(bytes: &[u8]) -> ImportResult<ParsedPointCloud> {
    let text = std::str::from_utf8(bytes).map_err(|err| ImportError::Invalid(err.to_string()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| ImportError::Invalid("empty ply".into()))?;
    if header.trim() != "ply" {
        return Err(ImportError::Invalid("missing ply header".into()));
    }

    let mut vertex_count = 0usize;
    let mut properties: Vec<String> = Vec::new();
    loop {
        let line = lines
            .next()
            .ok_or_else(|| ImportError::Invalid("unexpected eof in header".into()))?;
        if line.starts_with("element vertex ") {
            vertex_count = line
                .split_whitespace()
                .nth(2)
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| ImportError::Invalid("invalid vertex count".into()))?;
        } else if line.starts_with("property ") {
            if let Some(name) = line.split_whitespace().last() {
                properties.push(name.to_string());
            }
        } else if line.starts_with("element face ") || line.trim() == "end_header" {
            break;
        }
    }

    if vertex_count == 0 {
        return Err(ImportError::Invalid("ply has zero vertices".into()));
    }

    let index = |name: &str| properties.iter().position(|prop| prop == name);
    let x = index("x").ok_or_else(|| ImportError::Invalid("missing x property".into()))?;
    let y = index("y").ok_or_else(|| ImportError::Invalid("missing y property".into()))?;
    let z = index("z").ok_or_else(|| ImportError::Invalid("missing z property".into()))?;
    let red = index("red");
    let green = index("green");
    let blue = index("blue");
    let intensity = index("intensity");
    let classification = index("classification").or_else(|| index("class"));

    let mut parsed = ParsedPointCloud::default();
    for _ in 0..vertex_count {
        let line = lines
            .next()
            .ok_or_else(|| ImportError::Invalid("unexpected eof in vertex data".into()))?;
        let values: Vec<f32> = line
            .split_whitespace()
            .map(|value| value.parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|_| ImportError::Invalid("invalid vertex numeric data".into()))?;
        parsed.positions.push([values[x], values[y], values[z]]);
        if let (Some(r), Some(g), Some(b)) = (red, green, blue) {
            parsed.colors.push([
                values[r].clamp(0.0, 255.0) as u8,
                values[g].clamp(0.0, 255.0) as u8,
                values[b].clamp(0.0, 255.0) as u8,
            ]);
        }
        if let Some(index) = intensity {
            parsed.intensity.push(values[index]);
        }
        if let Some(index) = classification {
            parsed.classification.push(values[index] as u8);
        }
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
}
