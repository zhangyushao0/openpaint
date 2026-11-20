//! Shared interface: panels, input ownership, canvas arbitration. Reads the app projection,
//! keeps UI-local state, emits intents. Never writes artwork or GPU state directly. A mode's
//! screens come in through hooks (`hooks.rs`, D112); the ui names no mode.
//!
//! No direct GPU API, no file system (`cargo repo-check`).
//!
//! The screens (ARCHITECTURE section 10 for the conventions), each file's `//!` saying what it
//! holds: the canvas's shell (`shell*.rs`, `hooks.rs`, `canvas_input.rs`, `side_bar.rs`,
//! `notices.rs`, `pen_marks.rs`, `sheet.rs`: the canvas's shadow on the desk and the view status,
//! `taps.rs`, `panels.rs`); its panels (`actions_panel.rs`, `pen_buttons_page.rs`,
//! `adjust_panel.rs`, `adjust_controls.rs`, `color_panel.rs`, `color_palette.rs`, `layer_panel.rs`,
//! `layer_rows.rs`, `layer_options.rs`, `layer_drag.rs`, `brush_panel.rs`, `brush_list.rs`,
//! `brush_editor.rs`, `brush_settings.rs`, `brush_dynamics.rs`, `brush_strip.rs`; `row_drag.rs`
//! carries a held row of layers or sets); its bars (`selection_bar.rs`, `transform_bar.rs`,
//! `transform_drag.rs`, `shape_bar.rs`, `shape_drag.rs`, `copy_bar.rs`, `drop_bar.rs`,
//! `color_drop.rs`); the reference window (`reference_window.rs`, `reference_touch.rs`); the home
//! (`home/`, D116; `dates.rs` names the days); the shared widgets (`components/`), the theme
//! (`design/`) and every word shown (`text/`, D100); the replay's screen and its video's last line
//! (`replay/`, S163).
#![forbid(unsafe_code)]

mod actions_panel;
mod adjust_controls;
mod adjust_panel;
mod brush_dynamics;
mod brush_editor;
mod brush_list;
mod brush_panel;
mod brush_settings;
mod brush_strip;
mod canvas_input;
mod color_drop;
mod color_palette;
mod color_panel;
pub mod components;
mod copy_bar;
pub mod dates;
pub mod design;
mod drop_bar;
mod home;
mod hooks;
mod layer_drag;
mod layer_options;
mod layer_panel;
mod layer_rows;
mod notices;
mod panels;
mod pen_buttons_page;
mod pen_marks;
mod reference_touch;
mod reference_window;
mod replay;
mod row_drag;
mod selection_bar;
mod shape_bar;
mod shape_drag;
mod sheet;
mod shell;
mod side_bar;
pub mod taps;
pub mod text;
mod transform_bar;
mod transform_drag;

pub use crate::brush_strip::StripStrokes;
pub use crate::components::{CanvasArea, Control, ControlMap, ScreenArea, TOP_BAND, Unresolved};
pub use crate::hooks::{CanvasHook, Chrome, HomeHook, Hooks, Page};
pub use crate::replay::{caption_band, video_caption};
pub use crate::shell::Shell;
