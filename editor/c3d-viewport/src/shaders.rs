const VERTEX_SHADER: &str = r#"
struct Transform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = transform.mvp * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}
"#;

const FRAGMENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

pub(crate) fn line_pipeline_desc<'a>(
    label: &'a str,
    color_format: c3d_rhi::TextureFormat,
) -> c3d_rhi::RenderPipelineDesc<'a> {
    c3d_rhi::RenderPipelineDesc {
        label,
        vertex_shader: VERTEX_SHADER,
        fragment_shader: FRAGMENT_SHADER,
        vertex_layouts: vec![mesh_vertex_layout()],
        color_format,
        depth_format: Some(c3d_rhi::DepthStencilFormat::Depth24Plus),
        depth_write: true,
        topology: c3d_rhi::PrimitiveTopology::LineList,
        cull_back_faces: false,
        alpha_blend: false,
    }
}

pub(crate) fn point_pipeline_desc<'a>(
    label: &'a str,
    color_format: c3d_rhi::TextureFormat,
) -> c3d_rhi::RenderPipelineDesc<'a> {
    c3d_rhi::RenderPipelineDesc {
        label,
        vertex_shader: VERTEX_SHADER,
        fragment_shader: FRAGMENT_SHADER,
        vertex_layouts: vec![mesh_vertex_layout()],
        color_format,
        depth_format: Some(c3d_rhi::DepthStencilFormat::Depth24Plus),
        depth_write: true,
        topology: c3d_rhi::PrimitiveTopology::PointList,
        cull_back_faces: false,
        alpha_blend: false,
    }
}

pub(crate) fn splat_pipeline_desc<'a>(
    label: &'a str,
    color_format: c3d_rhi::TextureFormat,
) -> c3d_rhi::RenderPipelineDesc<'a> {
    c3d_rhi::RenderPipelineDesc {
        label,
        vertex_shader: VERTEX_SHADER,
        fragment_shader: FRAGMENT_SHADER,
        vertex_layouts: vec![mesh_vertex_layout()],
        color_format,
        depth_format: Some(c3d_rhi::DepthStencilFormat::Depth24Plus),
        depth_write: false,
        topology: c3d_rhi::PrimitiveTopology::TriangleList,
        cull_back_faces: false,
        alpha_blend: true,
    }
}

pub(crate) fn mesh_pipeline_desc<'a>(
    label: &'a str,
    color_format: c3d_rhi::TextureFormat,
) -> c3d_rhi::RenderPipelineDesc<'a> {
    c3d_rhi::RenderPipelineDesc {
        label,
        vertex_shader: VERTEX_SHADER,
        fragment_shader: FRAGMENT_SHADER,
        vertex_layouts: vec![mesh_vertex_layout()],
        color_format,
        depth_format: Some(c3d_rhi::DepthStencilFormat::Depth24Plus),
        depth_write: true,
        topology: c3d_rhi::PrimitiveTopology::TriangleList,
        cull_back_faces: true,
        alpha_blend: false,
    }
}

pub(crate) fn mesh_vertex_layout() -> c3d_rhi::VertexBufferLayout {
    c3d_rhi::VertexBufferLayout {
        array_stride: 28,
        attributes: vec![
            c3d_rhi::VertexAttribute {
                location: 0,
                format: c3d_rhi::VertexFormat::Float32x3,
                offset: 0,
            },
            c3d_rhi::VertexAttribute {
                location: 1,
                format: c3d_rhi::VertexFormat::Float32x4,
                offset: 12,
            },
        ],
    }
}
