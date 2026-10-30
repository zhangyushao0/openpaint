//! What the practice card says of a box, free boxes or an ellipse compared (S84, S97, D108): its
//! title, then one short line of what is off, never a score, and never what the marks mean (each
//! How says that once); then Construction (看画法), which lays the way to see it faint over the
//! sheet and reads as on (its word in the ink pill) until tapped again. A box or free boxes: how
//! many edges miss their vanishing point, the verticals that lean, the front corners under 90
//! degrees; or that every edge heads for its point. An ellipse: rounder or flatter, turned which
//! way, which sides it falls short of or runs past; or that it fits. An estimate, a point in space
//! (S90), a solid (S93) or an object (S94): nothing but its title, the picture saying it all,
//! unless no mark, line or curve was found. The words are `text/practice_perspective.rs`'s,
//! `text/practice_points.rs`'s, `text/practice_solids.rs`'s and `text/practice_objects.rs`'s.

use openpaint_app::{Intent, Overlay, PracticeIntent, PracticeProjection};
use openpaint_practice::{Checked, Direction, Family};

use crate::components::{card_text, choices, text_choice};
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// A box or an ellipse compared; false when `practice` shows neither.
pub(crate) fn compared(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<Intent>,
) -> bool {
    let language = language(ui.ctx());
    let said = |ui: &mut egui::Ui, line: &str| card_text(ui, Words::made(line), false);
    match &practice.overlay {
        Some(Overlay::Box(check)) if check.edges.is_empty() => card_text(ui, t::NO_EDGES, false),
        Some(Overlay::Box(check)) => {
            said(ui, &t::off(language, counted(&check.edges, 0)));
            construction(ui, practice, taps);
        }
        Some(Overlay::FreeBoxes(check)) if check.boxes.is_empty() => {
            card_text(ui, t::NO_EDGES, false);
        }
        Some(Overlay::FreeBoxes(check)) => {
            let edges: Vec<Checked> = (check.boxes.iter())
                .flat_map(|free| free.edges.iter().copied())
                .collect();
            said(ui, &t::off(language, counted(&edges, check.narrow())));
            construction(ui, practice, taps);
        }
        Some(Overlay::Flat(check)) => nothing_read(ui, practice, check.metrics.is_empty()),
        Some(Overlay::Space(check)) => nothing_read(ui, practice, check.metrics.is_empty()),
        Some(Overlay::Object(check) | Overlay::Solid(check)) if check.metrics.is_empty() => {
            let object = matches!(practice.overlay, Some(Overlay::Object(_)));
            card_text(ui, if object { t::NO_OBJECT } else { t::NO_SOLID }, false);
        }
        Some(Overlay::Object(_) | Overlay::Solid(_)) => {}
        Some(Overlay::Ellipse(check)) => {
            if check.fitted.is_none() {
                card_text(ui, t::NO_ELLIPSE, false);
            } else if check.roundness.is_none() && check.tilt.is_none() && check.sides.is_empty() {
                card_text(ui, t::ELLIPSE_FITS, false);
            } else {
                let shape = (check.roundness, check.tilt);
                said(ui, &t::ellipse_differs(language, shape, &check.sides));
            }
            if check.fitted.is_some() {
                construction(ui, practice, taps);
            }
        }
        _ => return false,
    }
    true
}

/// Of `edges` checked, those that miss their vanishing point and the verticals that lean; with
/// `narrow` front corners.
fn counted(edges: &[Checked], narrow: usize) -> (usize, usize, usize) {
    let off = edges.iter().filter(|edge| !edge.on);
    let lean = off
        .clone()
        .filter(|edge| edge.direction == Direction::Vertical && edge.plumb.is_some())
        .count();
    (off.count() - lean, lean, narrow)
}

/// Construction (看画法): its word in the ink pill while the way to see it shows; a tap shows it
/// or takes it away.
fn construction(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    let on = practice.construction;
    ui.horizontal(|ui| {
        choices(ui, "construction", |ui, group| {
            if text_choice(ui, group, t::CONSTRUCTION, on).clicked() {
                push_once(taps, Intent::Practice(PracticeIntent::Construction(!on)));
            }
        });
    });
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

#[cfg(test)]
mod tests {
    use crate::text::{self as t, Language};

    #[test]
    fn a_box_s_card_says_what_is_off_in_one_short_line() {
        let en = |counts| t::off(Language::English, counts);
        assert_eq!(en((0, 0, 0)), "Every edge heads for its vanishing point.");
        assert_eq!(en((2, 0, 0)), "2 edges miss their vanishing point.");
        assert_eq!(
            en((1, 1, 2)),
            "1 edge misses its vanishing point; 1 vertical leans; 2 front corners under 90°."
        );
        let zh = t::off(Language::Chinese, (1, 0, 1));
        assert_eq!(zh, "1 条边没对准消失点，1 个前角小于 90°。");
        assert_eq!(t::cells_uneven(Language::English, 1), "1 cell uneven.");
        assert_eq!(t::cells_uneven(Language::Chinese, 3), "3 格不均匀。");
    }
}
