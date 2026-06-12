/// RGBA color with linear components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component.
    pub r: f32,
    /// Green component.
    pub g: f32,
    /// Blue component.
    pub b: f32,
    /// Alpha component.
    pub a: f32,
}

impl Color {
    /// Dark viewport background.
    pub const VIEWPORT_CLEAR: Self = Self {
        r: 0.08,
        g: 0.09,
        b: 0.11,
        a: 1.0,
    };
}

/// 2D extent in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent2D {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Extent2D {
    /// Returns true when both dimensions are non-zero.
    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Opaque shader handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32);

/// Opaque buffer handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u32);

/// Opaque pipeline handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u32);

/// Opaque render target handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderTargetHandle(pub u32);

/// Supported color target formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// BGRA8 UNORM sRGB surface format.
    Bgra8UnormSrgb,
    /// RGBA8 UNORM sRGB offscreen format.
    Rgba8UnormSrgb,
}

/// Supported depth formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthStencilFormat {
    /// 24-bit depth, 8-bit stencil.
    Depth24Plus,
}

/// Vertex attribute formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    /// Two-component float vector.
    Float32x2,
    /// Three-component float vector.
    Float32x3,
    /// Four-component float vector.
    Float32x4,
}

/// Index buffer formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    /// 16-bit indices.
    Uint16,
    /// 32-bit indices.
    Uint32,
}
