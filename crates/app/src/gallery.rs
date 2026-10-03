//! The gallery (Procreate's model): every artwork as a thumbnail with its title, the one changed
//! last first; "+" makes a new one, a tap opens one, and each can be renamed, duplicated or
//! deleted. This module decides what to ask of the library and what the user is told; the
//! store answers on the runtime's worker. Deleting cannot be undone: the UI asks first.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::error::AppError;
use crate::project::{PortError, ProjectId, ProjectSummary};
use crate::projection::{Availability, EditorAction};
use crate::title::Title;

/// What the user can ask of the gallery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GalleryIntent {
    /// From the canvas: store the artwork, drop its history and show the gallery.
    Show,
    /// A new blank artwork, opened at once.
    Create,
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
}

/// A call the gallery needs from the library, in the order it must run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    List,
    Create(Title),
    Open(ProjectId),
    Rename(ProjectId, Title),
    Duplicate(ProjectId),
    Delete(ProjectId),
}

/// How a library call went, for the gallery. A successful open or create leaves the gallery.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAnswer {
    Listed(Result<Vec<ProjectSummary>, PortError>),
    /// A rename, duplicate or delete.
    Changed(Result<(), PortError>),
    OpenFailed(PortError),
    CreateFailed(PortError),
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
    CreateFailed,
    /// A rename, duplicate or delete failed; the gallery shows the library as it is.
    ChangeFailed,
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
        }
    }

    /// Applies one intent; `Show` is the canvas's and changes nothing here.
    ///
    /// # Errors
    /// `Unavailable` while an artwork is being opened or created.
    pub fn dispatch(&mut self, intent: GalleryIntent) -> Result<(), AppError> {
        if intent == GalleryIntent::Dismiss {
            self.notice = GalleryNotice::None;
            return Ok(());
        }
        let availability = self.availability();
        if !availability.is_available() {
            return Err(AppError::Unavailable {
                action: EditorAction::Gallery,
                availability,
            });
        }
        let request = match intent {
            GalleryIntent::Show | GalleryIntent::Dismiss => return Ok(()),
            GalleryIntent::Create => LibraryRequest::Create(Title::untitled()),
            GalleryIntent::Open(id) => LibraryRequest::Open(id),
            GalleryIntent::Rename { id, title } => LibraryRequest::Rename(id, title),
            GalleryIntent::Duplicate(id) => LibraryRequest::Duplicate(id),
            GalleryIntent::Delete(id) => LibraryRequest::Delete(id),
        };
        self.opening = matches!(request, LibraryRequest::Create(_) | LibraryRequest::Open(_));
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
            LibraryAnswer::OpenFailed(_) => {
                self.opening = false;
                self.notice = GalleryNotice::OpenFailed;
            }
            LibraryAnswer::CreateFailed(_) => {
                self.opening = false;
                self.notice = GalleryNotice::CreateFailed;
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
