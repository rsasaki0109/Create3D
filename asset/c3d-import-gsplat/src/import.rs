use std::fs;
use std::path::Path;

use c3d_asset_gsplat::{
    GaussianSplatAssetData, GaussianSplatChunkPayload, GaussianSplatChunkRecord,
};
use c3d_core::AssetId;

use crate::chunking::{chunk_gaussian_splats, ParsedGaussianSplatCloud};
use crate::error::{ImportError, ImportResult};

/// Result of importing a 3DGS PLY file before AssetDB persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct GsplatImportResult {
    /// Gaussian splat metadata blob payload.
    pub metadata: GaussianSplatAssetData,
    /// Chunk payloads keyed by stable chunk id order.
    pub chunks: Vec<GaussianSplatChunkPayload>,
    /// Suggested entity name.
    pub name: String,
}

/// Returns true when a PLY header looks like a 3D Gaussian splat export.
pub fn looks_like_gsplat_ply(bytes: &[u8]) -> bool {
    let header = std::str::from_utf8(bytes).unwrap_or("");
    let end = header
        .find("end_header")
        .map(|index| index + "end_header".len())
        .unwrap_or(header.len().min(4096));
    let header = &header[..end];
    header.contains("f_dc_0")
        && header.contains("opacity")
        && header.contains("scale_0")
        && header.contains("rot_0")
}

/// Import a 3D Gaussian splat PLY file from disk.
pub fn import_gsplat_ply_path(path: &Path) -> ImportResult<GsplatImportResult> {
    let bytes = fs::read(path).map_err(|err| ImportError::Io(err.to_string()))?;
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("gaussian-splat")
        .to_string();
    import_gsplat_ply_bytes(&bytes, &name)
}

/// Import a 3D Gaussian splat PLY file from memory.
pub fn import_gsplat_ply_bytes(bytes: &[u8], name: &str) -> ImportResult<GsplatImportResult> {
    let parsed = parse_gsplat_ply(bytes)?;
    let chunks = chunk_gaussian_splats(parsed.clone(), 4_096)?;
    let metadata = build_metadata(&parsed, &chunks);
    Ok(GsplatImportResult {
        metadata,
        chunks,
        name: name.to_string(),
    })
}

pub(crate) fn build_metadata(
    parsed: &ParsedGaussianSplatCloud,
    chunks: &[GaussianSplatChunkPayload],
) -> GaussianSplatAssetData {
    let (bounds_min, bounds_max) = chunk_bounds(chunks);
    GaussianSplatAssetData {
        version: 1,
        splat_count: parsed.positions.len() as u64,
        bounds_min,
        bounds_max,
        sh_degree: 0,
        chunks: chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let (min, max) = payload_bounds(chunk);
                GaussianSplatChunkRecord {
                    chunk_id: index as u32,
                    bounds_min: min,
                    bounds_max: max,
                    splat_count: chunk.splat_count() as u32,
                    blob_asset_id: AssetId::new(),
                    lod_stride: if chunk.splat_count() > 8_192 { 4 } else { 1 },
                }
            })
            .collect(),
    }
}

fn chunk_bounds(chunks: &[GaussianSplatChunkPayload]) -> ([f32; 3], [f32; 3]) {
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

fn payload_bounds(chunk: &GaussianSplatChunkPayload) -> ([f32; 3], [f32; 3]) {
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

fn parse_gsplat_ply(bytes: &[u8]) -> ImportResult<ParsedGaussianSplatCloud> {
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
    let f_dc_0 = index("f_dc_0")
        .ok_or_else(|| ImportError::Invalid("missing f_dc_0 property (not a 3DGS ply)".into()))?;
    let f_dc_1 = index("f_dc_1").ok_or_else(|| ImportError::Invalid("missing f_dc_1".into()))?;
    let f_dc_2 = index("f_dc_2").ok_or_else(|| ImportError::Invalid("missing f_dc_2".into()))?;
    let opacity = index("opacity").ok_or_else(|| ImportError::Invalid("missing opacity".into()))?;
    let scale_0 = index("scale_0").ok_or_else(|| ImportError::Invalid("missing scale_0".into()))?;
    let scale_1 = index("scale_1").ok_or_else(|| ImportError::Invalid("missing scale_1".into()))?;
    let scale_2 = index("scale_2").ok_or_else(|| ImportError::Invalid("missing scale_2".into()))?;
    let rot_0 = index("rot_0").ok_or_else(|| ImportError::Invalid("missing rot_0".into()))?;
    let rot_1 = index("rot_1").ok_or_else(|| ImportError::Invalid("missing rot_1".into()))?;
    let rot_2 = index("rot_2").ok_or_else(|| ImportError::Invalid("missing rot_2".into()))?;
    let rot_3 = index("rot_3").ok_or_else(|| ImportError::Invalid("missing rot_3".into()))?;

    let mut parsed = ParsedGaussianSplatCloud::default();
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
        parsed.rotations.push(normalize_quaternion([
            values[rot_0],
            values[rot_1],
            values[rot_2],
            values[rot_3],
        ]));
        parsed.scales.push([
            values[scale_0].exp(),
            values[scale_1].exp(),
            values[scale_2].exp(),
        ]);
        parsed.opacities.push(sigmoid(values[opacity]));
        parsed.colors.push(sh_dc_to_rgb([
            values[f_dc_0],
            values[f_dc_1],
            values[f_dc_2],
        ]));
    }

    Ok(parsed)
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

const SH_C0: f32 = 0.282_095;

fn sh_dc_to_rgb(sh: [f32; 3]) -> [f32; 3] {
    [
        (SH_C0 * sh[0] + 0.5).clamp(0.0, 1.0),
        (SH_C0 * sh[1] + 0.5).clamp(0.0, 1.0),
        (SH_C0 * sh[2] + 0.5).clamp(0.0, 1.0),
    ]
}

fn normalize_quaternion(value: [f32; 4]) -> [f32; 4] {
    let length =
        (value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3])
            .sqrt()
            .max(f32::EPSILON);
    [
        value[0] / length,
        value[1] / length,
        value[2] / length,
        value[3] / length,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_minimal_gsplat_ply() {
        let ply = b"ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nproperty float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\nproperty float opacity\nproperty float scale_0\nproperty float scale_1\nproperty float scale_2\nproperty float rot_0\nproperty float rot_1\nproperty float rot_2\nproperty float rot_3\nend_header\n0 0 0 0 0 0 0 0 0 0 0 0 0 1\n";
        let imported = import_gsplat_ply_bytes(ply, "single").expect("import");
        assert_eq!(imported.metadata.splat_count, 1);
    }

    #[test]
    fn detects_gsplat_ply_header() {
        let ply = b"ply\nformat ascii 1.0\nproperty float f_dc_0\nproperty float opacity\nproperty float scale_0\nproperty float rot_0\nend_header\n";
        assert!(looks_like_gsplat_ply(ply));
        assert!(!looks_like_gsplat_ply(
            b"ply\nproperty float x\nend_header\n"
        ));
    }
}
