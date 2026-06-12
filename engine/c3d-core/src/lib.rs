//! `Create3D` core primitives shared across the engine.
//!
//! This crate intentionally stays free of editor, renderer, scene, and AI dependencies.
//! See the architecture dependency rules in `docs/architecture/dependency-rules.md`.

#![warn(missing_docs)]

pub mod error;
pub mod id;
pub mod logging;
pub mod math;
pub mod version;

pub use error::{C3dError, C3dResult};
pub use id::{AssetId, EntityId, OperationId, TransactionId, UlidGenerator};
pub use logging::{init_logging, LoggingConfig};
pub use math as c3d_math;
pub use version::{C3D_API_VERSION, C3D_VERSION};
