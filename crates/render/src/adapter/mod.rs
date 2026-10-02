//! The only place the renderer meets the UI framework (DECISIONS D01): painting a screen frame,
//! the canvas with tessellated egui output over it, into a window surface or an offscreen
//! target.

mod painter;

pub use painter::{ScreenFrame, ScreenPainter, UiFrame};
