//! The gallery (Procreate's model): every artwork as a thumbnail with its title, the one changed
//! last first; "+" makes a new one, a tap opens one, and each can be renamed, duplicated or
//! deleted, saved as an OpenPaint file or brought back from a kept version as a copy; "+" also
//! opens an OpenPaint file as a new artwork (`gallery_files.rs`); the Today card's options import
//! practice packs and add pictures of the user's own. This module decides what to ask of the
//! library and what the user is told; the store answers on the runtime's worker. Deleting cannot
//! be undone: the UI asks first. What a frame shows is `gallery_projection.rs`.

use std::collections::VecDeque;
use std::sync::Arc;

use openpaint_practice::{ItemRef, Pack, Tier};

use crate::error::AppError;
use crate::gallery_files::{FileJob, Files, problem};
use crate::gallery_projection::offered_unless;
pub use crate::gallery_projection::{FileProblem, GalleryNotice, GalleryProjection};
use crate::project::{PortError, ProjectId, ProjectSummary, VersionId};
use crate::projection::{Availability, EditorAction};
use crate::title::Title;
use crate::transfer::{HostAnswer, HostRequest};

/// What the user can ask of the gallery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GalleryIntent {
    /// From the canvas: store the artwork, drop its history and show the gallery.
    Show,
    /// A new blank artwork of the chosen size, opened at once.
    Create(CanvasPreset),
    Open(ProjectId),
    Rename {
        id: ProjectId,
        title: Title,
    },
    Duplicate(ProjectId),
    /// Asked only after the user confirmed it.
    Delete(ProjectId),
    /// Close the gallery's notice.
    Dismiss,
    /// "Save as OpenPaint file…": the artwork into a file the user chooses.
    SaveFile(ProjectId),
    /// "Open OpenPaint file…": a file the user chooses as a new artwork.
    OpenFile,
    /// A kept version of an artwork as a new artwork titled `title`.
    Restore {
        id: ProjectId,
        version: VersionId,
        title: Title,
    },
    /// A new practice session of the stage given, opened at once (the Today card and its
    /// options; an open one is continued by opening its sheet).
    Practice(u8),
    /// A new long colour copy of the picture chosen, opened at once (the practice options' New
    /// copy; one begun is opened by its sheet).
    Copy(ItemRef),
    /// A colour copy's name: its sheet's title.
    RenameCopy {
        sheet: ProjectId,
        title: Title,
    },
    /// A colour copy removed with its sheet, for good; asked only after the user confirmed it.
    RemoveCopy(ProjectId),
    /// "Import practice pack…": a pack file the user chooses, kept beside the library.
    ImportPack,
    /// "Add my picture…": a picture the user chooses, added to their own at the tier chosen first.
    AddPicture(Tier),
    /// One of the user's own pictures deleted, once the user confirmed it (`PictureShown`).
    RemovePicture(ItemRef),
    /// One of the user's own pictures moved to a tier.
    PictureTier(ItemRef, Tier),
}

/// The canvas sizes "+" offers (Procreate's new-canvas menu, reduced to four).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasPreset {
    /// The canvas area of the screen as it is held now.
    Screen,
    /// 2048 x 2048.
    Square,
    /// A4 at 300 dpi, 2480 x 3508.
    A4Portrait,
    /// 3508 x 2480.
    A4Landscape,
}

impl CanvasPreset {
    pub const ALL: [Self; 4] = [
        Self::Screen,
        Self::Square,
        Self::A4Portrait,
        Self::A4Landscape,
    ];

    /// The size in pixels on a canvas area of `screen` pixels. No side exceeds 4096, which
    /// every Vulkan device holds as a texture, nor `MAX_CANVAS_SIDE`.
    #[must_use]
    pub fn size(self, screen: [u32; 2]) -> [u32; 2] {
        match self {
            Self::Screen => screen.map(|side| side.clamp(1, 4096)),
            Self::Square => [2048, 2048],
            Self::A4Portrait => [2480, 3508],
            Self::A4Landscape => [3508, 2480],
        }
    }
}

