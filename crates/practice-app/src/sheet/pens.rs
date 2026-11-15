//! The pens a practice sheet's exercise is drawn or painted with (`sheet.rs`): where a pen
//! may land, which fit the shown exercise (each colour kind only what its work needs, S100), the
//! style the next stroke takes, and a whole copy's layers (S95, S96): its division on one, its line
//! drawing with Draw on the one above, its colour blocks with the painting pens on one under the
//! division (made the first time one is chosen), the division dimmed while Draw or a painting pen
//! is chosen (V大: lower the division's opacity, then draw or paint over it; over the blocks it
//! stays a guide) and hidden while compared, as the session lays them (`layers.rs`).

use openpaint_core::{Color, StrokeStyle, StrokeTool};
use openpaint_practice::Facts;

use super::{PracticeSheet, unavailable};
use crate::pens::value_colour;
use crate::projection::{Pen, PracticeNotice};
use crate::refused::Refused;
use openpaint_app::Availability;

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
    /// Whether a pen landing at `point` (canvas pixels) may draw, the sheet's picture layer
    /// holding `placed` pictures; if not, the notice says why. Drawing in the frame takes the
    /// comparison away: a correction is made by eye. On a value scale it takes the cell between
    /// the ends it lands in, for Fill cell, and lays nothing with the scale's own tools (S100: a
    /// cell is one flat value; with all tools the brush paints it). A frame whose picture lies
    /// under the marks takes nothing until it is laid (S112: so an undo never crosses it).
    pub(crate) fn may_draw(&mut self, point: [f32; 2], placed: usize) -> bool {
        let point = point.map(f64::from);
        let landed = match self.current() {
            None => Err(PracticeNotice::Done),
            Some(index) if self.waits_for_picture(index, placed) => Ok(false),
            Some(index) => {
                let frame = self.sheet.pairs[index].copy;
                if frame.holds(point) {
                    let cell = self.take_cell(index, frame.to_frame(point));
                    cell.map(|taken| !taken || self.all_tools)
                } else {
                    Err(PracticeNotice::DrawInFrame)
                }
            }
        };
        // A pen that may land lets an earlier refusal's notice go.
        self.notice = landed.err();
        if landed.is_ok() {
            self.laid();
        }
        landed == Ok(true)
    }

    /// The style the next stroke takes instead of the toolbox's `brush`: the pen chosen, while
    /// the exercise's own tools show and the toolbox paints (its eraser, held or chosen, erases);
    /// a painting pen in `brush`'s colour. A colour is filled and a value scale's cell filled, not
    /// painted; a long colour copy shows the full interface only, so its toolbox's brush paints
    /// (S104).
    pub(crate) fn pen_style(&self, brush: &StrokeStyle) -> Option<StrokeStyle> {
        let tools = &self.shown_facts()?.tools;
        let offered = !tools.pens.is_empty() || tools.paints;
        let pens = !self.all_tools && offered && brush.tool() == StrokeTool::Paint;
        pens.then(|| self.pens.stroke(self.pen, brush.color()))
    }

    /// The shown exercise is drawn or painted with `pen` from now on: a drawing pen for lines and
    /// shapes (the blue one only once compared), a painting pen for a colour combination, a
    /// level's pen for a division, those, Draw or a painting pen for a whole copy (S95, S96), a
    /// grey for value masses (S96); none for a colour or a value scale, which are filled (S100),
    /// nor for a long colour copy, painted with the toolbox's own brushes (S104).
    pub(super) fn choose(&mut self, pen: Pen) -> Result<(), Refused> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let compared = self.record.first(index).is_some();
        let (level, valued) = (pen.level().is_some(), pen.value().is_some());
        let fits = match self.facts(index) {
            Some(facts) => facts.tools.offers(pen, compared),
            None => pen.paints(),
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
        let greys = (self.shown_facts()).is_some_and(|facts| facts.tools.greys());
        (greys.then(|| self.valuing.value()).flatten())
            .and_then(|value| value_colour(value).ok())
            .unwrap_or(asked)
    }

    /// What a sheet holding a whole copy wants of its layers now; none on another sheet, or while
    /// the full interface shows (the layers are the user's then).
    pub(crate) fn layering(&self) -> Option<Layering> {
        let copying = |facts: &Facts| facts.tools.layered;
        let holds = (self.record.exercises()).any(|laid| copying(laid.exercise.kind.facts()));
        if !holds || self.all_tools {
            return None;
        }
        let on = match self.pen {
            Pen::Draw => CopyLayer::Lines,
            pen if pen.paints() => CopyLayer::Blocks,
            _ => CopyLayer::Division,
        };
        Some(match self.current() {
            Some(index) if self.facts(index).is_some_and(copying) => Layering::Copying {
                on,
                hidden: self.compared.is_some(),
            },
            _ => Layering::Rest,
        })
    }
}
