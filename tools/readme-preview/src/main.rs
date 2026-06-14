//! Offscreen renderer that captures the real Create3D viewport for README assets.

use std::fs;
use std::path::{Path, PathBuf};

use ab_glyph::{point, Font, FontRef, PxScale};
use c3d_asset_db::AssetDb;
mod preview_scene;

use c3d_ecs::{project_scene_to_ecs, RuntimeWorld};
use c3d_rhi::Extent2D;
use c3d_rhi_wgpu::WgpuBackend;
use c3d_viewport::{
    MeshGpuCache, OrbitCamera, PointCloudGpuCache, SplatGpuCache, ViewportRenderer,
    ViewportShadingMode,
};
use gif::{DisposalMethod, Encoder, Frame, Repeat};
use image::{ImageBuffer, Rgba, RgbaImage};
use preview_scene::{build_preview_scene, PreviewLabels};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const LEFT_PANEL: u32 = 220;
const RIGHT_PANEL: u32 = 260;
const TOP_BAR: u32 = 36;
const VIEWPORT_W: u32 = WIDTH - LEFT_PANEL - RIGHT_PANEL;
const VIEWPORT_H: u32 = HEIGHT - TOP_BAR;

fn main() {
    let root = workspace_root();
    let assets_dir = root.join("docs/assets");
    fs::create_dir_all(&assets_dir).expect("create assets dir");

    let png_path = assets_dir.join("editor-preview.png");
    let gif_path = assets_dir.join("editor-preview.gif");

    let (frames, labels) = render_orbit_frames(36);
    let composed = compose_editor_frame(&frames[0], &labels);
    composed.save(&png_path).expect("write editor preview png");
    write_gif(&gif_path, &frames, &labels);
    println!("Wrote {}", png_path.display());
    println!("Wrote {}", gif_path.display());
}

fn render_orbit_frames(frame_count: usize) -> (Vec<RgbaImage>, PreviewLabels) {
    let mut backend = WgpuBackend::headless().expect("headless wgpu backend");
    let extent = Extent2D {
        width: VIEWPORT_W,
        height: VIEWPORT_H,
    };
    let viewport = ViewportRenderer::new(&mut backend, extent).expect("viewport renderer");
    let mut mesh_cache = MeshGpuCache::default();
    let mut point_cloud_cache = PointCloudGpuCache::default();
    let mut splat_cache = SplatGpuCache::default();

    let temp = std::env::temp_dir().join("create3d-readme-preview-assets");
    let _ = fs::create_dir_all(&temp);
    let mut assets = AssetDb::open(&temp).expect("open temp asset db");
    let (scene, labels) = build_preview_scene(&mut assets);

    let mut runtime = RuntimeWorld::new();
    project_scene_to_ecs(&scene, &mut runtime);

    let mut camera = OrbitCamera {
        target: c3d_core::math::Vec3::new(0.05, 0.85, 0.0),
        distance: 6.6,
        pitch: 0.42,
        ..OrbitCamera::default()
    };

    let mut frames = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        camera.yaw = (frame as f32 / frame_count as f32) * std::f32::consts::TAU + 0.6;
        viewport
            .render(
                &mut backend,
                &camera,
                &mut runtime,
                &assets,
                &mut mesh_cache,
                &mut point_cloud_cache,
                &mut splat_cache,
                ViewportShadingMode::Material,
            )
            .expect("render viewport frame");

        let (read_extent, pixels) = backend
            .read_render_target_rgba8(viewport.target())
            .expect("read viewport pixels");
        frames.push(rgba8_to_image(
            read_extent.width,
            read_extent.height,
            &pixels,
        ));
    }
    (frames, labels)
}

fn rgba8_to_image(width: u32, height: u32, pixels: &[u8]) -> RgbaImage {
    ImageBuffer::from_fn(width, height, |x, y| {
        let i = ((y * width + x) * 4) as usize;
        Rgba([pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]])
    })
}

