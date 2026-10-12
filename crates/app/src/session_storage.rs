//! The session's side of storage (`storage.rs`): which version to hand to the store now and as
//! the user leaves, how saving went, Retry and the notice; and ending the stroke being drawn
//! when the pen can no longer lift it (S50).

use openpaint_core::Artwork;

use super::Session;
use crate::error::AppError;
use crate::project::PortError;
use crate::projection::EditorAction;
use crate::storage::StorageIntent;

impl Session {
    /// The version the store should write now, if any: the current one when it is not stored
    /// yet, no save is running and its save did not just fail. The session then counts that
    /// save as running until `save_finished`.
    pub fn save_request(&mut self) -> Option<Artwork> {
        self.autosave.request(self.history.current())
    }

    /// The version the store should write as the user leaves the app: as `save_request`, but a
    /// version whose save failed is tried once more, since the process may not come back. A
    /// transform in progress (an inserted picture too) and an adjustment under way are committed
    /// first, so neither is lost; the caller ends a stroke being drawn before (`end_stroke`).
    pub fn leave_request(&mut self) -> Option<Artwork> {
        self.commit_transform();
        self.apply_adjustment();
        if self.autosave.can_retry() {
            self.autosave.retry();
        }
        self.save_request()
    }

    /// Commits the stroke being drawn as its lift would (every sample so far, the line's end
    /// onto the pen, its mirrored copies; the renderer tapers its end as at a lift), when the pen
    /// can no longer lift it: the app is left or its window goes mid-stroke (S50). Returns
    /// whether there was one; the pen's later events for it change nothing.
    ///
    /// # Errors
    /// As a lift's: the stroke is still committed with the samples it had.
    pub fn end_stroke(&mut self) -> Result<bool, AppError> {
        self.strokes.end(&mut self.history)
    }

    /// The store finished writing `version` with `result`. A failure shows in the projection
    /// with a retry; the next change or a retry tries again.
    pub fn save_finished(&mut self, version: &Artwork, result: &Result<(), PortError>) {
        self.autosave.finished(version, result);
    }

    /// True while a save handed out by `save_request` has not finished.
    #[must_use]
    pub fn saving(&self) -> bool {
        self.autosave.saving()
    }

    pub(super) fn storage(&mut self, intent: StorageIntent) -> Result<(), AppError> {
        match intent {
            StorageIntent::RetrySave => {
                self.check(EditorAction::RetrySave)?;
                self.autosave.retry();
            }
            StorageIntent::Dismiss => {
                self.check(EditorAction::DismissNotice)?;
                self.autosave.dismiss();
            }
        }
        Ok(())
    }
}
