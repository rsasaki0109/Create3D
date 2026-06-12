//! Render Hardware Interface traits for `Create3D`.
//!
//! Renderer and viewport crates depend on this module, not on a specific GPU backend.

#![warn(missing_docs)]

mod error;
mod types;

pub use error::{RhiError, RhiResult};
pub use types::{
    BufferHandle, Color, DepthStencilFormat, Extent2D, IndexFormat, PipelineHandle,
    RenderTargetHandle, ShaderHandle, TextureFormat, VertexFormat,
};

/// Describes a GPU buffer uploaded once at creation time.
#[derive(Debug, Clone)]
pub struct BufferInit<'a> {
    /// Debug label.
    pub label: &'a str,
    /// Initial contents.
    pub contents: &'a [u8],
    /// Usage flags encoded as backend-specific bits by the implementation.
    pub vertex: bool,
    /// Index buffer usage.
    pub index: bool,
    /// Uniform buffer usage.
    pub uniform: bool,
}

/// Vertex attribute layout for pipeline creation.
#[derive(Debug, Clone, Copy)]
pub struct VertexAttribute {
    /// Shader location index.
    pub location: u32,
    /// Attribute format.
    pub format: VertexFormat,
    /// Byte offset within the vertex.
    pub offset: u64,
}

/// Vertex buffer layout description.
#[derive(Debug, Clone)]
pub struct VertexBufferLayout {
    /// Array stride in bytes.
    pub array_stride: u64,
    /// Vertex attributes.
    pub attributes: Vec<VertexAttribute>,
}

/// Render pipeline creation parameters.
#[derive(Debug, Clone)]
pub struct RenderPipelineDesc<'a> {
    /// Debug label.
    pub label: &'a str,
    /// Vertex shader WGSL source.
    pub vertex_shader: &'a str,
    /// Fragment shader WGSL source.
    pub fragment_shader: &'a str,
    /// Vertex buffer layouts.
    pub vertex_layouts: Vec<VertexBufferLayout>,
    /// Target texture format.
    pub color_format: TextureFormat,
    /// Depth format when enabled.
    pub depth_format: Option<DepthStencilFormat>,
    /// Whether depth testing is enabled.
    pub depth_write: bool,
    /// Primitive topology for draw calls.
    pub topology: PrimitiveTopology,
    /// Whether back-face culling is enabled.
    pub cull_back_faces: bool,
}

/// Primitive topology for render pipelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    /// Triangle list topology.
    TriangleList,
    /// Line list topology.
    LineList,
    /// Point list topology.
    PointList,
}

/// GPU backend abstraction used by viewport and renderer crates.
pub trait RhiBackend {
    /// Create a shader module from WGSL source.
    fn create_shader(&mut self, label: &str, source: &str) -> RhiResult<ShaderHandle>;

    /// Create a buffer initialized with data.
    fn create_buffer_init(&mut self, desc: BufferInit<'_>) -> RhiResult<BufferHandle>;

    /// Create a render pipeline.
    fn create_render_pipeline(&mut self, desc: RenderPipelineDesc<'_>)
        -> RhiResult<PipelineHandle>;

    /// Create an offscreen render target.
    fn create_render_target(
        &mut self,
        label: &str,
        extent: Extent2D,
        color_format: TextureFormat,
        depth_format: Option<DepthStencilFormat>,
    ) -> RhiResult<RenderTargetHandle>;
}
