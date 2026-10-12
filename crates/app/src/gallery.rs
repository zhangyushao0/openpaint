//! The gallery (Procreate's model): every artwork as a thumbnail with its title, the one changed
//! last first; "+" makes a new one, a tap opens one, and each can be renamed, duplicated or
//! deleted, saved as an OpenPaint file or brought back from a kept version as a copy; "+" also
//! opens an OpenPaint file as a new artwork (`gallery_files.rs`). This module decides what to
//! ask of the library and what the user is told; the store answers on the runtime's worker.
//! Deleting cannot be undone: the UI asks first.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::error::AppError;
use crate::gallery_files::{FileJob, Files, problem};
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
}

/// Why a chosen file did not open as an artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileProblem {
    /// Not an OpenPaint file.
    NotOpenPaint,
    /// An OpenPaint file that breaks its format or check sum.
    Damaged,
    /// Written by a newer OpenPaint than this one.
    Newer,
    /// The file could not be read.
    Unreadable,
}

/// What the gallery tells the user, with an OK to close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryNotice {
    #[default]
    None,
    /// The library could not be listed.
    ListFailed,
    /// The artwork could not be opened; it stays in the library as it was.
    OpenFailed,
    /// The artwork's records could not be upgraded; they stay as they were (S50).
    NeedsRepair,
    CreateFailed,
    /// A rename, duplicate or delete failed; the gallery shows the library as it is.
    ChangeFailed,
    /// The artwork was saved as an OpenPaint file.
    FileSaved,
    FileSaveFailed,
    /// The chosen file was not added; nothing changed.
    FileRefused(FileProblem),
    /// The version could not be restored; nothing changed.
    RestoreFailed,
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

/// A snapshot of the gallery for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryProjection {
    items: Arc<[ProjectSummary]>,
    listed: bool,
    opening: bool,
    notice: GalleryNotice,
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
        let file = matches!(intent, GalleryIntent::SaveFile(_) | GalleryIntent::OpenFile);
        let availability = if file && self.files.busy() {
            Availability::Running
        } else {
            self.availability()
        };
        if !availability.is_available() {
            return Err(AppError::Unavailable {
                action: EditorAction::Gallery,
                availability,
            });
        }
        let request = match intent {
            GalleryIntent::Show | GalleryIntent::Dismiss => return Ok(()),
            GalleryIntent::SaveFile(id) => {
                let item = self.items.iter().find(|item| item.id == id);
                let title = item.map_or_else(Title::untitled, |item| item.title.clone());
                self.files.save(id, &title);
                return Ok(());
            }
            GalleryIntent::OpenFile => {
                self.files.open();
                return Ok(());
            }
            GalleryIntent::Create(preset) => LibraryRequest::Create(Title::untitled(), preset),
            GalleryIntent::Open(id) => LibraryRequest::Open(id),
            GalleryIntent::Rename { id, title } => LibraryRequest::Rename(id, title),
            GalleryIntent::Duplicate(id) => LibraryRequest::Duplicate(id),
            GalleryIntent::Delete(id) => LibraryRequest::Delete(id),
            GalleryIntent::Restore { id, version, title } => {
                LibraryRequest::Restore(id, version, title)
            }
        };
        self.opening = matches!(
            request,
            LibraryRequest::Create(..) | LibraryRequest::Open(_)
        );
        self.requests.push_back(request);
        if !self.opening {
            self.requests.push_back(LibraryRequest::List);
        }
        Ok(())
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
            LibraryAnswer::Listed(Err(_)) => {
                self.listed = true;
                self.notice = GalleryNotice::ListFailed;
            }
            LibraryAnswer::Changed(Ok(())) => {}
            LibraryAnswer::Changed(Err(_)) => self.notice = GalleryNotice::ChangeFailed,
            LibraryAnswer::OpenFailed(error) => {
                self.opening = false;
                self.notice = match error {
                    PortError::NotUpgraded { .. } => GalleryNotice::NeedsRepair,
                    _ => GalleryNotice::OpenFailed,
                };
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
    pub fn availability(&self) -> Availability {
        if self.opening {
            Availability::Running
        } else {
            Availability::Available
        }
    }

    #[must_use]
    pub fn projection(&self) -> GalleryProjection {
        GalleryProjection {
            items: Arc::clone(&self.items),
            listed: self.listed,
            opening: self.opening,
            notice: self.notice,
        }
    }
}

impl Default for Gallery {
    fn default() -> Self {
        Self::new()
    }
}

impl GalleryProjection {
    /// The artworks, the one changed last first.
    #[must_use]
    pub fn items(&self) -> &[ProjectSummary] {
        &self.items
    }

    /// False until the library has been listed; an empty list before that is not "no artworks".
    #[must_use]
    pub const fn listed(&self) -> bool {
        self.listed
    }

    /// Whether the artworks can be opened, created, renamed, duplicated and deleted now.
    #[must_use]
    pub const fn availability(&self) -> Availability {
        if self.opening {
            Availability::Running
        } else {
            Availability::Available
        }
    }

    #[must_use]
    pub const fn notice(&self) -> GalleryNotice {
        self.notice
    }
}
