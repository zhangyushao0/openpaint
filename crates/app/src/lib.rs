//! Application flow: intents, sessions, history, workspace, editor state and the capability
//! ports (traits) that store and platform code implement.
//!
//! Depends on `openpaint-core` only. No GPU, no UI, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod adjust;
mod brushes;
mod editor_state;
mod error;
mod export;
mod fill;
mod gallery;
mod gallery_files;
mod gestures;
mod history;
mod input;
mod insert;
mod intent;
mod layer_rules;
mod layers;
mod palette;
mod palm;
mod project;
mod projection;
mod recipe;
mod reference;
mod selection;
mod session;
mod storage;
mod streamline;
mod stroke;
mod title;
mod toolbox;
mod transfer;
mod transform;
mod view;

pub use crate::adjust::{AdjustIntent, AdjustProjection, AdjustRefusal};
pub use crate::brushes::{Brush, BrushKey, BrushLibrary, SizeRange};
pub use crate::editor_state::{BrushChoice, BrushSetting, EditorState};
pub use crate::error::AppError;
pub use crate::export::{
    ARTWORK_EXTENSION, ExportIntent, ExportJob, ExportNotice, ExportScale, artwork_file_name,
    file_name,
};
pub use crate::fill::{
    DEFAULT_THRESHOLD, FillFailed, FillIntent, FillJob, FillProjection, FillRefusal,
};
pub use crate::gallery::{
    CanvasPreset, FileProblem, Gallery, GalleryIntent, GalleryNotice, GalleryProjection,
    LibraryAnswer, LibraryRequest,
};
pub use crate::gallery_files::{FileJob, MAX_ARTWORK_FILE_BYTES};
pub use crate::gestures::{
    GestureFrame, Gestured, Gestures, PictureGestured, PictureGestures, Touch, TouchAction,
};
pub use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
pub use crate::input::{Contact, ContactPhase, Device, InputBatch, InputSample};
pub use crate::insert::{InsertIntent, InsertJob};
pub use crate::intent::Intent;
pub use crate::layers::{GroupRow, LayerIntent, LayerList, LayerRow, MaskRow};
pub use crate::palette::{Hsv, RECENT_COLORS};
pub use crate::palm::PalmRejection;
pub use crate::project::{
    KEPT_VERSIONS, KeptVersion, PortError, Project, ProjectId, ProjectStore, ProjectSummary,
    Thumbnail, VersionId,
};
pub use crate::projection::{Availability, EditorAction, ScreenProjection, SessionProjection};
pub use crate::recipe::RecipeError;
pub use crate::reference::{
    MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS, MAX_REFERENCE_SIDE, ReferenceImage, ReferenceIntent,
    ReferenceNotice,
};
pub use crate::selection::{SelectionIntent, SelectionProjection, SelectionShape};
pub use crate::session::Session;
pub use crate::storage::{StorageIntent, StorageNotice};
pub use crate::streamline::StreamLine;
pub use crate::stroke::StrokeIntent;
pub use crate::title::{MAX_TITLE_CHARS, Title, TitleError, UNTITLED};
pub use crate::toolbox::{
    BrushIntent, BrushProjection, BrushRow, MIN_OPACITY, SMUDGE_STRENGTH, Tool, ToolIntent, Toolbox,
};
pub use crate::transfer::{
    Delivery, ExportError, HostAnswer, HostRequest, ImageError, ImageFormat, ImageSink,
    ImageSource, OpenJob, TransferJob, TransferProjection, Transfers,
};
pub use crate::transform::{
    LiftFailed, TransformFrame, TransformIntent, TransformJob, TransformProjection,
    TransformRefusal, picture_place,
};
pub use crate::view::{
    CanvasView, FIT_MARGIN, MAX_ZOOM, MIN_ZOOM, ViewGesture, ViewIntent, ViewMapping, Viewport,
};
