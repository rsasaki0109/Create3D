//! 3D Gaussian splat PLY importer for Create3D.

#![warn(missing_docs)]

mod chunking;
mod error;
mod import;
mod synthetic;

pub use error::{ImportError, ImportResult};
pub use import::{
    import_gsplat_ply_bytes, import_gsplat_ply_path, looks_like_gsplat_ply, GsplatImportResult,
};
pub use synthetic::generate_synthetic_gaussian_splats;
