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

/// Generate a dense indoor site scan for editor marketing previews.
pub fn generate_preview_site_scan() -> ImportResult<PlyImportResult> {
    let parsed = build_site_scan_point_cloud();
    let chunks = chunk_point_cloud(parsed.clone(), 8_192)?;
    let metadata = crate::import::build_metadata(&parsed, &chunks);
    Ok(PlyImportResult {
        metadata,
        chunks,
        name: "Site Scan".into(),
    })
}

fn build_site_scan_point_cloud() -> ParsedPointCloud {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut intensity = Vec::new();
    let mut classification = Vec::new();

    let mut push = |position: [f32; 3], color: [u8; 3], class: u8| {
        let height = position[1].clamp(0.0, 2.8);
        intensity.push(height / 2.8);
        positions.push(position);
        colors.push(color);
        classification.push(class);
    };

    // Floor slab with subtle variation.
    for ix in 0..90 {
        for iz in 0..90 {
            let x = ix as f32 * 0.045 - 2.0;
            let z = iz as f32 * 0.045 - 2.0;
            let noise = ((ix * 17 + iz * 31) % 13) as f32 * 0.002;
            push(
                [x, noise, z],
                [118 + (ix % 5) as u8, 122 + (iz % 4) as u8, 128],
                2,
            );
        }
    }

    // Back wall (z = -2.0) and side wall (x = -2.0).
    for ix in 0..90 {
        for iy in 0..56 {
            let x = ix as f32 * 0.045 - 2.0;
            let y = iy as f32 * 0.05;
            push([x, y, -2.0], [96, 98, 104], 3);
            push([-2.0, y, x], [92, 94, 100], 3);
        }
    }

    // Ceiling strip for enclosure cues.
    for ix in 0..90 {
        for iz in 0..90 {
            let x = ix as f32 * 0.045 - 2.0;
            let z = iz as f32 * 0.045 - 2.0;
            if x < -1.85 || z < -1.85 {
                push([x, 2.75, z], [110, 112, 118], 3);
            }
        }
    }

    // Central scanned object: fibonacci sphere with warm scan tones.
    let center = [0.35, 0.95, 0.15];
    let radius = 0.72;
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let samples = 18_000usize;
    for index in 0..samples {
        let t = index as f32 / samples as f32;
        let inclination = (1.0 - 2.0 * t).acos();
        let azimuth = golden * index as f32;
        let nx = inclination.sin() * azimuth.cos();
        let ny = inclination.cos();
        let nz = inclination.sin() * azimuth.sin();
        let jitter = ((index * 19) % 97) as f32 * 0.00035;
        let position = [
            center[0] + nx * (radius + jitter),
            center[1] + ny * (radius + jitter),
            center[2] + nz * (radius + jitter),
        ];
        let albedo = [
            (180.0 + nx * 40.0) as u8,
            (150.0 + ny * 35.0) as u8,
            (120.0 + nz * 30.0) as u8,
        ];
        push(position, albedo, 6);
    }

    // Low table under the object.
    for ix in 0..36 {
        for iz in 0..24 {
            let x = ix as f32 * 0.03 - 0.35;
            let z = iz as f32 * 0.03 - 0.15;
            push([x, 0.42, z], [132, 96, 68], 5);
            push([x, 0.45, z], [138, 102, 74], 5);
        }
    }

    ParsedPointCloud {
        positions,
        colors,
        intensity,
        classification,
    }
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
