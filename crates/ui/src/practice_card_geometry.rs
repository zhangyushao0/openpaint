//! What the practice card says of a box or an ellipse compared (S84): what lies over the copy, and
//! in plain words what the geometry shows, never a score. A completed box: how many straight edges
//! were found and how many head for their vanishing point. Free boxes: how many edges in how many
//! boxes, the boxes whose edges of a direction do not meet at one point on the horizon, the front
//! corners under 90 degrees, the verticals that lean; or that all is well. An ellipse: rounder or
//! flatter, turned which way, which sides it falls short of or runs past; or that it fits. Then
//! how to go on: correct in blue and compare again. The words are `text/practice_perspective.rs`'s.

use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::Direction;

use crate::components::card_text;
use crate::text::{self as t, Words, language};

/// A box or an ellipse compared; false when `practice` shows neither.
pub(crate) fn compared(ui: &mut egui::Ui, practice: &PracticeProjection) -> bool {
    let language = language(ui.ctx());
    let said = |ui: &mut egui::Ui, line: &str| card_text(ui, Words::made(line), false);
    match &practice.overlay {
        Some(Overlay::Box(check)) => {
            card_text(ui, t::SHOWS_BOX, true);
            let found = check.edges.len();
            if found == 0 {
                card_text(ui, t::NO_EDGES, false);
            } else {
                let on = check.edges.iter().filter(|edge| edge.on).count();
                said(ui, &t::box_edges(language, (found, on)));
            }
            card_text(ui, t::CORRECT_GROUP_IN_BLUE, true);
        }
        Some(Overlay::FreeBoxes(check)) => {
            card_text(ui, t::SHOWS_FREE_BOXES, true);
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
            card_text(ui, t::CORRECT_GROUP_IN_BLUE, true);
        }
        Some(Overlay::Ellipse(check)) => {
            card_text(ui, t::SHOWS_ELLIPSE, true);
            if check.fitted.is_none() {
                card_text(ui, t::NO_ELLIPSE, false);
            } else if check.roundness.is_none() && check.tilt.is_none() && check.sides.is_empty() {
                card_text(ui, t::ELLIPSE_FITS, false);
            } else {
                let shape = (check.roundness, check.tilt);
                said(ui, &t::ellipse_differs(language, shape, &check.sides));
            }
            card_text(ui, t::ELLIPSE_NEXT, true);
        }
        _ => return false,
    }
    true
}
