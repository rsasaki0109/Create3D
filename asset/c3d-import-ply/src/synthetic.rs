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
    // Use a large chunk target so the whole room stays GPU-resident under the
    // default residency cap (the preview is a fixed marketing shot, not streamed).
    let chunks = chunk_point_cloud(parsed.clone(), 30_000)?;
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

    // Floor slab: dense enough to read as a solid ground plane (points are 1px),
    // with a cool elevation gradient and a subtle scan-line shimmer. A radial
    // falloff darkens the room away from the focal sphere so the eye stays on it.
    for ix in 0..210 {
        for iz in 0..210 {
            let x = ix as f32 * 0.019 - 2.0;
            let z = iz as f32 * 0.019 - 2.0;
            let noise = ((ix * 17 + iz * 31) % 13) as f32 * 0.002;
            let shimmer = ((ix / 3 + iz / 3) % 6) as f32 * 0.012;
            let color = dim(elevation_color(noise, shimmer), room_falloff(x, z));
            push([x, noise, z], color, 2);
        }
    }

    // Back wall (z = -2.0) and side wall (x = -2.0): cool gradient rising with height.
    for ix in 0..210 {
        for iy in 0..150 {
            let x = ix as f32 * 0.019 - 2.0;
            let y = iy as f32 * 0.019;
            let shimmer = (iy / 3 % 5) as f32 * 0.01;
            let back = dim(elevation_color(y, shimmer), room_falloff(x, -2.0));
            let side = dim(elevation_color(y, shimmer + 0.015), room_falloff(-2.0, x));
            push([x, y, -2.0], back, 3);
            push([-2.0, y, x], side, 3);
        }
    }

    // Ceiling strip for enclosure cues.
    for ix in 0..210 {
        for iz in 0..210 {
            let x = ix as f32 * 0.019 - 2.0;
            let z = iz as f32 * 0.019 - 2.0;
            if x < -1.78 || z < -1.78 {
                let color = dim(elevation_color(2.75, 0.0), room_falloff(x, z));
                push([x, 2.75, z], color, 3);
            }
        }
    }

    // Central scanned object: fibonacci sphere with a vivid spectral sweep so it
    // reads as the focal point against the dark viewport.
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
        // Hue follows the azimuth for a rainbow sweep; brightness follows the
        // surface normal so the top catches the light.
        let hue = (azimuth.rem_euclid(std::f32::consts::TAU)) / std::f32::consts::TAU;
        let value = 0.78 + 0.22 * ny.clamp(-1.0, 1.0);
        push(position, hsv_to_rgb(hue, 0.85, value), 6);
    }

    // Faint outer shell: a soft rainbow halo just outside the sphere so the focal
    // point reads as glowing rather than a hard ball.
    let shell_samples = 5_000usize;
    for index in 0..shell_samples {
        let t = index as f32 / shell_samples as f32;
        let inclination = (1.0 - 2.0 * t).acos();
        let azimuth = golden * index as f32;
        let nx = inclination.sin() * azimuth.cos();
        let ny = inclination.cos();
        let nz = inclination.sin() * azimuth.sin();
        let shell_radius = radius + 0.13;
        let position = [
            center[0] + nx * shell_radius,
            center[1] + ny * shell_radius,
            center[2] + nz * shell_radius,
        ];
        let hue = (azimuth.rem_euclid(std::f32::consts::TAU)) / std::f32::consts::TAU;
        let value = (0.78 + 0.22 * ny.clamp(-1.0, 1.0)) * 0.55;
        push(position, hsv_to_rgb(hue, 0.7, value), 6);
    }

    // Low table under the object: warm wood tones to contrast the cool room.
    for ix in 0..36 {
        for iz in 0..24 {
            let x = ix as f32 * 0.03 - 0.35;
            let z = iz as f32 * 0.03 - 0.15;
            push([x, 0.42, z], [176, 122, 78], 5);
            push([x, 0.45, z], [192, 136, 88], 5);
        }
    }

    ParsedPointCloud {
        positions,
        colors,
        intensity,
        classification,
    }
}

/// Radial brightness multiplier centered on the focal sphere's footprint, so the
/// room dims toward the corners and the eye is drawn to the center.
fn room_falloff(x: f32, z: f32) -> f32 {
    let dx = x - 0.35;
    let dz = z - 0.15;
    let dist = (dx * dx + dz * dz).sqrt();
    (1.0 - 0.42 * (dist / 3.2)).clamp(0.58, 1.0)
}

/// Scale an RGB triple by a brightness factor.
fn dim(color: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (color[0] as f32 * factor) as u8,
        (color[1] as f32 * factor) as u8,
        (color[2] as f32 * factor) as u8,
    ]
}

/// Cool elevation ramp (deep indigo floor -> teal -> airy cyan) used for the room
/// shell, with a small additive shimmer term for scan-line texture.
fn elevation_color(height: f32, shimmer: f32) -> [u8; 3] {
    let t = (height / 2.8).clamp(0.0, 1.0);
    // Three-stop gradient through indigo, teal, and pale cyan.
    let stops = [
        (0.0_f32, [58.0_f32, 84.0, 152.0]),
        (0.5, [44.0, 156.0, 178.0]),
        (1.0, [156.0, 220.0, 226.0]),
    ];
    let (lo, hi) = if t < 0.5 {
        (stops[0], stops[1])
    } else {
        (stops[1], stops[2])
    };
    let span = (hi.0 - lo.0).max(1e-3);
    let local = ((t - lo.0) / span).clamp(0.0, 1.0);
    let mut out = [0u8; 3];
    for (channel, slot) in out.iter_mut().enumerate() {
        let value = lo.1[channel] + (hi.1[channel] - lo.1[channel]) * local;
        let lifted = value + shimmer * 255.0;
        *slot = lifted.clamp(0.0, 255.0) as u8;
    }
    out
}

/// Convert an HSV triple (each in 0..1) to 8-bit RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let sector = h.floor() as i32;
    let frac = h - sector as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * frac);
    let t = v * (1.0 - s * (1.0 - frac));
    let (r, g, b) = match sector.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).clamp(0.0, 255.0) as u8,
        (g * 255.0).clamp(0.0, 255.0) as u8,
        (b * 255.0).clamp(0.0, 255.0) as u8,
    ]
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
