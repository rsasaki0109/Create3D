//! STL mesh importer for Create3D.

#![warn(missing_docs)]

mod error;
mod import;

pub use error::{ImportError, ImportResult};
pub use import::{import_stl_bytes, import_stl_path};
