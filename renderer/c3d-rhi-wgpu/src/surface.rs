//! Swapchain frame resources and shared GPU handle views.

/// Swapchain frame wrapper.
pub struct SurfaceFrame {
    texture: wgpu::SurfaceTexture,
}

impl SurfaceFrame {
    pub(crate) fn new(texture: wgpu::SurfaceTexture) -> Self {
        Self { texture }
    }

    /// Create a view for rendering into the swapchain image.
    pub fn view(&self) -> wgpu::TextureView {
        self.texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Present the frame to the window surface.
    pub fn present(self) {
        self.texture.present();
    }
}

/// GPU handles exposed for UI bootstrap code.
pub struct WgpuHandles<'a> {
    /// wgpu device.
    pub device: &'a wgpu::Device,
    /// wgpu queue.
    pub queue: &'a wgpu::Queue,
    /// Surface texture format.
    pub surface_format: wgpu::TextureFormat,
}

/// Color and depth views for an offscreen viewport target.
pub struct RenderTargetResources {
    /// Color texture view.
    pub color_view: wgpu::TextureView,
    /// Depth texture view when allocated.
    pub depth_view: Option<wgpu::TextureView>,
    /// Color texture for egui registration.
    pub color_texture: wgpu::Texture,
}
