//! Product orchestration: the frame loop, input normalisation, bounded workers, lifecycle
//! draining and startup. Platform hosts plug in through traits; no JNI here.
//!
//! The frame (`frame/`, its steps in its `mod.rs`'s `//!`; `start.rs`, `receive.rs` the workers'
//! answers, the canvas opening, the intents applied; `ui_pass.rs`); the workers, each a
//! `worker::Queue` on its own named thread (ARCHITECTURE section 3): `storage/`, `transfer.rs`,
//! `fill.rs`, `pictures.rs`, `brush_pictures.rs` with `strip.rs`, `display.rs`, `raster.rs`,
//! the replay's videos `video.rs` (S163), practice's in `practice/`. The rest: the open canvas
//! `canvas/` (`Work::Artwork | Work::Sheet`, D119; the frame's intents, the eyedropper, the
//! target and view tiers, the display cache, the replay shown instead of the artwork),
//! `screen/` with its jobs (`Screen { Opening, Canvas, Gallery }`), practice composed
//! `practice/` (D112), background work while idle `ahead.rs` and `touches.rs` (D96), `pacing.rs`
//! (D97), device loss `recover.rs` and `shared.rs`, the GPU budget `budget/` with what it logs
//! (D85, D93), `clock.rs`, input `normalise.rs`, `input.rs`, `pointer.rs`,
//! `text.rs`, the host's requests and the version line `host.rs`, `seeds.rs`, the harness's frames
//! and test hooks `offscreen.rs` (`fix_clock`), `painted.rs`, the debug benchmarks `bench/` (D85)
//! and the recorder of frames in real use `recorder/`.
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
mod recorder;
mod recover;
mod resident;
mod screen;
mod seeds;
mod shared;
mod storage;
mod strip;
mod text;
mod touches;
mod transfer;
mod ui_pass;
mod video;
mod worker;

pub use crate::bench::{SCENARIOS, STROKES_ADDED, busiest};
pub use crate::budget::{ARENA_SHARE, TRIM_UI_HIDDEN};
pub use crate::error::RuntimeError;
pub use crate::frame::{Runtime, StartOptions};
pub use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
pub use crate::pacing::{Paced, Wanted};
pub use crate::painted::{drawn, pile_path, style_of};
pub use crate::practice::{PracticeTest, skill_start};
pub use crate::recorder::{FRAMES_FILE, FRAMES_KEPT, FrameRecorder};
pub use crate::text::{EditKey, TextInput};
pub use crate::transfer::{Encoded, encoded};
pub use crate::video::{VideoShared, make_video};
pub use openpaint_app::{
    BytesSource, HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, Language, PortError,
    ScreenProjection,
};
pub use openpaint_practice_app::Start;
pub use openpaint_render::{
    Attached, Background, Counts, Spent, StrokeDrew, StrokeParts, SurfaceWindow, take_spent,
};
pub use openpaint_store::DisplayCache;
pub use openpaint_ui::design::{FontFile, TextFonts};
pub use openpaint_ui::{CanvasArea, Control, ControlMap, Unresolved};
