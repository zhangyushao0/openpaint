//! Autosave: which version must be stored next, what the user is told when storing or opening
//! failed, and the intents that answer it. The writing itself is the store's, on a worker the
//! runtime owns; this module only decides.

use openpaint_core::Artwork;

use crate::project::PortError;

/// What the user can ask about storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageIntent {
    /// Try again at once to save the version whose save failed.
    RetrySave,
    /// Close the notice that the last artwork could not be opened.
    Dismiss,
}

/// What the screen tells the user about storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageNotice {
    /// Nothing to tell: every version is saved or about to be.
    #[default]
    None,
    /// The last save failed and nothing newer is stored; the drawing is still on screen.
    /// `retrying` while another attempt runs.
    SaveFailed { retrying: bool },
    /// The last artwork could not be opened; a new blank one was started and the old one is
    /// kept in the library as it was.
    OpenFailed,
}

/// The autosave flow of one session.
#[derive(Debug)]
pub(crate) struct Autosave {
    /// The version the store holds, or the one the session started with.
    saved: Artwork,
    /// The version being written now.
    running: Option<Artwork>,
    /// A version whose save failed; it is not tried again by itself, only after a change or a
    /// retry, so a store that keeps failing is not hammered every frame.
    failed: Option<Artwork>,
    /// From a failed save until a save succeeds.
    failing: bool,
    open_failed: bool,
}

impl Autosave {
    /// A flow whose store holds `saved` (a blank artwork counts as stored: nothing to lose).
    pub(crate) fn new(saved: Artwork) -> Self {
        Self {
            saved,
            running: None,
            failed: None,
            failing: false,
            open_failed: false,
        }
    }

    pub(crate) fn after_failed_open(saved: Artwork) -> Self {
        Self {
            open_failed: true,
            ..Self::new(saved)
        }
    }

    /// The version to hand to the store now, if any: `current` when it is not stored, no save
    /// is running and it did not just fail. The flow then counts it as running.
    pub(crate) fn request(&mut self, current: &Artwork) -> Option<Artwork> {
        let failed_already = self
            .failed
            .as_ref()
            .is_some_and(|failed| failed.same_version(current));
        if self.running.is_some() || current.same_version(&self.saved) || failed_already {
            return None;
        }
        self.running = Some(current.clone());
        Some(current.clone())
    }

    /// The store finished writing `version`. Returns false, changing nothing, when `version`
    /// is not the one running.
    pub(crate) fn finished(&mut self, version: &Artwork, result: &Result<(), PortError>) -> bool {
        if !self
            .running
            .as_ref()
            .is_some_and(|running| running.same_version(version))
        {
            return false;
        }
        self.running = None;
        match result {
            Ok(()) => {
                self.saved = version.clone();
                self.failed = None;
                self.failing = false;
            }
            Err(_) => {
                self.failed = Some(version.clone());
                self.failing = true;
            }
        }
        true
    }

    /// True while the store is writing a version.
    pub(crate) fn saving(&self) -> bool {
        self.running.is_some()
    }

    pub(crate) fn can_retry(&self) -> bool {
        self.failing && self.running.is_none()
    }

    /// A save runs while the last one's failure is still shown.
    pub(crate) fn retrying(&self) -> bool {
        self.failing && self.running.is_some()
    }

    /// Lets the failed version be requested again. Call only when `can_retry`.
    pub(crate) fn retry(&mut self) {
        self.failed = None;
    }

    pub(crate) fn can_dismiss(&self) -> bool {
        self.open_failed
    }

    pub(crate) fn dismiss(&mut self) {
        self.open_failed = false;
    }

    pub(crate) fn notice(&self) -> StorageNotice {
        if self.failing {
            StorageNotice::SaveFailed {
                retrying: self.retrying(),
            }
        } else if self.open_failed {
            StorageNotice::OpenFailed
        } else {
            StorageNotice::None
        }
    }
}
