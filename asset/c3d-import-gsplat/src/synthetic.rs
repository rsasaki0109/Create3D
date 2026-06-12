use crate::chunking::{chunk_gaussian_splats, ParsedGaussianSplatCloud};
use crate::error::{ImportError, ImportResult};
use crate::import::GsplatImportResult;

/// Generate a synthetic Gaussian splat cloud for performance and residency tests.
pub fn generate_synthetic_gaussian_splats(
    splat_count: usize,
    chunk_target: usize,
) -> ImportResult<GsplatImportResult> {
    if splat_count == 0 {
        return Err(ImportError::Invalid("splat count must be non-zero".into()));
    }
    let parsed = ParsedGaussianSplatCloud {
        positions: (0..splat_count)
            .map(|index| {
                let t = index as f32 * 0.01;
                [t.sin(), t.cos(), (index % 100) as f32 * 0.05]
            })
            .collect(),
        rotations: vec![[0.0, 0.0, 0.0, 1.0]; splat_count],
        scales: vec![[0.05, 0.05, 0.05]; splat_count],
        opacities: (0..splat_count)
            .map(|index| 0.4 + (index % 10) as f32 * 0.05)
            .collect(),
        colors: (0..splat_count)
            .map(|index| {
                [
                    (index % 255) as f32 / 255.0,
                    ((index * 3) % 255) as f32 / 255.0,
                    ((index * 7) % 255) as f32 / 255.0,
                ]
            })
            .collect(),
    };
    let chunks = chunk_gaussian_splats(parsed.clone(), chunk_target)?;
    let metadata = crate::import::build_metadata(&parsed, &chunks);
    Ok(GsplatImportResult {
        metadata,
        chunks,
        name: "synthetic-gaussian-splat".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_cloud_is_chunked() {
        let imported = generate_synthetic_gaussian_splats(10_000, 2_048).expect("synthetic");
        assert!(imported.chunks.len() > 1);
        assert_eq!(imported.metadata.splat_count, 10_000);
    }
}
