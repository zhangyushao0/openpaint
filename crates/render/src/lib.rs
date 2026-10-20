//! Derived rendering on wgpu: an artwork's scene drawn by the raster core into the screen's canvas
//! and its view tiers, exports, picks, pictures and previews. Everything here can be dropped and
//! rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
#![forbid(unsafe_code)]

mod active;
mod adapter;
mod affine;
mod budget;
mod canvas;
mod canvas_raster;
mod change;
mod core;
mod curve;
mod difference;
mod drawing;
mod edits;
mod error;
mod export;
mod export_raster;
mod fill_edge;
mod filter;
mod frame;
mod frame_clock;
mod geom;
mod gpu;
mod gpu_clock;
mod gpu_frames;
mod gradient;
mod image;
mod output;
mod pace;
mod paper;
mod preview;
mod profile;
mod protocol;
mod raster;
mod raster_gpu;
mod raster_kinds;
mod raster_lay;
mod raster_plan;
mod raster_scene;
mod raster_states;
mod readback;
mod sample;
mod scene;
mod scene_adjust;
mod scene_append;
mod scene_cleared;
mod scene_content;
mod scene_layer;
mod scene_ops;
mod scene_reuse;
mod screen;
mod shapes;
mod smudge;
mod stroke;
mod stroke_path;
mod stroke_stamp;
mod surface;
mod tally;
mod thumbnail;
mod underlay;
mod view_atlas;
mod view_fill;
mod view_kept;
mod view_needed;
mod view_raster;
mod view_raster_fill;
mod view_raster_jobs;
mod view_shown;
mod view_tier;
mod view_tiles;
mod waiting;

pub use crate::adapter::{
    CHROME_ISLANDS, CHROME_PANELS, ChromePanel, ChromeShapes, ScreenFrame, ScreenPainter, UiFrame,
    chrome_shape,
};
pub use crate::budget::{
    CacheBytes, DeviceClass, GpuMemory, OWN_BUDGET, SHARED_BUDGET, SHARED_PART,
};
pub use crate::canvas::{CanvasCopy, CanvasTarget};
pub use crate::change::changed_regions;
pub use crate::core::Core;
pub use crate::difference::{EDGE, ImageDifference, image_difference};
pub use crate::error::RenderError;
pub use crate::export::{Banding, Bands};
pub use crate::frame::FrameRequest;
pub use crate::geom::PixelRect;
pub use crate::image::RgbaImage;
pub use crate::output::MAX_SCALE;
pub use crate::pace::{BACKGROUND_NS, Background, IDLE_BACKGROUND_NS};
pub use crate::profile::{Spent, take_spent};
pub use crate::screen::{BrushCursor, CanvasPlacement, ScreenTarget};
pub use crate::stroke::{curve_pieces, stroke_region, tip_radius};
pub use crate::surface::{Surface, SurfaceWindow};
pub use crate::tally::Counts;
pub use crate::thumbnail::{thumbnail_reference, thumbnail_size};
pub use crate::view_kept::HeldTier;
pub use crate::view_raster_jobs::{RasterAnswer, RasterDone, RasterDrawer, RasterJob, RasterOrder};
pub use crate::view_tier::{ViewCache, ViewFrame, ViewWork};
pub use crate::waiting::Focus;
