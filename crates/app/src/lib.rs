//! Application flow: intents, sessions, history, workspace, editor state and the capability
//! ports (traits) that store and platform code implement.
//!
//! Depends on `openpaint-core` only. No GPU, no UI, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod error;
mod history;
mod input;
mod intent;
mod projection;
mod session;
mod stroke;

pub use crate::error::AppError;
pub use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
pub use crate::input::{Contact, ContactPhase, Device, InputBatch, InputSample};
pub use crate::intent::Intent;
pub use crate::projection::{Availability, EditorAction, SessionProjection};
pub use crate::session::Session;
pub use crate::stroke::StrokeIntent;
