use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use c3d_rhi::{
    BufferHandle, BufferInit, Color, DepthStencilFormat, Extent2D, IndexFormat, PipelineHandle,
    RenderPipelineDesc, RenderTargetHandle, RhiBackend, RhiError, RhiResult, ShaderHandle,
    TextureFormat,
};
use wgpu::util::DeviceExt;

use crate::convert::{
    map_depth_format, map_index_format, map_texture_format, map_topology, map_vertex_format,
};
use crate::surface::{RenderTargetResources, SurfaceFrame, WgpuHandles};

struct ShaderEntry {
    #[allow(dead_code)]
    module: wgpu::ShaderModule,
}

struct BufferEntry {
    buffer: wgpu::Buffer,
}

struct PipelineEntry {
    pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    bind_group_layout: wgpu::BindGroupLayout,
}

struct TargetEntry {
    resources: RenderTargetResources,
    extent: Extent2D,
    color_format: TextureFormat,
}

/// wgpu-backed RHI context.
pub struct WgpuBackend {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    next_id: AtomicU32,
    shaders: HashMap<u32, ShaderEntry>,
    buffers: HashMap<u32, BufferEntry>,
    pipelines: HashMap<u32, PipelineEntry>,
    targets: HashMap<u32, TargetEntry>,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl WgpuBackend {
    /// Create a backend and attach it to a visible window surface.
    pub fn from_window(window: std::sync::Arc<winit::window::Window>) -> RhiResult<Self> {
        pollster::block_on(Self::from_window_async(window))
    }

    async fn from_window_async(window: std::sync::Arc<winit::window::Window>) -> RhiResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| RhiError::Surface(err.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| RhiError::Initialization(err.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("create3d-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|err| RhiError::Initialization(err.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("transform-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transform-uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            instance,
            device,
            queue,
            surface: Some(surface),
            surface_config: Some(config),
            next_id: AtomicU32::new(1),
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            pipelines: HashMap::new(),
            targets: HashMap::new(),
            uniform_buffer,
            uniform_bind_group_layout,
        })
    }

    /// Create a backend without a window surface for offscreen rendering.
    pub fn headless() -> RhiResult<Self> {
        pollster::block_on(Self::headless_async())
    }

    async fn headless_async() -> RhiResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| RhiError::Initialization(err.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("create3d-headless-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|err| RhiError::Initialization(err.to_string()))?;

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("transform-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transform-uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            instance,
            device,
            queue,
            surface: None,
            surface_config: None,
            next_id: AtomicU32::new(1),
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            pipelines: HashMap::new(),
            targets: HashMap::new(),
            uniform_buffer,
            uniform_bind_group_layout,
        })
    }

    fn alloc_handle(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn buffer(&self, handle: BufferHandle) -> RhiResult<&wgpu::Buffer> {
        self.buffers
            .get(&handle.0)
            .map(|entry| &entry.buffer)
            .ok_or(RhiError::InvalidHandle)
    }

    fn pipeline_entry(&self, handle: PipelineHandle) -> RhiResult<&PipelineEntry> {
        self.pipelines.get(&handle.0).ok_or(RhiError::InvalidHandle)
    }

    fn target_entry(&self, handle: RenderTargetHandle) -> RhiResult<&TargetEntry> {
        self.targets.get(&handle.0).ok_or(RhiError::InvalidHandle)
    }

    fn target_entry_mut(&mut self, handle: RenderTargetHandle) -> RhiResult<&mut TargetEntry> {
        self.targets
            .get_mut(&handle.0)
            .ok_or(RhiError::InvalidHandle)
    }

    /// Resize the window surface when the window changes size.
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let (Some(surface), Some(config)) = (&self.surface, self.surface_config.as_mut()) {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// Acquire the next swapchain frame for UI composition.
    pub fn acquire_surface_frame(&self) -> RhiResult<SurfaceFrame> {
        let surface = self
            .surface
            .as_ref()
            .ok_or_else(|| RhiError::Surface("surface not configured".into()))?;
        let frame = surface
            .get_current_texture()
            .map_err(|err| RhiError::Surface(err.to_string()))?;
        Ok(SurfaceFrame::new(frame))
    }

    /// Submit command buffers to the GPU queue.
    pub fn submit(&self, commands: impl IntoIterator<Item = wgpu::CommandBuffer>) {
        self.queue.submit(commands);
    }

    /// Upload bytes into an existing buffer handle.
    pub fn write_buffer(&self, handle: BufferHandle, bytes: &[u8]) {
        if let Ok(buffer) = self.buffer(handle) {
            self.queue.write_buffer(buffer, 0, bytes);
        }
    }

    /// Borrow wgpu handles needed by UI bootstrap code.
    pub fn handles(&self) -> WgpuHandles<'_> {
        WgpuHandles {
            device: &self.device,
            queue: &self.queue,
            surface_format: self
                .surface_config
                .as_ref()
                .map(|config| config.format)
                .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb),
        }
    }

    /// Borrow GPU resources for a render target.
    pub fn target_resources(
        &self,
        handle: RenderTargetHandle,
    ) -> RhiResult<&RenderTargetResources> {
        Ok(&self.target_entry(handle)?.resources)
    }

    /// Resize an offscreen render target.
    pub fn resize_render_target(
        &mut self,
        handle: RenderTargetHandle,
        extent: Extent2D,
    ) -> RhiResult<()> {
        if !extent.is_valid() {
            return Ok(());
        }
        let entry = self.target_entry(handle)?;
        if entry.extent == extent {
            return Ok(());
        }
        let color_format = entry.color_format;
        let resources = create_target_resources(
            &self.device,
            "viewport-target",
            extent,
            color_format,
            Some(DepthStencilFormat::Depth24Plus),
        )?;
        let entry = self.target_entry_mut(handle)?;
        entry.resources = resources;
        entry.extent = extent;
        Ok(())
    }
}

impl RhiBackend for WgpuBackend {
    fn create_shader(&mut self, label: &str, source: &str) -> RhiResult<ShaderHandle> {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let id = self.alloc_handle();
        self.shaders.insert(id, ShaderEntry { module });
        Ok(ShaderHandle(id))
    }

