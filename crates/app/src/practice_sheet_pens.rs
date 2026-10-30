//! The pens a practice sheet's exercise is drawn or painted with (`practice_sheet.rs`): which
//! fit the shown exercise, the style the next stroke takes, and a whole copy's two layers (S95):
//! its division on one, its line drawing with Draw on the one above, the division dimmed while
//! Draw is chosen (V大: lower the division's opacity, then draw over it) and hidden while compared,
//! as the session lays them (`practice_layers.rs`).

use openpaint_core::{StrokeStyle, StrokeTool};
use openpaint_practice::Family;

use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::practice::Pen;
use crate::projection::Availability;

/// What a sheet holding a whole copy wants of its division's and line drawing's layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layering {
    /// Another exercise shows, or the sheet is done: the division shows (dimmed or not).
    Rest,
    /// The whole copy shows with its own tools: strokes go on the line drawing's layer while
    /// Draw is chosen (`lines`, the division dimmed under it), else on the division's; the
    /// division hidden while compared.
    Copying { lines: bool, hidden: bool },
}

impl PracticeSheet {
    /// The style the next stroke takes instead of the toolbox's `brush`: the pen chosen, while
    /// the exercise's own tools show and the toolbox paints (its eraser, held or chosen,
    /// erases); a painting pen in `brush`'s colour. A colour is filled, not painted.
    pub(crate) fn pen_style(&self, brush: &StrokeStyle) -> Option<StrokeStyle> {
        let family = self.family(self.current()?)?;
        let pens = !self.all_tools && family != Family::Colour && brush.tool() == StrokeTool::Paint;
        pens.then(|| self.pens.stroke(self.pen, brush.color()))
    }

    /// The shown exercise is drawn or painted with `pen` from now on: a drawing pen for lines and
    /// shapes (the blue one only once compared), a painting pen for painted exercises, a level's
    /// pen for a division, those or Draw for a whole copy (S95).
    pub(super) fn choose(&mut self, pen: Pen) -> Result<(), AppError> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let compared = self.record.result(index).is_some();
        let level = pen.level().is_some();
        let fits = match self.family(index) {
            Some(Family::Division) => level,
            Some(Family::WholeCopy) => level || pen == Pen::Draw,
            _ if self.drawn(index) => !pen.paints() && !level && (pen != Pen::Correct || compared),
            _ => pen.paints(),
        };
        if !fits {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.pen = pen;
        if pen.paints() {
            self.painting = pen;
        } else if level {
            self.dividing = pen;
        } else if pen != Pen::Correct {
            self.drawing = pen;
        }
        Ok(())
    }

    /// What a sheet holding a whole copy wants of its two layers now; none on another sheet, or
    /// while the full interface shows (the layers are the user's then).
    pub(crate) fn layering(&self) -> Option<Layering> {
        let holds = (self.record.exercises.iter()).any(|e| e.kind.family() == Family::WholeCopy);
        if !holds || self.all_tools {
            return None;
        }
        Some(match self.current() {
            Some(index) if self.family(index) == Some(Family::WholeCopy) => Layering::Copying {
                lines: self.pen == Pen::Draw,
                hidden: self.compared.is_some(),
            },
            _ => Layering::Rest,
        })
    }
}
