//! wgpu implementation of the Create3D RHI.

#![warn(missing_docs)]

mod backend;
mod convert;
mod surface;

pub use backend::WgpuBackend;
pub use surface::{RenderTargetResources, SurfaceFrame, WgpuHandles};
