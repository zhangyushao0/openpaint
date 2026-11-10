//! The only place the renderer meets the UI framework: painting a screen frame,
//! the canvas with tessellated egui output over it, into a window surface or an offscreen
//! target, and the chrome's paper drawn inside that output (`chrome_shape`).

mod chrome;
mod painter;

pub use chrome::{ChromePanel, ChromeShapes, chrome_shape};
pub use painter::{ScreenFrame, ScreenPainter, UiFrame};
