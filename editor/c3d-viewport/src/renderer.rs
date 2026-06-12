use std::mem;

use c3d_asset_db::AssetDb;
use c3d_core::math::{Mat4, Vec3};
use c3d_ecs::{RenderMeshKind, RuntimeWorld, SceneDrawable};
use c3d_rhi::{
    BufferHandle, BufferInit, Color, DepthStencilFormat, Extent2D, IndexFormat, PipelineHandle,
    RenderTargetHandle, RhiBackend, TextureFormat,
};
use c3d_rhi_wgpu::WgpuBackend;

use crate::camera::OrbitCamera;
use crate::gizmo::gizmo_vertices;
use crate::mesh::{
    axis_vertices, cube_indices, cube_vertices, cube_wireframe_vertices, grid_vertices, Vertex,
};
use crate::mesh_cache::MeshGpuCache;
use crate::mode::ViewportShadingMode;
use crate::point_cloud_cache::PointCloudGpuCache;
use crate::shaders::{
    line_pipeline_desc, mesh_pipeline_desc, mesh_vertex_layout, point_pipeline_desc,
};

/// Viewport renderer that draws grid, axes, and placeholder scene cubes.
pub struct ViewportRenderer {
    target: RenderTargetHandle,
    extent: Extent2D,
    line_pipeline: PipelineHandle,
    mesh_pipeline: PipelineHandle,
    point_pipeline: PipelineHandle,
    grid_buffer: BufferHandle,
    axis_buffer: BufferHandle,
    cube_buffer: BufferHandle,
    cube_index_buffer: BufferHandle,
    cube_wireframe_buffer: BufferHandle,
    gizmo_buffer: BufferHandle,
    grid_vertex_count: u32,
    axis_vertex_count: u32,
    gizmo_vertex_count: u32,
    cube_wireframe_vertex_count: u32,
}

