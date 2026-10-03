//! The session's side of the Transform tool (`transform.rs`): turning it on lifts the active
//! layer's content or the selection's (the runtime samples the pixels), turning it off or any
//! other edit commits it, and a selection's content moved leaves nothing selected.

use openpaint_core::Patch;

use super::Session;
use crate::error::AppError;
use crate::selection::SelectionIntent;
use crate::transform::{LiftFailed, TransformIntent, TransformJob};

impl Session {
    /// The pixels to lift for the transform just begun, once.
    pub fn transform_request(&mut self) -> Option<TransformJob> {
        self.transforms.job()
    }

    /// The runtime lifted the transform's pixels: the patch, or `None` when nothing is painted
    /// there.
    pub fn transform_lifted(&mut self, lifted: Result<Option<Patch>, LiftFailed>) {
        self.transforms.lifted(lifted);
    }

    /// True while the Transform tool is on.
    #[must_use]
    pub const fn transforming(&self) -> bool {
        self.transforms.active()
    }

    pub(super) fn transform(&mut self, intent: TransformIntent) -> Result<(), AppError> {
        match intent {
            TransformIntent::Tool(true) if !self.transforms.active() => {
                self.fills.interrupt();
                let canvas = self.artwork().canvas();
                self.selection.handle(canvas, SelectionIntent::Tool(false));
                let drawing = self.strokes.active().is_some();
                let clip = self.selection.mask().cloned();
                let current = self.history.current();
                self.transforms.begin(current, drawing, clip.as_ref())
            }
            TransformIntent::Tool(true) => Ok(()),
            TransformIntent::Tool(false) => {
                self.commit_transform();
                self.transforms.forget_refusal();
                Ok(())
            }
            TransformIntent::Place(placement) => {
                self.transforms.place(placement);
                Ok(())
            }
            TransformIntent::Settle => {
                self.transforms.settle();
                Ok(())
            }
            step => {
                self.transforms.step(&step);
                Ok(())
            }
        }
    }

    /// Commits a transform in progress; a selection's content moved leaves nothing selected
    /// (draweave), since what was selected is no longer there.
    pub(super) fn commit_transform(&mut self) {
        if self.transforms.commit(&mut self.history) {
            let canvas = self.artwork().canvas();
            self.selection.handle(canvas, SelectionIntent::Deselect);
        }
    }
}
