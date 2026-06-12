//! Mesh authoring helpers: primitives, topology validation, and edit operations.

#![warn(missing_docs)]

mod ops;
mod primitives;
mod processing;
mod thumbnail;
mod topology;

pub use ops::subdivide_triangles;
pub use primitives::{plane, primitive, unit_cube, PrimitiveKind};
pub use processing::{compute_normals, compute_tangents};
pub use thumbnail::render_mesh_thumbnail_png;
pub use topology::{AuthoringMesh, TopologyError, TriangleTopology};