fn compose_editor_frame(viewport: &RgbaImage, labels: &PreviewLabels) -> RgbaImage {
    let mut canvas = ImageBuffer::from_pixel(WIDTH, HEIGHT, Rgba([26, 26, 26, 255]));
    fill_rect(&mut canvas, 0, 0, WIDTH, TOP_BAR, Rgba([34, 34, 38, 255]));
    fill_rect(
        &mut canvas,
        0,
        TOP_BAR,
        LEFT_PANEL,
        HEIGHT,
        Rgba([30, 30, 34, 255]),
    );
    fill_rect(
        &mut canvas,
        WIDTH - RIGHT_PANEL,
        TOP_BAR,
        RIGHT_PANEL,
        HEIGHT - TOP_BAR,
        Rgba([30, 30, 34, 255]),
    );
    blit(&mut canvas, LEFT_PANEL, TOP_BAR, viewport);

    let font = load_font();
    let top_bar = format!(
        "Create3D  |  RGB  |  {} pts  |  {}",
        labels.point_count, labels.entity_name
    );
    draw_text(
        &mut canvas,
        &font,
        12.0,
        10.0,
        16.0,
        &top_bar,
        Rgba([210, 210, 215, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        12.0,
        (TOP_BAR + 14) as f32,
        15.0,
        "Hierarchy",
        Rgba([180, 180, 190, 255]),
    );
    fill_rect(
        &mut canvas,
        8,
        TOP_BAR + 38,
        LEFT_PANEL - 16,
        22,
        Rgba([55, 95, 145, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        16.0,
        (TOP_BAR + 42) as f32,
        14.0,
        &labels.entity_name,
        Rgba([240, 240, 245, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        (WIDTH - RIGHT_PANEL + 12) as f32,
        (TOP_BAR + 14) as f32,
        15.0,
        "Inspector",
        Rgba([180, 180, 190, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        (WIDTH - RIGHT_PANEL + 12) as f32,
        (TOP_BAR + 44) as f32,
        13.0,
        "Point Cloud",
        Rgba([150, 150, 160, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        (WIDTH - RIGHT_PANEL + 12) as f32,
        (TOP_BAR + 64) as f32,
        13.0,
        "Color Mode: RGB",
        Rgba([120, 120, 130, 255]),
    );
    draw_text(
        &mut canvas,
        &font,
        (WIDTH - RIGHT_PANEL + 12) as f32,
        (TOP_BAR + 84) as f32,
        13.0,
        &format!("Points: {}", labels.point_count),
        Rgba([120, 120, 130, 255]),
    );
    canvas
}

fn write_gif(path: &Path, viewport_frames: &[RgbaImage], labels: &PreviewLabels) {
    let composed_frames: Vec<RgbaImage> = viewport_frames
        .iter()
        .map(|frame| compose_editor_frame(frame, labels))
        .collect();
    let mut encoder = Encoder::new(
        std::fs::File::create(path).expect("create gif"),
        WIDTH as u16,
        HEIGHT as u16,
        &[],
    )
    .expect("gif encoder");
    encoder.set_repeat(Repeat::Infinite).expect("gif repeat");
    for image in composed_frames {
        let mut raw = image.into_raw();
        let mut frame = Frame::from_rgba(WIDTH as u16, HEIGHT as u16, &mut raw);
        frame.delay = 8;
        frame.dispose = DisposalMethod::Keep;
        encoder.write_frame(&frame).expect("write gif frame");
    }
}

fn fill_rect(canvas: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for py in y..y.saturating_add(h).min(canvas.height()) {
        for px in x..x.saturating_add(w).min(canvas.width()) {
            canvas.put_pixel(px, py, color);
        }
    }
}

fn blit(canvas: &mut RgbaImage, x: u32, y: u32, source: &RgbaImage) {
    for py in 0..source.height() {
        for px in 0..source.width() {
            if x + px < canvas.width() && y + py < canvas.height() {
                canvas.put_pixel(x + px, y + py, *source.get_pixel(px, py));
            }
        }
    }
}

fn load_font() -> FontRef<'static> {
    FontRef::try_from_slice(include_bytes!("DejaVuSans.ttf")).expect("embedded font")
}

fn draw_text(
    canvas: &mut RgbaImage,
    font: &FontRef<'_>,
    x: f32,
    y: f32,
    size: f32,
    text: &str,
    color: Rgba<u8>,
) {
    let scale = PxScale::from(size);
    let scaled = font.as_scaled(scale);
    let mut pen_x = x;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(glyph) =
            font.outline_glyph(glyph_id.with_scale_and_position(scale, point(pen_x, y)))
        {
            glyph.draw(|gx, gy, alpha| {
                if alpha <= 0.01 {
                    return;
                }
                let px = gx as i32;
                let py = gy as i32;
                if px >= 0
                    && py >= 0
                    && (px as u32) < canvas.width()
                    && (py as u32) < canvas.height()
                {
                    let a = (alpha * color[3] as f32) as u8;
                    let blended = Rgba([color[0], color[1], color[2], a]);
                    let dst = canvas.get_pixel(px as u32, py as u32);
                    canvas.put_pixel(px as u32, py as u32, alpha_blend(*dst, blended));
                }
            });
        }
        pen_x += ab_glyph::ScaleFont::h_advance(&scaled, glyph_id);
    }
}

fn alpha_blend(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let blend = |s: u8, d: u8| ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a) as u8;
    Rgba([
        blend(src[0], dst[0]),
        blend(src[1], dst[1]),
        blend(src[2], dst[2]),
        (out_a * 255.0) as u8,
    ])
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|tools| tools.parent())
        .expect("readme-preview lives at tools/readme-preview")
        .to_path_buf()
}
