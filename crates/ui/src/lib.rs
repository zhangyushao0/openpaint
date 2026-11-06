//! Shared interface: panels, input ownership, canvas arbitration. Reads the app projection,
//! keeps UI-local state, emits intents. Never writes artwork or GPU state directly.
//!
//! No direct GPU API, no file system (`cargo repo-check`).
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
pub mod design;
mod drop_bar;
mod gallery;
mod layer_drag;
mod layer_options;
mod layer_panel;
mod layer_rows;
mod notices;
mod panels;
mod pen_buttons_page;
mod pen_marks;
mod practice_bar;
mod practice_card;
mod practice_card_colour;
mod practice_card_disc;
mod practice_card_division;
mod practice_card_geometry;
mod practice_card_key;
mod practice_demo;
mod practice_demo_key;
mod practice_demo_marks;
mod practice_demo_wheel;
mod practice_division_marks;
mod practice_geometry_marks;
mod practice_key_marks;
mod practice_mark_lines;
mod practice_marks;
mod practice_masses_marks;
mod practice_object_marks;
mod practice_paint_marks;
mod practice_points_marks;
mod practice_solid_marks;
mod practice_tools;
mod reference_touch;
mod reference_window;
mod row_drag;
mod selection_bar;
mod shape_bar;
mod shape_drag;
mod sheet;
mod shell;
mod side_bar;
mod taps;
pub mod text;
mod transform_bar;
mod transform_drag;

pub use crate::brush_strip::StripStrokes;
pub use crate::components::{CanvasArea, Control, ControlMap, ScreenArea, TOP_BAND, Unresolved};
pub use crate::shell::Shell;