#[allow(clippy::too_many_arguments)]
impl ViewportRenderer {
    /// Initialize viewport GPU resources.
    pub fn new(backend: &mut WgpuBackend, initial_extent: Extent2D) -> c3d_rhi::RhiResult<Self> {
        let color_format = TextureFormat::Rgba8UnormSrgb;
        let target = backend.create_render_target(
            "viewport",
            initial_extent,
            color_format,
            Some(DepthStencilFormat::Depth24Plus),
        )?;

        let line_pipeline = backend
            .create_render_pipeline(line_pipeline_desc("viewport-line-pipeline", color_format))?;
        let mesh_pipeline = backend
            .create_render_pipeline(mesh_pipeline_desc("viewport-mesh-pipeline", color_format))?;
        let point_pipeline = backend
            .create_render_pipeline(point_pipeline_desc("viewport-point-pipeline", color_format))?;

        let grid_vertices = grid_vertices(10, 1.0);
        let axis_vertices = axis_vertices();
        let cube_vertices = cube_vertices();
        let cube_wireframe_vertices = cube_wireframe_vertices();
        let cube_indices = cube_indices();

        let grid_buffer = backend.create_buffer_init(BufferInit {
            label: "grid-vertices",
            contents: bytes_of_slice(&grid_vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;
        let axis_buffer = backend.create_buffer_init(BufferInit {
            label: "axis-vertices",
            contents: bytes_of_slice(&axis_vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;
        let cube_buffer = backend.create_buffer_init(BufferInit {
            label: "cube-vertices",
            contents: bytes_of_slice(&cube_vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;
        let cube_index_buffer = backend.create_buffer_init(BufferInit {
            label: "cube-indices",
            contents: bytes_of_slice(&cube_indices),
            vertex: false,
            index: true,
            uniform: false,
        })?;
        let cube_wireframe_buffer = backend.create_buffer_init(BufferInit {
            label: "cube-wireframe",
            contents: bytes_of_slice(&cube_wireframe_vertices),
            vertex: true,
            index: false,
            uniform: false,
        })?;
        let gizmo_buffer = backend.create_buffer_init(BufferInit {
            label: "gizmo-vertices",
            contents: bytes_of_slice(&gizmo_vertices(Vec3::ZERO, 1.0)),
            vertex: true,
            index: false,
            uniform: false,
        })?;

        Ok(Self {
            target,
            extent: initial_extent,
            line_pipeline,
            mesh_pipeline,
            point_pipeline,
            grid_buffer,
            axis_buffer,
            cube_buffer,
            cube_index_buffer,
            cube_wireframe_buffer,
            gizmo_buffer,
            grid_vertex_count: grid_vertices.len() as u32,
            axis_vertex_count: axis_vertices.len() as u32,
            gizmo_vertex_count: 0,
            cube_wireframe_vertex_count: cube_wireframe_vertices.len() as u32,
        })
    }

    /// Upload translate gizmo vertices for the selected entity origin.
    pub fn prepare_gizmo(
        &mut self,
        backend: &WgpuBackend,
        origin: Option<Vec3>,
    ) -> c3d_rhi::RhiResult<()> {
        if let Some(origin) = origin {
            let vertices = gizmo_vertices(origin, 1.0);
            self.gizmo_vertex_count = vertices.len() as u32;
            backend.write_buffer(self.gizmo_buffer, bytes_of_slice(&vertices));
        } else {
            self.gizmo_vertex_count = 0;
        }
        Ok(())
    }

    /// Resize the viewport render target.
    pub fn resize(
        &mut self,
        backend: &mut WgpuBackend,
        extent: Extent2D,
    ) -> c3d_rhi::RhiResult<()> {
        if extent == self.extent {
            return Ok(());
        }
        backend.resize_render_target(self.target, extent)?;
        self.extent = extent;
        Ok(())
    }

    /// Render target handle for UI registration.
    pub fn target(&self) -> RenderTargetHandle {
        self.target
    }

    /// Current viewport extent in pixels.
    pub fn extent(&self) -> Extent2D {
        self.extent
    }

    /// Render the viewport into the offscreen target.
    pub fn render(
        &self,
        backend: &mut WgpuBackend,
        camera: &OrbitCamera,
        runtime: &mut RuntimeWorld,
        assets: &AssetDb,
        mesh_cache: &mut MeshGpuCache,
        point_cloud_cache: &mut PointCloudGpuCache,
        shading_mode: ViewportShadingMode,
    ) -> c3d_rhi::RhiResult<()> {
        let aspect = self.extent.width as f32 / self.extent.height.max(1) as f32;
        let view_proj = camera.view_projection(aspect);
        let drawables = runtime.drawables();
        let point_cloud_drawables = runtime.point_cloud_drawables();
        let camera_position = camera.eye_position();

        for drawable in &drawables {
            if let RenderMeshKind::Asset(mesh_id) = drawable.mesh {
                mesh_cache.prepare(backend, assets, mesh_id, drawable.material_id, shading_mode)?;
            }
        }

        let mut point_cloud_batches = Vec::new();
        for drawable in &point_cloud_drawables {
            let draws = point_cloud_cache.prepare(
                backend,
                assets,
                drawable.point_cloud,
                drawable.world,
                camera_position,
            )?;
            point_cloud_batches.push((drawable.world, draws));
        }

        let line_pipeline = self.line_pipeline;
        let mesh_pipeline = self.mesh_pipeline;
        let point_pipeline = self.point_pipeline;
        let grid_buffer = self.grid_buffer;
        let axis_buffer = self.axis_buffer;
        let cube_buffer = self.cube_buffer;
        let cube_index_buffer = self.cube_index_buffer;
        let cube_wireframe_buffer = self.cube_wireframe_buffer;
        let gizmo_buffer = self.gizmo_buffer;
        let grid_vertex_count = self.grid_vertex_count;
        let axis_vertex_count = self.axis_vertex_count;
        let gizmo_vertex_count = self.gizmo_vertex_count;
        let cube_wireframe_vertex_count = self.cube_wireframe_vertex_count;

        backend.encode_viewport_draw(&self.target, Color::VIEWPORT_CLEAR, |backend, pass| {
            backend.pass_set_pipeline(pass, line_pipeline);
            backend.pass_set_vertex_buffer(pass, 0, grid_buffer);
            backend.pass_set_transform(pass, to_uniform(view_proj));
            backend.pass_draw(pass, grid_vertex_count);

            backend.pass_set_vertex_buffer(pass, 0, axis_buffer);
            backend.pass_set_transform(pass, to_uniform(view_proj));
            backend.pass_draw(pass, axis_vertex_count);

            if gizmo_vertex_count > 0 {
                backend.pass_set_vertex_buffer(pass, 0, gizmo_buffer);
                backend.pass_set_transform(pass, to_uniform(view_proj));
                backend.pass_draw(pass, gizmo_vertex_count);
            }

            backend.pass_set_pipeline(pass, mesh_pipeline);
            if shading_mode != ViewportShadingMode::Wireframe {
                for drawable in drawables.clone() {
                    match drawable {
                        SceneDrawable {
                            world,
                            mesh: RenderMeshKind::Cube,
                            ..
                        } => {
                            backend.pass_set_vertex_buffer(pass, 0, cube_buffer);
                            backend.pass_set_index_buffer(
                                pass,
                                cube_index_buffer,
                                IndexFormat::Uint16,
                            );
                            let mvp = view_proj * world;
                            backend.pass_set_transform(pass, to_uniform(mvp));
                            backend.pass_draw_indexed(pass, 36);
                        }
                        SceneDrawable {
                            world,
                            mesh: RenderMeshKind::Asset(mesh_id),
                            ..
                        } => {
                            if let Some(cached) =
                                mesh_cache.get(mesh_id, drawable.material_id, shading_mode)
                            {
                                backend.pass_set_vertex_buffer(pass, 0, cached.vertex_buffer);
                                backend.pass_set_index_buffer(
                                    pass,
                                    cached.index_buffer,
                                    cached.index_format,
                                );
                                let mvp = view_proj * world;
                                backend.pass_set_transform(pass, to_uniform(mvp));
                                backend.pass_draw_indexed(pass, cached.index_count);
                            }
                        }
                    }
                }
            }

            if shading_mode == ViewportShadingMode::Wireframe {
                backend.pass_set_pipeline(pass, line_pipeline);
                for drawable in drawables {
                    match drawable {
                        SceneDrawable {
                            world,
                            mesh: RenderMeshKind::Cube,
                            ..
                        } => {
                            backend.pass_set_vertex_buffer(pass, 0, cube_wireframe_buffer);
                            let mvp = view_proj * world;
                            backend.pass_set_transform(pass, to_uniform(mvp));
                            backend.pass_draw(pass, cube_wireframe_vertex_count);
                        }
                        SceneDrawable {
                            world,
                            mesh: RenderMeshKind::Asset(mesh_id),
                            ..
                        } => {
                            if let Some(cached) =
                                mesh_cache.get(mesh_id, drawable.material_id, shading_mode)
                            {
                                backend.pass_set_vertex_buffer(pass, 0, cached.wireframe_buffer);
                                let mvp = view_proj * world;
                                backend.pass_set_transform(pass, to_uniform(mvp));
                                backend.pass_draw(pass, cached.wireframe_vertex_count);
                            }
                        }
                    }
                }
            }

            backend.pass_set_pipeline(pass, point_pipeline);
            for (world, draws) in point_cloud_batches {
                for draw in draws {
                    backend.pass_set_vertex_buffer(pass, 0, draw.vertex_buffer);
                    let mvp = view_proj * world;
                    backend.pass_set_transform(pass, to_uniform(mvp));
                    backend.pass_draw(pass, draw.vertex_count);
                }
            }
        })
    }
}

fn bytes_of_slice<T: bytemuck::Pod>(data: &[T]) -> &[u8] {
    bytemuck::cast_slice(data)
}

fn to_uniform(matrix: Mat4) -> [[f32; 4]; 4] {
    matrix.to_cols_array_2d()
}

#[allow(dead_code)]
fn mesh_layout_bytes() -> c3d_rhi::VertexBufferLayout {
    mesh_vertex_layout()
}

#[allow(dead_code)]
fn assert_vertex_layout_matches() {
    let _ = mem::size_of::<Vertex>();
}
