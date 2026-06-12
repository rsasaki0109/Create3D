use c3d_rhi::{DepthStencilFormat, IndexFormat, PrimitiveTopology, TextureFormat, VertexFormat};

pub(crate) fn map_topology(topology: PrimitiveTopology) -> wgpu::PrimitiveTopology {
    match topology {
        PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        PrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
        PrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
    }
}

pub(crate) fn map_texture_format(format: TextureFormat) -> wgpu::TextureFormat {
    match format {
        TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
    }
}

pub(crate) fn map_depth_format(format: DepthStencilFormat) -> wgpu::TextureFormat {
    match format {
        DepthStencilFormat::Depth24Plus => wgpu::TextureFormat::Depth24Plus,
    }
}

pub(crate) fn map_vertex_format(format: VertexFormat) -> wgpu::VertexFormat {
    match format {
        VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
        VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
        VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
    }
}

pub(crate) fn map_index_format(format: IndexFormat) -> wgpu::IndexFormat {
    match format {
        IndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
        IndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
    }
}
