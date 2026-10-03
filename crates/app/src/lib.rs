//! Application flow: intents, sessions, history, workspace, editor state and the capability
//! ports (traits) that store and platform code implement.
//!
//! Depends on `openpaint-core` only. No GPU, no UI, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod brushes;
mod error;
mod export;
mod gallery;
mod history;
mod input;
mod intent;
mod layers;
mod palette;
mod palm;
mod project;
mod projection;
mod recipe;
mod reference;
mod session;
mod storage;
mod stroke;
mod title;
mod toolbox;
mod transfer;

pub use crate::brushes::{Brush, BrushKey, BrushLibrary, SizeRange};
pub use crate::error::AppError;
pub use crate::export::{ExportIntent, ExportJob, ExportNotice, file_name};
pub use crate::gallery::{
    Gallery, GalleryIntent, GalleryNotice, GalleryProjection, LibraryAnswer, LibraryRequest,
};
pub use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
pub use crate::input::{Contact, ContactPhase, Device, InputBatch, InputSample};
pub use crate::intent::Intent;
pub use crate::layers::{LayerIntent, LayerList, LayerRow};
pub use crate::palette::{Hsv, RECENT_COLORS};
pub use crate::palm::PalmRejection;
pub use crate::project::{PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail};
pub use crate::projection::{Availability, EditorAction, ScreenProjection, SessionProjection};
pub use crate::recipe::RecipeError;
pub use crate::reference::{
    MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS, MAX_REFERENCE_SIDE, ReferenceImage, ReferenceIntent,
    ReferenceNotice,
};
pub use crate::session::Session;
pub use crate::storage::{StorageIntent, StorageNotice};
pub use crate::stroke::StrokeIntent;
pub use crate::title::{MAX_TITLE_CHARS, Title, TitleError, UNTITLED};
pub use crate::toolbox::{
    BrushChoice, BrushIntent, BrushProjection, BrushRow, BrushSetting, EditorState, MIN_OPACITY,
    Tool, ToolIntent, Toolbox,
};
pub use crate::transfer::{
    Delivery, ExportError, HostAnswer, HostRequest, ImageError, ImageFormat, ImageSink,
    ImageSource, OpenJob, TransferJob, TransferProjection, Transfers,
};
