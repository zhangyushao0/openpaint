//! `Session`: the open artwork, its history and the editor flow around it. The UI emits
//! intents, the session applies them; the renderer reads the current version and the
//! in-progress stroke beside it; the UI reads the projection; the runtime asks it which version
//! to save and reports how saving went.

use openpaint_core::{ActiveStroke, Artwork, CanvasSize};

use crate::error::AppError;
use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
use crate::intent::Intent;
use crate::layers::{self, LayerList, Layers};
use crate::project::PortError;
use crate::projection::{Availability, EditorAction, LAYER_ACTIONS, SessionProjection};
use crate::storage::{Autosave, StorageIntent};
use crate::stroke::Strokes;

#[derive(Debug)]
pub struct Session {
    history: History,
    strokes: Strokes,
    layers: Layers,
    autosave: Autosave,
}

impl Session {
    /// A blank artwork of `canvas` pixels with an empty history.
    ///
    /// # Errors
    /// `Core` when the default brush is invalid (a programming error).
    pub fn new(canvas: CanvasSize) -> Result<Self, AppError> {
        Self::open(Artwork::new(canvas))
    }

    /// `artwork` as the store holds it, with an empty history (history is not stored).
    ///
    /// # Errors
    /// `Core` when the default brush is invalid (a programming error).
    pub fn open(artwork: Artwork) -> Result<Self, AppError> {
        Ok(Self {
            autosave: Autosave::new(artwork.clone()),
            history: History::new(artwork, HISTORY_BUDGET),
            strokes: Strokes::new()?,
            layers: Layers::default(),
        })
    }

    /// A blank artwork after the last one could not be opened; the projection tells the user
    /// until they close the notice.
    ///
    /// # Errors
    /// `Core` when the default brush is invalid (a programming error).
    pub fn after_failed_open(canvas: CanvasSize) -> Result<Self, AppError> {
        let mut session = Self::new(canvas)?;
        session.autosave = Autosave::after_failed_open(session.artwork().clone());
        Ok(session)
    }

    /// Applies one intent. On error the session is as it was before, except that a stroke
    /// whose lift failed to extend is still committed with the samples it had.
    ///
    /// # Errors
    /// `Core` when the artwork refuses the edit (a locked layer, an invalid sample);
    /// `Unavailable` when the projection does not offer the action.
    pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError> {
        if let Intent::Layer(layer) = &intent {
            let drawing = self.strokes.active().is_some();
            return self.layers.handle(&mut self.history, drawing, layer);
        }
        self.layers.interrupt();
        match intent {
            Intent::Stroke(stroke) => {
                let mut artwork = self.history.current().clone();
                let result = self.strokes.handle(&mut artwork, stroke);
                self.history.record(artwork);
                result
            }
            Intent::History(step) => self.step(step),
            Intent::Storage(storage) => self.storage(storage),
            Intent::Layer(_) => Ok(()),
        }
    }

    /// The current version. While a stroke is drawn it is the version the stroke started on.
    #[must_use]
    pub fn artwork(&self) -> &Artwork {
        self.history.current()
    }

    /// The stroke being drawn, which is not in the artwork until it is committed.
    #[must_use]
    pub fn active(&self) -> Option<&ActiveStroke> {
        self.strokes.active()
    }

    /// What the UI may offer now.
    #[must_use]
    pub fn projection(&self) -> SessionProjection {
        let availability = |possible: bool| {
            if self.strokes.active().is_some() {
                Availability::StrokeInProgress
            } else if possible {
                Availability::Available
            } else {
                Availability::NothingLeft
            }
        };
        let retry_save = if self.autosave.can_retry() {
            Availability::Available
        } else if self.autosave.retrying() {
            Availability::Running
        } else {
            Availability::NothingLeft
        };
        let dismiss_notice = if self.autosave.can_dismiss() {
            Availability::Available
        } else {
            Availability::NothingLeft
        };
        let artwork = self.history.current();
        let drawing = self.strokes.active().is_some();
        SessionProjection {
            undo: availability(self.history.can_undo()),
            redo: availability(self.history.can_redo()),
            retry_save,
            dismiss_notice,
            layer_actions: LAYER_ACTIONS
                .map(|action| layers::availability(artwork, drawing, action, artwork.active())),
            storage: self.autosave.notice(),
            layers: Some(LayerList::of(artwork)),
        }
    }

    /// The history, for inspection.
    #[must_use]
    pub fn history(&self) -> &History {
        &self.history
    }

    /// The version the store should write now, if any: the current one when it is not stored
    /// yet, no save is running and its save did not just fail. The session then counts that
    /// save as running until `save_finished`.
    pub fn save_request(&mut self) -> Option<Artwork> {
        self.autosave.request(self.history.current())
    }

    /// The version the store should write as the user leaves the app: as `save_request`, but a
    /// version whose save failed is tried once more, since the process may not come back.
    pub fn leave_request(&mut self) -> Option<Artwork> {
        if self.autosave.can_retry() {
            self.autosave.retry();
        }
        self.save_request()
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

    fn step(&mut self, step: HistoryIntent) -> Result<(), AppError> {
        let action = match step {
            HistoryIntent::Undo => EditorAction::Undo,
            HistoryIntent::Redo => EditorAction::Redo,
        };
        self.check(action)?;
        match step {
            HistoryIntent::Undo => self.history.undo(),
            HistoryIntent::Redo => self.history.redo(),
        };
        Ok(())
    }

    fn storage(&mut self, intent: StorageIntent) -> Result<(), AppError> {
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

    /// `Unavailable` unless the projection offers `action` now.
    fn check(&self, action: EditorAction) -> Result<(), AppError> {
        let availability = self.projection().availability(action);
        if availability.is_available() {
            Ok(())
        } else {
            Err(AppError::Unavailable {
                action,
                availability,
            })
        }
    }
}
