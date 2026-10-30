//! What the practice card shows of a division or a whole copy compared (S95, D108: shows, not
//! tells). Both: their title and the slider setting how strongly the picture lies under the
//! division or over the copy (half at first: a dark picture raised reads under the lines); a
//! whole copy's point the eyedropper compared where its colour blocks were painted, on the wheel
//! and in the colour table (S96). A
//! division, under it: a table of its three levels, each named beside a short line in its colour,
//! the lines drawn and how far they run on the mean (a share of the picture's longer side), the
//! main lines' count in the marking red past V大's five; no sentence.

use egui::{Align2, Sense, Stroke, TextStyle, Vec2};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Family, Level, LevelRow, MOST_MAIN};

use crate::design::{CORRECTION, INK, INK_SECONDARY, SPACE};
use crate::practice_card_colour::shown_slider;
use crate::practice_division_marks::level_colour;
use crate::text::{self as t, language};

/// The table's number columns' width, and the level's colour line before its name, in points.
const COLUMN: f32 = 96.0;
const SWATCH: f32 = 3.0 * SPACE;

/// A division or a whole copy compared; false when `practice` shows neither.
pub(crate) fn compared(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<openpaint_app::Intent>,
) -> bool {
    match &practice.overlay {
        Some(Overlay::Division { check, shown, .. }) => {
            shown_slider(ui, *shown, taps);
            let language = language(ui.ctx());
            let heads = [t::LINES_DRAWN, t::MEAN_LENGTH].map(|head| head.get(language));
            row(ui, (None, ""), [heads[0], heads[1]], None);
            for level in Level::ALL {
                let LevelRow { lines, length } = check.rows[level.index()];
                let count = lines.to_string();
                let mean = length.map_or_else(|| "—".to_owned(), t::mean_length);
                let red = (level == Level::Main && lines > MOST_MAIN).then_some(CORRECTION);
                let name = t::level(level).get(language);
                row(ui, (Some(level), name), [&count, &mean], red);
            }
            true
        }
        Some(Overlay::Picture { shown, .. }) if practice.family == Some(Family::WholeCopy) => {
            shown_slider(ui, *shown, taps);
            crate::practice_card_colour::whole_copy_probe(ui, practice);
            true
        }
        _ => false,
    }
}

/// A row of the table: a level's colour line and its name (none: the heads' row, quiet), and two
/// numbers at the right, the first in `red` where given.
fn row(
    ui: &mut egui::Ui,
    (level, name): (Option<Level>, &str),
    [first, second]: [&str; 2],
    red: Option<egui::Color32>,
) {
    let font = TextStyle::Body.resolve(ui.style());
    let height = ui.text_style_height(&TextStyle::Body) + SPACE;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    let ink = if level.is_some() { INK } else { INK_SECONDARY };
    let middle = rect.center().y;
    if let Some(level) = level {
        let ends = [rect.left(), rect.left() + SWATCH - SPACE].map(|x| egui::pos2(x, middle));
        painter.line_segment(ends, Stroke::new(3.0, level_colour(level)));
        let left = egui::pos2(rect.left() + SWATCH, middle);
        painter.text(left, Align2::LEFT_CENTER, name, font.clone(), ink);
    }
    let columns = [(first, red.unwrap_or(ink), COLUMN), (second, ink, 0.0)];
    for (text, colour, before) in columns {
        let at = egui::pos2(rect.right() - before, middle);
        painter.text(at, Align2::RIGHT_CENTER, text, font.clone(), colour);
    }
}
