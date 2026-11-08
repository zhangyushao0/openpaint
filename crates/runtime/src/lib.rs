//! Product orchestration: the frame loop, input normalisation, bounded workers, lifecycle
//! draining and startup. Platform hosts plug in through traits; no JNI here.
#![forbid(unsafe_code)]

mod ahead;
mod bench;
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
mod painted;
mod pictures;
mod pointer;
mod practice;
mod raster;
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

pub use crate::bench::{SCENARIOS, STROKES_ADDED, busiest};
pub use crate::budget::{ARENA_SHARE, TRIM_UI_HIDDEN};
pub use crate::error::RuntimeError;
pub use crate::frame::{Runtime, StartOptions};
pub use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
pub use crate::pacing::Paced;
pub use crate::painted::{drawn, pile_path, style_of};
pub use crate::practice::PracticeTest;
pub use crate::text::{EditKey, TextInput};
pub use crate::transfer::{Encoded, encoded};
pub use openpaint_app::{
    BytesSource, HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, Language, PortError,
    ScreenProjection,
};
pub use openpaint_render::{Background, Counts, Spent, SurfaceWindow, take_spent};
pub use openpaint_store::DisplayCache;
pub use openpaint_ui::design::{FontFile, TextFonts};
pub use openpaint_ui::{CanvasArea, Control, ControlMap, Unresolved};
