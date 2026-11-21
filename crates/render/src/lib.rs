//! Derived rendering on wgpu: an artwork's scene drawn by the raster core into the screen's canvas
//! and its view tiers, exports, picks, pictures and previews. Everything here can be dropped and
//! rebuilt from the artwork.
//!
//! Depends on `openpaint-core` only. The UI framework may appear only under `src/adapter/`
//! (`cargo repo-check`).
//!
//! Most modules lie at the root, a family told apart by its prefix (a structural slice ahead,
//! ROADMAP); a module with parts is a folder, its `mod.rs` and the parts it owns. The modules by
//! what they serve, each file's `//!` saying what it holds:
//! - The scene: `scene/` (`Scene::build_knowing`: visible layers, records, smudges, adjustments,
//!   filters, frames, moves and nested records as `Item`s and `Op`s in drawing order, with
//!   conservative canvas bounds) and `scene_*` (a layer, adjustments, `scene_content/` with nested
//!   content, cleared content, `scene_append/` strokes taken in place, spans reused from the scene
//!   before, `SceneOps` on the GPU), `edits.rs`, `stroke/` with `stroke_*.rs`, `curve/`,
//!   `fill_edge/` (D56), `shapes.rs`, `paper.rs`, `active/` (the tracked stroke), `change.rs`
//!   (`changed_regions`), `preview/`; `protocol/` owns `Op`: its operands, named slots and the WGSL
//!   prelude generated from them.
//! - The raster core (D93): `raster_scene/` (units per item), `raster_plan/` (a region's passes),
//!   `raster_lay.rs`, `raster_kinds.rs`, `raster/` (its frame, reads, slots and `batch.rs`, a
//!   frame's regions one command buffer), `raster_gpu.rs` (the shaders and pipelines),
//!   `raster_states.rs`, `raster_worker.rs` (what the runtime's raster worker draws), `geom.rs`,
//!   `output.rs` (a target's scale, D55), `affine.rs`.
//! - The canvas: `canvas/` (`CanvasTarget`; tiles handed to the raster worker, the kept scene and
//!   its retunes, residuals, the budget's rooms, the display cache's copies, the stroke being drawn
//!   from snapshots per output tile, D91).
//! - View tiers (D85, D93): `view_tier/` and `view_*.rs` (`ViewCache`, its tiles, atlases, kept
//!   tiers, fills, the blit's bindings).
//! - Readers: `export.rs`, `export_raster.rs`, `sample.rs`, `thumbnail.rs`, `readback.rs` (D64),
//!   `underlay.rs`, `difference.rs` (`image_difference` for tests and devtools), `image.rs`.
//! - Smudges and filters: `smudge/` (D46), `filter/` (D59).
//! - Pacing and memory: `pace/`, `gpu_clock.rs`, `frame_clock.rs`, `timestamps.rs`,
//!   `gpu_frames.rs`, `waiting.rs`, `tally.rs` (`Counts`), `profile.rs`, `budget.rs`.
//! - The screen: `screen.rs`, `surface.rs`, `gradient.rs`, `gpu.rs`, `core.rs`, `frame.rs`,
//!   `adapter/` (the UI framework's painter and the chrome pass).
//! - The replay's video: `video/` (`VideoTarget`: its frames drawn on a canvas of its own and
//!   scaled by area in integers, its end card; S163).
#![forbid(unsafe_code)]

mod active;
mod adapter;
mod affine;
mod budget;
mod canvas;
mod change;
mod core;
mod curve;
mod difference;
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
mod raster_worker;
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
mod timestamps;
mod underlay;
mod video;
mod view_atlas;
mod view_fill;
mod view_kept;
mod view_needed;
mod view_raster;
mod view_raster_fill;
mod view_shown;
mod view_tier;
mod view_tiles;
mod waiting;

pub use crate::adapter::{
    ChromePanel, ChromeShapes, ScreenFrame, ScreenPainter, UiFrame, chrome_shape,
};
pub use crate::budget::{CacheBytes, DeviceClass, GpuMemory, GpuObjects, OWN_BUDGET};
pub use crate::canvas::{CanvasCopy, CanvasTarget};
pub use crate::change::changed_regions;
pub use crate::core::Core;
pub use crate::difference::{ImageDifference, image_difference};
pub use crate::error::RenderError;
pub use crate::export::{Banding, Bands};
pub use crate::frame::FrameRequest;
pub use crate::geom::PixelRect;
pub use crate::image::RgbaImage;
pub use crate::pace::{BACKGROUND_NS, Background};
pub use crate::profile::{Attached, Spent, StrokeDrew, StrokeParts, take_spent};
pub use crate::raster_worker::{RasterAnswer, RasterDone, RasterDrawer, RasterJob, RasterOrder};
pub use crate::screen::{BrushCursor, CanvasPlacement, ScreenTarget};
pub use crate::stroke::{stroke_region, tip_radius};
pub use crate::surface::{Surface, SurfaceWindow};
pub use crate::tally::Counts;
pub use crate::thumbnail::{thumbnail_reference, thumbnail_size};
pub use crate::video::{VideoTarget, video_reference};
pub use crate::view_kept::HeldTier;
pub use crate::view_tier::{ViewCache, ViewFrame, ViewWork};
pub use crate::waiting::Focus;
