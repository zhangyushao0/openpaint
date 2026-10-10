//! Derived rendering on wgpu: stroke programs, region evaluation, export, picking, previews.
//! Everything here can be dropped and rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
#![forbid(unsafe_code)]

mod active;
mod active_snapshot;
mod adapter;
mod affine;
mod change;
mod core;
mod curve;
mod edits;
mod error;
mod evaluator;
mod export;
mod fill_edge;
mod filter;
mod frame;
mod geom;
mod gpu;
mod gradient;
mod image;
mod output;
mod paper;
mod plan;
mod preview;
mod program;
mod protocol;
mod readback;
mod sample;
mod scene;
mod scene_adjust;
mod scene_cleared;
mod scene_content;
mod scene_layer;
mod screen;
mod shapes;
mod smudge;
mod snapshot;
mod stroke;
mod stroke_path;
mod stroke_stamp;
mod surface;
mod thumbnail;
mod view_atlas;
mod view_fill;
mod view_needed;
mod view_shown;
mod view_tier;
mod view_tiles;

pub use crate::adapter::{
    CHROME_ISLANDS, CHROME_PANELS, ChromePanel, ChromeShapes, ScreenFrame, ScreenPainter, UiFrame,
    chrome_shape,
};
pub use crate::change::changed_regions;
pub use crate::core::Core;
pub use crate::error::RenderError;
pub use crate::evaluator::{CanvasTarget, EvaluationWork, RegionEvaluator};
pub use crate::export::{Banding, Bands};
pub use crate::frame::FrameRequest;
pub use crate::geom::PixelRect;
pub use crate::image::RgbaImage;
pub use crate::output::MAX_SCALE;
pub use crate::screen::{BrushCursor, CanvasPlacement, ScreenTarget};
pub use crate::stroke::{curve_pieces, stroke_region, tip_radius};
pub use crate::surface::{Surface, SurfaceWindow};
pub use crate::thumbnail::{thumbnail_reference, thumbnail_size};
pub use crate::view_tier::{ViewCache, ViewFrame, ViewWork};
