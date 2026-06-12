use crate::chunking::{chunk_point_cloud, ParsedPointCloud};
use crate::error::{ImportError, ImportResult};
use crate::import::PlyImportResult;

/// Generate a synthetic point cloud for performance and residency tests.
pub fn generate_synthetic_point_cloud(
    point_count: usize,
    chunk_target: usize,
) -> ImportResult<PlyImportResult> {
    if point_count == 0 {
        return Err(ImportError::Invalid("point count must be non-zero".into()));
    }
    let parsed = ParsedPointCloud {
        positions: (0..point_count)
            .map(|index| {
                let t = index as f32 * 0.01;
                [t.sin(), t.cos(), (index % 100) as f32 * 0.05]
            })
            .collect(),
        colors: (0..point_count)
            .map(|index| {
                [
                    (index % 255) as u8,
                    ((index * 3) % 255) as u8,
                    ((index * 7) % 255) as u8,
                ]
            })
            .collect(),
        intensity: (0..point_count)
            .map(|index| (index % 1024) as f32 / 1024.0)
            .collect(),
        classification: (0..point_count).map(|index| (index % 8) as u8).collect(),
    };
    let chunks = chunk_point_cloud(parsed.clone(), chunk_target)?;
    let metadata = crate::import::build_metadata(&parsed, &chunks);
    Ok(PlyImportResult {
        metadata,
        chunks,
        name: "synthetic-point-cloud".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_cloud_is_chunked() {
        let imported = generate_synthetic_point_cloud(20_000, 4_000).expect("synthetic");
        assert!(imported.chunks.len() > 1);
        assert_eq!(imported.metadata.point_count, 20_000);
    }
}
