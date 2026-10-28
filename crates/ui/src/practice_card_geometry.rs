//! What the practice card says of a box or an ellipse compared (S84, D108): a short line of what
//! the geometry found, never a score, and never what the marks mean (each How says that once). A
//! completed box: how many of its edges head for their vanishing point. Free boxes: how many
//! boxes and edges, then the boxes whose edges of a direction do not meet, the front corners
//! under 90 degrees, the verticals that lean; or that all is well. An ellipse: rounder or
//! flatter, turned which way, which sides it falls short of or runs past; or that it fits. An
//! estimate or a point in space (S90): nothing but its title, the picture saying it all, unless no
//! mark or line was found. The words are `text/practice_perspective.rs`'s and
//! `text/practice_points.rs`'s.

use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Direction, Family};

use crate::components::card_text;
use crate::text::{self as t, Words, language};

/// A box or an ellipse compared; false when `practice` shows neither.
pub(crate) fn compared(ui: &mut egui::Ui, practice: &PracticeProjection) -> bool {
    let language = language(ui.ctx());
    let said = |ui: &mut egui::Ui, line: &str| card_text(ui, Words::made(line), false);
    match &practice.overlay {
        Some(Overlay::Box(check)) => {
            let found = check.edges.len();
            if found == 0 {
                card_text(ui, t::NO_EDGES, false);
            } else {
                let on = check.edges.iter().filter(|edge| edge.on).count();
                said(ui, &t::box_edges(language, (found, on)));
            }
        }
        Some(Overlay::FreeBoxes(check)) => {
            let edges: usize = check.boxes.iter().map(|free| free.edges.len()).sum();
            if edges == 0 {
                card_text(ui, t::NO_EDGES, false);
            } else {
                said(ui, &t::free_found(language, (edges, check.boxes.len())));
                let apart = check.boxes.iter().filter(|f| !f.apart.is_empty()).count();
                let narrow: usize = check.boxes.iter().map(|f| f.narrow.len()).sum();
                let lean = (check.boxes.iter().flat_map(|f| f.edges.iter()))
                    .filter(|e| e.direction == Direction::Vertical && !e.on)
                    .count();
                if apart + narrow + lean == 0 {
                    card_text(ui, t::BOXES_MEET, false);
                }
                if apart > 0 {
                    said(ui, &t::boxes_apart(language, apart));
                }
                if narrow > 0 {
                    said(ui, &t::corners_narrow(language, narrow));
                }
                if lean > 0 {
                    said(ui, &t::verticals_lean(language, lean));
                }
            }
        }
        Some(Overlay::Flat(check)) => nothing_read(ui, practice, check.metrics.is_empty()),
        Some(Overlay::Space(check)) => nothing_read(ui, practice, check.metrics.is_empty()),
        Some(Overlay::Ellipse(check)) => {
            if check.fitted.is_none() {
                card_text(ui, t::NO_ELLIPSE, false);
            } else if check.roundness.is_none() && check.tilt.is_none() && check.sides.is_empty() {
                card_text(ui, t::ELLIPSE_FITS, false);
            } else {
                let shape = (check.roundness, check.tilt);
                said(ui, &t::ellipse_differs(language, shape, &check.sides));
            }
        }
        _ => return false,
    }
    true
}

/// An estimate or a point in space compared: the picture speaks, unless `nothing` was read; then
/// that no mark (or no straight line, where a line is drawn) was found.
fn nothing_read(ui: &mut egui::Ui, practice: &PracticeProjection, nothing: bool) {
    if !nothing {
        return;
    }
    let marked = matches!(
        practice.family,
        Some(Family::Point | Family::Proportion | Family::Centre | Family::Mirror)
    );
    card_text(ui, if marked { t::NO_MARK } else { t::NO_EDGES }, false);
}
