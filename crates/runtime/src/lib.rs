//! Product orchestration: the frame loop, input normalisation, bounded workers, lifecycle
//! draining and startup. Platform hosts plug in through traits; no JNI here.
#![forbid(unsafe_code)]

mod canvas;
mod error;
mod frame;
mod input;
mod normalise;
mod pointer;

pub use crate::error::RuntimeError;
pub use crate::frame::Runtime;
pub use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
pub use openpaint_render::SurfaceWindow;
