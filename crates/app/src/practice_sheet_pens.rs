//! The pens a practice sheet's exercise is drawn or painted with (`practice_sheet.rs`): which
//! fit the shown exercise, the style the next stroke takes, and a whole copy's layers (S95, S96):
//! its division on one, its line drawing with Draw on the one above, its colour blocks with the
//! painting pens on one under the division (made the first time one is chosen), the division
//! dimmed while Draw or a painting pen is chosen (V大: lower the division's opacity, then draw or
//! paint over it; over the blocks it stays a guide) and hidden while compared, as the session lays
//! them (`practice_layers.rs`).

use openpaint_core::{Color, StrokeStyle, StrokeTool};
use openpaint_practice::Family;

use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::practice::Pen;
use crate::practice_pens::value_colour;
use crate::projection::Availability;

/// What a sheet holding a whole copy wants of its layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layering {
    /// Another exercise shows, or the sheet is done: the division shows (dimmed or not).
    Rest,
    /// The whole copy shows with its own tools: strokes go on the layer `on` (the division
    /// dimmed while it is the line drawing's or the colour blocks'); the division hidden while
    /// compared.
    Copying { on: CopyLayer, hidden: bool },
}

/// A whole copy's layer the pen draws on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyLayer {
    Division,
    Lines,
    Blocks,
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
    /// pen for a division, those, Draw or a painting pen for a whole copy (S95, S96), a grey for
    /// value masses (S96).
    pub(super) fn choose(&mut self, pen: Pen) -> Result<(), AppError> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let compared = self.record.result(index).is_some();
        let (level, valued) = (pen.level().is_some(), pen.value().is_some());
        let fits = match self.family(index) {
            Some(Family::Division) => level,
            Some(Family::WholeCopy) => level || pen == Pen::Draw || pen.paints(),
            Some(Family::ValueMasses) => valued,
            _ if self.drawn(index) => {
                !pen.paints() && !level && !valued && (pen != Pen::Correct || compared)
            }
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
        } else if valued {
            self.valuing = pen;
        } else if pen != Pen::Correct {
            self.drawing = pen;
        }
        Ok(())
    }

    /// The colour a fill of the shown exercise takes when `asked` is the colour picked: value
    /// masses' grey chosen (S96), else `asked`.
    pub(crate) fn fill_colour(&self, asked: Color) -> Color {
        let masses =
            (self.current()).and_then(|index| self.family(index)) == Some(Family::ValueMasses);
        (masses.then(|| self.valuing.value()).flatten())
            .and_then(|value| value_colour(value).ok())
            .unwrap_or(asked)
    }

    /// What a sheet holding a whole copy wants of its layers now; none on another sheet, or while
    /// the full interface shows (the layers are the user's then).
    pub(crate) fn layering(&self) -> Option<Layering> {
        let holds = (self.record.exercises.iter()).any(|e| e.kind.family() == Family::WholeCopy);
        if !holds || self.all_tools {
            return None;
        }
        let on = match self.pen {
            Pen::Draw => CopyLayer::Lines,
            pen if pen.paints() => CopyLayer::Blocks,
            _ => CopyLayer::Division,
        };
        Some(match self.current() {
            Some(index) if self.family(index) == Some(Family::WholeCopy) => Layering::Copying {
                on,
                hidden: self.compared.is_some(),
            },
            _ => Layering::Rest,
        })
    }
}
