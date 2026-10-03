//! `Session`: the open artwork, its history and the editor flow around it. The UI emits
//! intents, the session applies them; the renderer reads the current version and the
//! in-progress stroke beside it; the UI reads the projection; the runtime asks it which version
//! to save and reports how saving went. The session also keeps how the artwork is viewed, which
//! is no history step and is not stored.

use openpaint_core::{ActiveStroke, Artwork, CanvasSize, RegionMask, StrokeStyle};

use crate::error::AppError;
use crate::fill::{FillFailed, FillJob, Fills};
use crate::history::{HISTORY_BUDGET, History, HistoryIntent};
use crate::intent::Intent;
use crate::layers::{self, LayerList, Layers};
use crate::project::PortError;
use crate::projection::{Availability, EditorAction, LAYER_ACTIONS, SessionProjection};
use crate::storage::{Autosave, StorageIntent};
use crate::streamline::StreamLine;
use crate::stroke::Strokes;
use crate::view::{CanvasView, ViewIntent};

#[derive(Debug)]
pub struct Session {
    history: History,
    strokes: Strokes,
    layers: Layers,
    fills: Fills,
    autosave: Autosave,
    /// How the artwork shows; it lives as long as the session (every artwork opens at fit).
    view: CanvasView,
}

impl Session {
    /// A blank artwork of `canvas` pixels with an empty history; strokes start with `brush`
    /// (the toolbox's style, `Toolbox::style`).
    #[must_use]
    pub fn new(canvas: CanvasSize, brush: StrokeStyle) -> Self {
        Self::open(Artwork::new(canvas), brush)
    }

    /// `artwork` as the store holds it, with an empty history (history is not stored).
    #[must_use]
    pub fn open(artwork: Artwork, brush: StrokeStyle) -> Self {
        Self {
            autosave: Autosave::new(artwork.clone()),
            history: History::new(artwork, HISTORY_BUDGET),
            strokes: Strokes::new(brush),
            layers: Layers::default(),
            fills: Fills::default(),
            view: CanvasView::FIT,
        }
    }

    /// A blank artwork after the last one could not be opened; the projection tells the user
    /// until they close the notice.
    #[must_use]
    pub fn after_failed_open(canvas: CanvasSize, brush: StrokeStyle) -> Self {
        let mut session = Self::new(canvas, brush);
        session.autosave = Autosave::after_failed_open(session.artwork().clone());
        session
    }

    /// The next stroke starts with `brush`; a stroke being drawn keeps the style it began with.
    pub fn set_brush(&mut self, brush: StrokeStyle) {
        self.strokes.set_style(brush);
    }

    /// The next stroke follows the pen through `streamline` (the toolbox's, per brush); a
    /// stroke being drawn keeps the StreamLine it began with.
    pub const fn set_streamline(&mut self, streamline: StreamLine) {
        self.strokes.set_streamline(streamline);
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
            self.fills.interrupt();
            return self.layers.handle(&mut self.history, drawing, layer);
        }
        if let Intent::View(view) = intent {
            self.navigate(view);
            return Ok(());
        }
        self.layers.interrupt();
        if let Intent::Fill(fill) = intent {
            let drawing = self.strokes.active().is_some();
            return self.fills.handle(&mut self.history, drawing, fill);
        }
        if matches!(
            intent,
            Intent::Stroke(_) | Intent::History(_) | Intent::Gallery(_)
        ) {
            self.fills.interrupt();
        }
        match intent {
            Intent::Stroke(stroke) => {
                let mut artwork = self.history.current().clone();
                let result = self.strokes.handle(&mut artwork, stroke);
                self.history.record(artwork);
                result
            }
            Intent::History(step) => self.step(step),
            Intent::Storage(storage) => self.storage(storage),
            // The toolbox, export and the reference belong to the app, not to this artwork: the
            // runtime hands their intents to them (and the brush's style to `set_brush`).
            Intent::Layer(_)
            | Intent::View(_)
            | Intent::Fill(_)
            | Intent::Brush(_)
            | Intent::Tool(_)
            | Intent::Export(_)
            | Intent::Reference(_) => Ok(()),
            // Leaving is the runtime's (it stores the artwork first); the session only says
            // whether it may happen now.
            Intent::Gallery(_) => self.check(EditorAction::Gallery),
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

    /// What the canvas shows: the current version, or the preview of the fill being placed on
    /// it (which is not in history).
    #[must_use]
    pub fn shown(&self) -> &Artwork {
        let current = self.history.current();
        self.fills.shown(current).unwrap_or(current)
    }

    /// The newest fill request for the fill worker, when none runs and it was not handed out.
    pub fn fill_request(&mut self) -> Option<FillJob> {
        self.fills.job()
    }

    /// The fill worker computed request `sequence`: the region, or why there is none.
    pub fn fill_finished(&mut self, sequence: u64, result: Result<RegionMask, FillFailed>) {
        self.fills.finished(&mut self.history, sequence, result);
    }

    /// True while a fill is being placed or adjusted.
    #[must_use]
    pub fn filling(&self) -> bool {
        self.fills.projection().active
    }

    /// What the UI may offer now. Something there is nothing for is `NothingLeft` even while a
    /// stroke is drawn, so a control's look never changes between pen-down and pen-up.
    #[must_use]
    pub fn projection(&self) -> SessionProjection {
        let availability = |possible: bool| {
            if !possible {
                Availability::NothingLeft
            } else if self.strokes.active().is_some() {
                Availability::StrokeInProgress
            } else {
                Availability::Available
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
            gallery: availability(true),
            storage: self.autosave.notice(),
            layers: Some(LayerList::of(artwork)),
            view: self.view,
            canvas: Some(artwork.canvas()),
            drawing,
            fill: self.fills.projection(),
            brush: None,
            transfers: None,
        }
    }

    /// How the artwork shows now.
    #[must_use]
    pub const fn view(&self) -> CanvasView {
        self.view
    }

    /// Navigation never touches the artwork or its history.
    fn navigate(&mut self, intent: ViewIntent) {
        match intent {
            ViewIntent::Gesture { gesture, viewport } => {
                let canvas = self.artwork().canvas();
                self.view = self.view.after(&gesture, canvas, &viewport);
            }
            ViewIntent::Fit => self.view = CanvasView::FIT,
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
