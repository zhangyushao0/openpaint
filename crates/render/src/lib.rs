//! Derived rendering on wgpu: stroke programs, region evaluation, export, picking, previews.
//! Everything here can be dropped and rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
#![forbid(unsafe_code)]

mod adapter;
mod core;
mod error;
mod evaluator;
mod frame;
mod geom;
mod gpu;
mod image;
mod plan;
mod program;
mod protocol;
mod readback;
mod scene;
mod surface;

pub use crate::adapter::{UiFrame, UiPainter};
pub use crate::core::Core;
pub use crate::error::RenderError;
pub use crate::evaluator::{CanvasTarget, RegionEvaluator};
pub use crate::frame::FrameRequest;
pub use crate::geom::PixelRect;
pub use crate::image::RgbaImage;
pub use crate::scene::stroke_region;
pub use crate::surface::{Surface, SurfaceWindow};
