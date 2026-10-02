//! The only place the renderer meets the UI framework (DECISIONS D01): painting tessellated
//! egui output over the canvas, into a window surface or an offscreen target.

mod painter;

pub use painter::{UiFrame, UiPainter};
