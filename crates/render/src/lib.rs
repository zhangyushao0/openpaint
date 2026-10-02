//! Derived rendering on wgpu: stroke programs, region evaluation, export, picking, previews.
//! Everything here can be dropped and rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
#![forbid(unsafe_code)]

mod adapter;
mod core;
mod error;
mod gpu;
mod image;
mod readback;
mod surface;

pub use crate::adapter::{UiFrame, UiPainter};
pub use crate::core::Core;
pub use crate::error::RenderError;
pub use crate::image::RgbaImage;
pub use crate::surface::{Surface, SurfaceWindow};