    fn create_buffer_init(&mut self, desc: BufferInit<'_>) -> RhiResult<BufferHandle> {
        let mut usage = wgpu::BufferUsages::empty();
        if desc.vertex {
            usage |= wgpu::BufferUsages::VERTEX;
        }
        if desc.index {
            usage |= wgpu::BufferUsages::INDEX;
        }
        if desc.uniform {
            usage |= wgpu::BufferUsages::UNIFORM;
        }
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(desc.label),
                contents: desc.contents,
                usage,
            });
        let id = self.alloc_handle();
        self.buffers.insert(id, BufferEntry { buffer });
        Ok(BufferHandle(id))
    }

    fn create_render_pipeline(
        &mut self,
        desc: RenderPipelineDesc<'_>,
    ) -> RhiResult<PipelineHandle> {
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("generated-vertex"),
                source: wgpu::ShaderSource::Wgsl(desc.vertex_shader.into()),
            });
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("generated-fragment"),
                source: wgpu::ShaderSource::Wgsl(desc.fragment_shader.into()),
            });

        let mut attribute_storage = Vec::new();
        let mut layout_ranges = Vec::new();
        for layout in &desc.vertex_layouts {
            let start = attribute_storage.len();
            for attribute in &layout.attributes {
                attribute_storage.push(wgpu::VertexAttribute {
                    format: map_vertex_format(attribute.format),
                    offset: attribute.offset,
                    shader_location: attribute.location,
                });
            }
            layout_ranges.push((layout.array_stride, start..attribute_storage.len()));
        }
        let vertex_layouts: Vec<wgpu::VertexBufferLayout<'_>> = layout_ranges
            .iter()
            .map(|(array_stride, range)| wgpu::VertexBufferLayout {
                array_stride: *array_stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attribute_storage[range.clone()],
            })
            .collect();

        let bind_group_layout = self.uniform_bind_group_layout.clone();
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(desc.label),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let color_format = map_texture_format(desc.color_format);
        let mut depth_stencil = None;
        if let Some(depth_format) = desc.depth_format {
            depth_stencil = Some(wgpu::DepthStencilState {
                format: map_depth_format(depth_format),
                depth_write_enabled: desc.depth_write,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            });
        }

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(desc.label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: if desc.alpha_blend {
                            Some(wgpu::BlendState::ALPHA_BLENDING)
                        } else {
                            Some(wgpu::BlendState::REPLACE)
                        },
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: map_topology(desc.topology),
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: if desc.cull_back_faces {
                        Some(wgpu::Face::Back)
                    } else {
                        None
                    },
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let id = self.alloc_handle();
        self.pipelines.insert(
            id,
            PipelineEntry {
                pipeline,
                bind_group_layout,
            },
        );
        Ok(PipelineHandle(id))
    }

    fn create_render_target(
        &mut self,
        label: &str,
        extent: Extent2D,
        color_format: TextureFormat,
        depth_format: Option<DepthStencilFormat>,
    ) -> RhiResult<RenderTargetHandle> {
        let resources =
            create_target_resources(&self.device, label, extent, color_format, depth_format)?;
        let id = self.alloc_handle();
        self.targets.insert(
            id,
            TargetEntry {
                resources,
                extent,
                color_format,
            },
        );
        Ok(RenderTargetHandle(id))
    }
}

