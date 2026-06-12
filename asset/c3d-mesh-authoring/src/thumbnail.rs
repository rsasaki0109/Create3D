use c3d_asset_material::MaterialAssetData;
use c3d_asset_mesh::MeshAssetData;
use image::{ImageBuffer, Rgba};

/// Render a mesh/material thumbnail to PNG bytes using a CPU rasterizer.
pub fn render_mesh_thumbnail_png(
    mesh: &MeshAssetData,
    material: &MaterialAssetData,
    size: u32,
) -> Result<Vec<u8>, String> {
    mesh.validate().map_err(|err| err.to_string())?;
    let resolved = material.resolved().map_err(|err| err.to_string())?;
    let (min, max) = mesh
        .local_bounds()
        .ok_or_else(|| "mesh has no bounds".to_string())?;
    let width = size.max(16);
    let height = size.max(16);
    let mut image = ImageBuffer::from_pixel(width, height, Rgba([30, 30, 34, 255]));

    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let extent = [
        (max[0] - min[0]).max(0.01),
        (max[1] - min[1]).max(0.01),
        (max[2] - min[2]).max(0.01),
    ];
    let scale = 0.85 / extent[0].max(extent[1]).max(extent[2]);

    let color = resolved.base_color;
    let rgba = Rgba([
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ]);

    for triangle in mesh.indices.chunks_exact(3) {
        let points = [
            project(
                mesh.positions[triangle[0] as usize],
                center,
                scale,
                width,
                height,
            ),
            project(
                mesh.positions[triangle[1] as usize],
                center,
                scale,
                width,
                height,
            ),
            project(
                mesh.positions[triangle[2] as usize],
                center,
                scale,
                width,
                height,
            ),
        ];
        rasterize_triangle(&mut image, points, rgba);
    }

    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|err| err.to_string())?;
    Ok(bytes)
}

fn project(position: [f32; 3], center: [f32; 3], scale: f32, width: u32, height: u32) -> [i32; 2] {
    let x = (position[0] - center[0]) * scale;
    let y = (position[1] - center[1]) * scale;
    let px = ((x * 0.5 + 0.5) * width as f32).round() as i32;
    let py = ((0.5 - y * 0.5) * height as f32).round() as i32;
    [px, py]
}

fn rasterize_triangle(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    pts: [[i32; 2]; 3],
    color: Rgba<u8>,
) {
    let min_x = pts
        .iter()
        .map(|p| p[0])
        .min()
        .unwrap_or(0)
        .clamp(0, image.width() as i32 - 1);
    let max_x = pts
        .iter()
        .map(|p| p[0])
        .max()
        .unwrap_or(0)
        .clamp(0, image.width() as i32 - 1);
    let min_y = pts
        .iter()
        .map(|p| p[1])
        .min()
        .unwrap_or(0)
        .clamp(0, image.height() as i32 - 1);
    let max_y = pts
        .iter()
        .map(|p| p[1])
        .max()
        .unwrap_or(0)
        .clamp(0, image.height() as i32 - 1);

    let area = edge(pts[0], pts[1], pts[2]);
    if area == 0 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = [x, y];
            let w0 = edge(pts[1], pts[2], p);
            let w1 = edge(pts[2], pts[0], p);
            let w2 = edge(pts[0], pts[1], p);
            if (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0) {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn edge(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> i32 {
    (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
}

#[cfg(test)]
mod tests {
    use c3d_asset_material::MaterialGraphData;

    use super::*;

    #[test]
    fn thumbnail_png_has_signature() {
        let mesh = c3d_asset_mesh::MeshAssetData {
            version: 1,
            positions: vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]],
            normals: Vec::new(),
            uvs: Vec::new(),
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let material = MaterialAssetData {
            version: 1,
            base_color: [0.8, 0.2, 0.2, 1.0],
            base_color_texture: None,
            graph: Some(MaterialGraphData::from_base_color([0.8, 0.2, 0.2, 1.0])),
        };
        let png = render_mesh_thumbnail_png(&mesh, &material, 64).expect("thumbnail png");
        assert!(png.starts_with(b"\x89PNG"));
    }
}
