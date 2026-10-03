//! Derived rendering on wgpu: stroke programs, region evaluation, export, picking, previews.
//! Everything here can be dropped and rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
#![forbid(unsafe_code)]

mod active;
mod adapter;
mod change;
mod core;
mod error;
mod evaluator;
mod export;
mod frame;
mod geom;
mod gpu;
mod image;
mod plan;
mod program;
mod protocol;
mod readback;
mod scene;
mod screen;
mod stroke;
mod surface;
mod thumbnail;

pub use crate::adapter::{ScreenFrame, ScreenPainter, UiFrame};
pub use crate::change::changed_regions;
pub use crate::core::Core;
pub use crate::error::RenderError;
pub use crate::evaluator::{CanvasTarget, RegionEvaluator};
pub use crate::frame::FrameRequest;
pub use crate::geom::PixelRect;
pub use crate::image::RgbaImage;
pub use crate::screen::{CanvasPlacement, ScreenTarget};
pub use crate::stroke::stroke_region;
pub use crate::surface::{Surface, SurfaceWindow};
pub use crate::thumbnail::{thumbnail_reference, thumbnail_size};