impl WgpuBackend {
    /// Encode and submit a viewport render pass.
    pub fn encode_viewport_draw(
        &self,
        target: &RenderTargetHandle,
        clear_color: Color,
        draw: impl FnOnce(&WgpuBackend, &mut wgpu::RenderPass<'_>),
    ) -> RhiResult<()> {
        let entry = self.target_entry(*target)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("viewport-encoder"),
            });

        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &entry.resources.color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear_color.r as f64,
                    g: clear_color.g as f64,
                    b: clear_color.b as f64,
                    a: clear_color.a as f64,
                }),
                store: wgpu::StoreOp::Store,
            },
        };

        let depth_attachment = entry.resources.depth_view.as_ref().map(|view| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport-pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            draw(self, &mut pass);
        }

        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Bind a pipeline on an active render pass.
    pub fn pass_set_pipeline(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: PipelineHandle) {
        if let Ok(entry) = self.pipeline_entry(pipeline) {
            pass.set_pipeline(&entry.pipeline);
        }
    }

    /// Bind a vertex buffer on an active render pass.
    pub fn pass_set_vertex_buffer(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: u32,
        buffer: BufferHandle,
    ) {
        if let Ok(buffer) = self.buffer(buffer) {
            pass.set_vertex_buffer(slot, buffer.slice(..));
        }
    }

    /// Bind an index buffer on an active render pass.
    pub fn pass_set_index_buffer(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        buffer: BufferHandle,
        format: IndexFormat,
    ) {
        if let Ok(buffer) = self.buffer(buffer) {
            pass.set_index_buffer(buffer.slice(..), map_index_format(format));
        }
    }

    /// Upload and bind a transform matrix on group 0.
    pub fn pass_set_transform(&self, pass: &mut wgpu::RenderPass<'_>, matrix: [[f32; 4]; 4]) {
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&matrix));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transform-bind-group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buffer.as_entire_binding(),
            }],
        });
        pass.set_bind_group(0, &bind_group, &[]);
    }

    /// Draw non-indexed geometry on an active render pass.
    pub fn pass_draw(&self, pass: &mut wgpu::RenderPass<'_>, vertices: u32) {
        pass.draw(0..vertices, 0..1);
    }

    /// Draw indexed geometry on an active render pass.
    pub fn pass_draw_indexed(&self, pass: &mut wgpu::RenderPass<'_>, indices: u32) {
        pass.draw_indexed(0..indices, 0, 0..1);
    }

    /// Read RGBA8 pixels from an offscreen render target after rendering.
    pub fn read_render_target_rgba8(
        &self,
        target: RenderTargetHandle,
    ) -> RhiResult<(Extent2D, Vec<u8>)> {
        let entry = self.target_entry(target)?;
        let width = entry.extent.width.max(1);
        let height = entry.extent.height.max(1);
        let bytes_per_row = width * 4;
        let aligned_bytes_per_row = bytes_per_row.div_ceil(256) * 256;
        let buffer_size = aligned_bytes_per_row * height;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &entry.resources.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait)
            .map_err(|err| RhiError::Initialization(err.to_string()))?;
        receiver
            .recv()
            .map_err(|err| RhiError::Initialization(err.to_string()))?
            .map_err(|err| RhiError::Initialization(err.to_string()))?;

        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * aligned_bytes_per_row) as usize;
            let end = start + bytes_per_row as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();

        Ok((entry.extent, pixels))
    }
}

fn create_target_resources(
    device: &wgpu::Device,
    label: &str,
    extent: Extent2D,
    color_format: TextureFormat,
    depth_format: Option<DepthStencilFormat>,
) -> RhiResult<RenderTargetResources> {
    let width = extent.width.max(1);
    let height = extent.height.max(1);
    let format = map_texture_format(color_format);

    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("{label}-color")),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let depth_view = depth_format.map(|format| {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}-depth")),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: map_depth_format(format),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    });

    Ok(RenderTargetResources {
        color_view,
        depth_view,
        color_texture,
    })
}
