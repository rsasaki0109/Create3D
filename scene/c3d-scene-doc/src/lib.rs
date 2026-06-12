//! Authoritative scene document (`SceneDB`) for `Create3D`.

#![warn(missing_docs)]

mod entity;
mod error;
mod scene_doc;
mod serialize;

pub use entity::{Entity, EntitySnapshot};
pub use error::{SceneError, SceneResult};
pub use scene_doc::SceneDoc;
pub use serialize::{SceneDocument, SceneEntityRecord};