/// A call the gallery needs from the library, in the order it must run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    List,
    Create(Title, CanvasPreset),
    Open(ProjectId),
    Rename(ProjectId, Title),
    Duplicate(ProjectId),
    Delete(ProjectId),
    Restore(ProjectId, VersionId, Title),
    /// A new practice session of the stage given and its sheet, which becomes the open one.
    Practice(u8),
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

/// The gallery's flow while it is shown.
#[derive(Debug)]
pub struct Gallery {
    items: Arc<[ProjectSummary]>,
    /// False until the first listing has arrived.
    listed: bool,
    requests: VecDeque<LibraryRequest>,
    /// An open or create is on its way; everything else waits for it.
    opening: bool,
    notice: GalleryNotice,
    files: Files,
}

impl Gallery {
    /// A gallery that first asks the library for its artworks.
    #[must_use]
    pub fn new() -> Self {
        let mut gallery = Self::listed(Vec::new());
        gallery.listed = false;
        gallery.requests.push_back(LibraryRequest::List);
        gallery
    }

    /// A gallery showing `items`, already listed.
    #[must_use]
    pub fn listed(items: Vec<ProjectSummary>) -> Self {
        Self {
            items: items.into(),
            listed: true,
            requests: VecDeque::new(),
            opening: false,
            notice: GalleryNotice::None,
            files: Files::default(),
        }
    }

    /// Applies one intent; `Show` is the canvas's and changes nothing here.
    ///
    /// # Errors
    /// `Unavailable` while an artwork is being opened or created, and for a file while a
    /// system dialog is open or a file is read or written.
    pub fn dispatch(&mut self, intent: GalleryIntent) -> Result<(), AppError> {
        if intent == GalleryIntent::Dismiss {
            self.notice = GalleryNotice::None;
            return Ok(());
        }
        let file = matches!(
            intent,
            GalleryIntent::SaveFile(_)
                | GalleryIntent::OpenFile
                | GalleryIntent::ImportPack
                | GalleryIntent::AddPicture(_)
        );
        let availability = if file && self.files.busy() {
            Availability::Running
        } else {
            self.availability()
        };
        availability.require(EditorAction::Gallery)?;
        if !self.file_dialog(&intent)
            && let Some(request) = request_of(intent)
        {
            self.ask(request);
        }
        Ok(())
    }

    /// A system dialog `intent` asks for (a file to save into, a file, a pack or a picture to
    /// read); whether it was one.
    fn file_dialog(&mut self, intent: &GalleryIntent) -> bool {
        match intent {
            GalleryIntent::SaveFile(id) => {
                let item = self.items.iter().find(|item| item.id == *id);
                let title = item.map_or_else(Title::untitled, |item| item.title.clone());
                self.files.save(*id, &title);
            }
            GalleryIntent::OpenFile => self.files.open(),
            GalleryIntent::ImportPack => self.files.import_pack(),
            GalleryIntent::AddPicture(tier) => self.files.add_picture(*tier),
            _ => return false,
        }
        true
    }

    /// `request` asked of the library, in order; an open or create waits for nothing more, any
    /// other change of the artworks is followed by their list again.
    fn ask(&mut self, request: LibraryRequest) {
        self.opening = matches!(
            request,
            LibraryRequest::Create(..)
                | LibraryRequest::Open(_)
                | LibraryRequest::Practice(_)
                | LibraryRequest::Copy(_)
        );
        // Practice's copies and pictures are no artworks: the list stays as it is.
        let artworks = !matches!(
            request,
            LibraryRequest::RenameCopy(..)
                | LibraryRequest::RemoveCopy(_)
                | LibraryRequest::RemovePicture(_)
                | LibraryRequest::PictureTier(..)
        );
        self.requests.push_back(request);
        if !self.opening && artworks {
            self.requests.push_back(LibraryRequest::List);
        }
    }

    /// The next call for the library, oldest first.
    pub fn request(&mut self) -> Option<LibraryRequest> {
        self.requests.pop_front()
    }

