//! `Session`: the open artwork, its history and the editor flow around it. The UI emits
//! intents, the session applies them; the renderer reads the current version and the
//! in-progress stroke beside it; the UI reads the projection.

use openpaint_core::{ActiveStroke, Artwork, CanvasSize};

use crate::error::AppError;
use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
use crate::intent::Intent;
use crate::projection::{Availability, EditorAction, SessionProjection};
use crate::stroke::Strokes;

#[derive(Debug)]
pub struct Session {
    history: History,
    strokes: Strokes,
}

impl Session {
    /// A blank artwork of `canvas` pixels with an empty history.
    ///
    /// # Errors
    /// `Core` when the default brush is invalid (a programming error).
    pub fn new(canvas: CanvasSize) -> Result<Self, AppError> {
        Ok(Self {
            history: History::new(Artwork::new(canvas), HISTORY_BUDGET),
            strokes: Strokes::new()?,
        })
    }

    /// Applies one intent. On error the session is as it was before, except that a stroke
    /// whose lift failed to extend is still committed with the samples it had.
    ///
    /// # Errors
    /// `Core` when the artwork refuses the edit (a locked layer, an invalid sample);
    /// `Unavailable` when the projection does not offer the action.
    pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError> {
        match intent {
            Intent::Stroke(stroke) => {
                let mut artwork = self.history.current().clone();
                let result = self.strokes.handle(&mut artwork, stroke);
                self.history.record(artwork);
                result
            }
            Intent::History(step) => self.step(step),
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
        SessionProjection {
            undo: availability(self.history.can_undo()),
            redo: availability(self.history.can_redo()),
        }
    }

    /// The history, for inspection.
    #[must_use]
    pub fn history(&self) -> &History {
        &self.history
    }

    fn step(&mut self, step: HistoryIntent) -> Result<(), AppError> {
        let action = match step {
            HistoryIntent::Undo => EditorAction::Undo,
            HistoryIntent::Redo => EditorAction::Redo,
        };
        let availability = self.projection().availability(action);
        if !availability.is_available() {
            return Err(AppError::Unavailable {
                action,
                availability,
            });
        }
        match step {
            HistoryIntent::Undo => self.history.undo(),
            HistoryIntent::Redo => self.history.redo(),
        };
        Ok(())
    }
}
