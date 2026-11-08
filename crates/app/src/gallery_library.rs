//! What the gallery (`gallery.rs`) asks of the library, in order, and how the library answers.

use crate::gallery::GalleryIntent;
use crate::project::{PortError, ProjectId, ProjectSummary, VersionId};
use crate::title::Title;

/// A call the gallery needs from the library, in the order it must run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    List,
    Create(Title, crate::gallery::CanvasPreset),
    Open(ProjectId),
    Rename(ProjectId, Title),
    Duplicate(ProjectId),
    Delete(ProjectId),
    Restore(ProjectId, VersionId, Title),
}

/// How a library call went, for the gallery. A successful open or create leaves the gallery.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAnswer {
    Listed(Result<Vec<ProjectSummary>, PortError>),
    /// A rename, duplicate or delete (or a mode's change).
    Changed(Result<(), PortError>),
    OpenFailed(PortError),
    /// A new artwork (or a mode's) was not made.
    CreateFailed(PortError),
    Restored(Result<(), PortError>),
    /// The artwork was written into the chosen file (`FileJob::Save`), or not.
    FileSaved(Result<(), PortError>),
    /// The chosen file was added as a new artwork (`FileJob::Open`), or refused.
    FileOpened(Result<(), PortError>),
}

/// The library's call `intent` asks for; none for the canvas's `Show`, a notice dismissed or a
/// system dialog (`Gallery::file_dialog`).
pub(crate) fn request_of(intent: GalleryIntent) -> Option<LibraryRequest> {
    Some(match intent {
        GalleryIntent::Show
        | GalleryIntent::Dismiss
        | GalleryIntent::SaveFile(_)
        | GalleryIntent::OpenFile => return None,
        GalleryIntent::Create(preset) => LibraryRequest::Create(Title::untitled(), preset),
        GalleryIntent::Open(id) => LibraryRequest::Open(id),
        GalleryIntent::Rename { id, title } => LibraryRequest::Rename(id, title),
        GalleryIntent::Duplicate(id) => LibraryRequest::Duplicate(id),
        GalleryIntent::Delete(id) => LibraryRequest::Delete(id),
        GalleryIntent::Restore { id, version, title } => {
            LibraryRequest::Restore(id, version, title)
        }
    })
}
