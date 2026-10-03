//! Product orchestration: the frame loop, input normalisation, bounded workers, lifecycle
//! draining and startup. Platform hosts plug in through traits; no JNI here.
#![forbid(unsafe_code)]

mod canvas;
mod error;
mod frame;
mod input;
mod normalise;
mod pointer;
mod preview;
mod screen;
mod storage;
mod text;
mod transfer;
mod ui_pass;
mod worker;

pub use crate::error::RuntimeError;
pub use crate::frame::Runtime;
pub use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
pub use crate::text::{EditKey, TextInput};
pub use openpaint_app::{HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, PortError};
pub use openpaint_render::SurfaceWindow;
