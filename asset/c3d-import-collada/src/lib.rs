//! Collada (`.dae`) mesh importer for Create3D.

#![warn(missing_docs)]

mod error;
mod import;

pub use error::{ImportError, ImportResult};
pub use import::{import_collada_bytes, import_collada_path};