    /// The library answered a call this gallery asked for.
    pub fn answer(&mut self, answer: LibraryAnswer) {
        match answer {
            LibraryAnswer::Listed(Ok(items)) => {
                self.items = items.into();
                self.listed = true;
            }
            LibraryAnswer::Listed(Err(error)) => {
                self.listed = true;
                self.notice = match error {
                    PortError::OtherFormat { .. } => GalleryNotice::OtherLibrary,
                    _ => GalleryNotice::ListFailed,
                };
            }
            LibraryAnswer::Changed(Ok(())) => {}
            LibraryAnswer::Changed(Err(_)) => self.notice = GalleryNotice::ChangeFailed,
            LibraryAnswer::OpenFailed(_) => {
                self.opening = false;
                self.notice = GalleryNotice::OpenFailed;
            }
            LibraryAnswer::CreateFailed(_) => {
                self.opening = false;
                self.notice = GalleryNotice::CreateFailed;
            }
            LibraryAnswer::Restored(result) => {
                if result.is_err() {
                    self.notice = GalleryNotice::RestoreFailed;
                }
            }
            LibraryAnswer::FileSaved(result) => {
                self.files.finished();
                self.notice = match result {
                    Ok(()) => GalleryNotice::FileSaved,
                    Err(_) => GalleryNotice::FileSaveFailed,
                };
            }
            LibraryAnswer::FileOpened(result) => {
                self.files.finished();
                match result {
                    Ok(()) => self.requests.push_back(LibraryRequest::List),
                    Err(error) => self.notice = GalleryNotice::FileRefused(problem(&error)),
                }
            }
            LibraryAnswer::PackImported(result) => {
                self.files.finished();
                self.notice = match result {
                    Ok(_) => GalleryNotice::PackImported,
                    Err(error) => GalleryNotice::PackRefused(problem(&error)),
                };
            }
            LibraryAnswer::PictureAdded(result) => {
                self.files.finished();
                self.notice = match result {
                    Ok(pack) => GalleryNotice::PictureAdded {
                        outline: pack.items.last().is_some_and(|i| i.outline.is_some()),
                    },
                    Err(_) => GalleryNotice::PictureRefused,
                };
            }
        }
    }

    /// The next thing to ask of the platform (a save dialog or a file picker), once.
    pub fn host_request(&mut self) -> Option<HostRequest> {
        self.files.request()
    }

    /// True while a file dialog asked for waits for the platform's answer.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.files.waiting()
    }

    /// The platform's answer to the file dialog that is open: the job for the library's
    /// worker, if any.
    pub fn host_answer(&mut self, answer: HostAnswer) -> Option<FileJob> {
        match self.files.answer(answer) {
            Ok(job) => job,
            Err(notice) => {
                self.notice = notice;
                None
            }
        }
    }

    /// Every gallery action is offered unless an artwork is being opened.
    #[must_use]
    pub const fn availability(&self) -> Availability {
        offered_unless(self.opening)
    }

    #[must_use]
    pub fn projection(&self) -> GalleryProjection {
        GalleryProjection {
            items: Arc::clone(&self.items),
            listed: self.listed,
            opening: self.opening,
            notice: self.notice,
            today: None,
        }
    }
}

impl Default for Gallery {
    fn default() -> Self {
        Self::new()
    }
}

/// The library's call `intent` asks for; none for the canvas's `Show`, a notice dismissed or a
/// system dialog (`Gallery::file_dialog`).
fn request_of(intent: GalleryIntent) -> Option<LibraryRequest> {
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
        GalleryIntent::Copy(picture) => LibraryRequest::Copy(picture),
        GalleryIntent::RenameCopy { sheet, title } => LibraryRequest::RenameCopy(sheet, title),
        GalleryIntent::RemoveCopy(sheet) => LibraryRequest::RemoveCopy(sheet),
        GalleryIntent::RemovePicture(picture) => LibraryRequest::RemovePicture(picture.item),
        GalleryIntent::PictureTier(picture, tier) => {
            LibraryRequest::PictureTier(picture.item, tier)
        }
    })
}
