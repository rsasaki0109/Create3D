//! Shared editor state and command registry for Create3D desktop tools.

#![warn(missing_docs)]

mod commands;
mod selection;

pub use commands::{CommandRegistry, EditorCommand};
pub use selection::SelectionState;
