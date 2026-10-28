//! What the gallery (`gallery.rs`) asks of the library, in order, and how the library answers: the
//! artworks' calls, a new practice session or colour copy, a copy's name or removal, the user's
//! own pictures, and the practice sheets the practice book reads (S89).

use openpaint_practice::{Family, ItemRef, Pack, Tier};

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
    /// A new practice session of the stage given and its sheet, which becomes the open one.
    Practice(u8),
    /// A new drill of the kind given and its sheet, which becomes the open one (S91).
    Drill(Family),
    /// A new long colour copy of the picture given and its sheet, which becomes the open one.
    Copy(ItemRef),
    /// A colour copy's sheet titled anew.
    RenameCopy(ProjectId, Title),
    /// A colour copy removed, its record and its sheet.
    RemoveCopy(ProjectId),
    /// One of the user's own pictures (by its item id) removed from their pack.
    RemovePicture(String),
    /// One of the user's own pictures (by its item id) moved to a tier.
    PictureTier(String, Tier),
    /// The sheets of the practice records given (by id) read for the practice book.
    ReadSheets(Vec<u64>),
}

impl LibraryRequest {
    /// Whether it changes the artworks listed, which are listed again after it (practice's
    /// copies, pictures and sheets read are no artworks).
    pub(crate) const fn changes_artworks(&self) -> bool {
        !matches!(
            self,
            Self::RenameCopy(..)
                | Self::RemoveCopy(_)
                | Self::RemovePicture(_)
                | Self::PictureTier(..)
                | Self::ReadSheets(_)
        )
    }
}

/// How a library call went, for the gallery. A successful open or create leaves the gallery.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAnswer {
    Listed(Result<Vec<ProjectSummary>, PortError>),
    /// A rename, duplicate or delete.
    Changed(Result<(), PortError>),
    OpenFailed(PortError),
    CreateFailed(PortError),
    Restored(Result<(), PortError>),
    /// The artwork was written into the chosen file (`FileJob::Save`), or not.
    FileSaved(Result<(), PortError>),
    /// The chosen file was added as a new artwork (`FileJob::Open`), or refused.
    FileOpened(Result<(), PortError>),
    /// The chosen practice pack was kept (`FileJob::ImportPack`), or refused.
    PackImported(Result<Pack, PortError>),
    /// The chosen picture was added to the user's own (`FileJob::AddPicture`), the personal
    /// pack as it is now (the picture its last item), or refused.
    PictureAdded(Result<Pack, PortError>),
}

/// The library's call `intent` asks for; none for the canvas's `Show`, a notice dismissed or a
/// system dialog (`Gallery::file_dialog`).
pub(crate) fn request_of(intent: GalleryIntent) -> Option<LibraryRequest> {
    Some(match intent {
        GalleryIntent::Show
        | GalleryIntent::Dismiss
        | GalleryIntent::SaveFile(_)
        | GalleryIntent::OpenFile
        | GalleryIntent::ImportPack
        | GalleryIntent::AddPicture(_) => return None,
        GalleryIntent::Create(preset) => LibraryRequest::Create(Title::untitled(), preset),
        GalleryIntent::Open(id) => LibraryRequest::Open(id),
        GalleryIntent::Rename { id, title } => LibraryRequest::Rename(id, title),
        GalleryIntent::Duplicate(id) => LibraryRequest::Duplicate(id),
        GalleryIntent::Delete(id) => LibraryRequest::Delete(id),
        GalleryIntent::Restore { id, version, title } => {
            LibraryRequest::Restore(id, version, title)
        }
        GalleryIntent::Practice(stage) => LibraryRequest::Practice(stage),
        GalleryIntent::Drill(family) => LibraryRequest::Drill(family),
        GalleryIntent::Copy(picture) => LibraryRequest::Copy(picture),
        GalleryIntent::RenameCopy { sheet, title } => LibraryRequest::RenameCopy(sheet, title),
        GalleryIntent::RemoveCopy(sheet) => LibraryRequest::RemoveCopy(sheet),
        GalleryIntent::RemovePicture(picture) => LibraryRequest::RemovePicture(picture.item),
        GalleryIntent::PictureTier(picture, tier) => {
            LibraryRequest::PictureTier(picture.item, tier)
        }
        GalleryIntent::ReadSheets(records) => LibraryRequest::ReadSheets(records),
    })
}
