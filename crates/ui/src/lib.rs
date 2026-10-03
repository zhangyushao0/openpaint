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
mod panels;
mod pen_marks;
mod reference_touch;
mod reference_window;
mod selection_bar;
mod sheet;
mod shell;
mod side_bar;
mod transform_bar;
mod transform_drag;

pub use crate::components::{CanvasArea, ScreenArea, TOP_BAND};
pub use crate::shell::Shell;
