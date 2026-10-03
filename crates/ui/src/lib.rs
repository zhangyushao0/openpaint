//! Shared interface: panels, input ownership, canvas arbitration. Reads the app projection,
//! keeps UI-local state, emits intents. Never writes artwork or GPU state directly.
//!
//! No direct GPU API, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod actions_panel;
mod brush_panel;
mod canvas_input;
mod color_drop;
mod color_panel;
pub mod components;
pub mod design;
mod drop_bar;
mod gallery;
mod layer_panel;
mod notices;
mod reference_window;
mod shell;
mod side_bar;

pub use crate::components::{CanvasArea, ScreenArea, TOP_BAND};
pub use crate::shell::Shell;
