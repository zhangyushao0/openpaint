//! Shared interface: panels, input ownership, canvas arbitration. Reads the app projection,
//! keeps UI-local state, emits intents. Never writes artwork or GPU state directly.
//!
//! No direct GPU API, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

pub mod components;
pub mod design;
mod layer_panel;
mod shell;

pub use crate::components::{ScreenArea, TOP_BAR_HEIGHT};
pub use crate::shell::Shell;
