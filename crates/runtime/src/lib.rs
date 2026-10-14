//! Product orchestration: the frame loop, input normalisation, bounded workers, lifecycle
//! draining and startup. Platform hosts plug in through traits; no JNI here.
#![forbid(unsafe_code)]

mod bench;
mod bench_raster;
mod bench_scenes;
mod brush_pictures;
mod budget;
mod canvas;
mod clock;
mod display;
mod error;
mod fill;
mod frame;
mod host;
mod input;
mod normalise;
mod offscreen;
mod pacing;
mod pictures;
mod pointer;
mod preview;
mod recover;
mod screen;
mod seeds;
mod shared;
mod storage;
mod strip;
mod text;
mod touches;
mod transfer;
mod ui_pass;
mod worker;

pub use crate::bench::bench;
pub use crate::budget::ARENA_SHARE;
pub use crate::error::RuntimeError;
pub use crate::frame::Runtime;
pub use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
pub use crate::pacing::Paced;
pub use crate::text::{EditKey, TextInput};
pub use openpaint_app::{
    HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, PortError, ScreenProjection,
};
pub use openpaint_render::{Background, Counts, Spent, SurfaceWindow, take_spent};
pub use openpaint_store::DisplayCache;
pub use openpaint_ui::design::{FontFile, TextFonts};
pub use openpaint_ui::{CanvasArea, Control, ControlMap, Unresolved};
