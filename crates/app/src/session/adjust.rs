//! The session's side of adjustments (`adjust.rs`): choosing one starts it on what shows now,
//! leaving the Selection tool (the selection stays and limits it); Apply, Cancel, undo (which
//! cancels) and any other edit or tool end it.

use super::Session;
use crate::adjust::AdjustIntent;
use crate::error::AppError;

impl Session {
    /// True while an adjustment is under way.
    #[must_use]
    pub const fn adjusting(&self) -> bool {
        self.adjusts.active()
    }

    pub(super) fn adjust(&mut self, intent: AdjustIntent) -> Result<(), AppError> {
        match intent {
            AdjustIntent::Begin(kind) => {
                // Another adjustment chosen keeps the one under way, as it shows.
                self.adjusts.apply(&mut self.history);
                let drawing = self.strokes.active().is_some();
                let clip = self.selection.mask().cloned();
                let current = self.history.current();
                self.adjusts.begin(current, drawing, kind, clip.as_ref())
            }
            AdjustIntent::Set(adjustment) => {
                self.adjusts.set(adjustment);
                Ok(())
            }
            AdjustIntent::Apply => {
                self.adjusts.apply(&mut self.history);
                Ok(())
            }
            AdjustIntent::Cancel => {
                self.adjusts.cancel();
                Ok(())
            }
            AdjustIntent::Dismiss => {
                self.adjusts.dismiss();
                Ok(())
            }
        }
    }

    /// Any other edit, tool or leaving keeps the adjustment under way as it shows; its notice
    /// goes.
    pub(super) fn apply_adjustment(&mut self) {
        self.adjusts.apply(&mut self.history);
        self.adjusts.dismiss();
    }
}
