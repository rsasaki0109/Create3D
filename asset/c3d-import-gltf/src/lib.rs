//! glTF/GLB import for Create3D.

#![warn(missing_docs)]

mod error;
mod import;
mod scene_ops;

pub use error::{ImportError, ImportResult};
pub use import::{
    import_gltf_bytes, import_gltf_path, GltfImportResult, ImportedMaterial, ImportedMesh,
    ImportedNode, ImportedTexture,
};
pub use scene_ops::import_result_to_scene_operations;
