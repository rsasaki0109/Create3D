//! Content-addressed asset storage for Create3D projects.

#![warn(missing_docs)]

mod blob;
mod db;
mod error;
mod hash;
mod index;

pub use blob::BlobStore;
pub use db::AssetDb;
pub use error::{AssetError, AssetResult};
pub use hash::ContentHash;
pub use index::{AssetEntry, AssetIndexDocument, AssetKind, AssetRecord};
