use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

pub(crate) fn grid_vertices(half_extent: i32, step: f32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let color = [0.35, 0.37, 0.40, 1.0];
    for i in -half_extent..=half_extent {
        let pos = i as f32 * step;
        vertices.push(Vertex {
            position: [-half_extent as f32 * step, 0.0, pos],
            color,
        });
        vertices.push(Vertex {
            position: [half_extent as f32 * step, 0.0, pos],
            color,
        });
        vertices.push(Vertex {
            position: [pos, 0.0, -half_extent as f32 * step],
            color,
        });
        vertices.push(Vertex {
            position: [pos, 0.0, half_extent as f32 * step],
            color,
        });
    }
    vertices
}

pub(crate) fn axis_vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [0.0, 0.0, 0.0],
            color: [0.9, 0.2, 0.2, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 0.0],
            color: [0.9, 0.2, 0.2, 1.0],
        },
        Vertex {
            position: [0.0, 0.0, 0.0],
            color: [0.2, 0.85, 0.3, 1.0],
        },
        Vertex {
            position: [0.0, 1.0, 0.0],
            color: [0.2, 0.85, 0.3, 1.0],
        },
        Vertex {
            position: [0.0, 0.0, 0.0],
            color: [0.25, 0.45, 0.95, 1.0],
        },
        Vertex {
            position: [0.0, 0.0, 1.0],
            color: [0.25, 0.45, 0.95, 1.0],
        },
    ]
}

pub(crate) fn cube_vertices() -> Vec<Vertex> {
    let c = [0.82, 0.62, 0.25, 1.0];
    let mut vertices = Vec::new();
    let faces = [
        ([-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [-0.5, 0.5, 0.5]),
        ([0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]),
        ([-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, -0.5, -0.5]),
        ([0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]),
        ([-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, -0.5]),
        ([0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5]),
        ([-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, 0.5]),
        ([0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]),
        ([-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, -0.5]),
        ([-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]),
        ([0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, 0.5]),
        ([0.5, -0.5, 0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]),
    ];
    for (a, b, d) in faces {
        vertices.push(Vertex {
            position: a,
            color: c,
        });
        vertices.push(Vertex {
            position: b,
            color: c,
        });
        vertices.push(Vertex {
            position: d,
            color: c,
        });
    }
    vertices
}

pub(crate) fn cube_indices() -> Vec<u16> {
    (0..36).collect()
}

pub(crate) fn cube_wireframe_vertices() -> Vec<Vertex> {
    let c = [0.95, 0.95, 0.95, 1.0];
    let corners = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    edges
        .into_iter()
        .flat_map(|(a, b)| {
            [
                Vertex {
                    position: corners[a],
                    color: c,
                },
                Vertex {
                    position: corners[b],
                    color: c,
                },
            ]
        })
        .collect()
}
