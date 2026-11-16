//! Progress seen in the practice book: beside the calendar (S89) the first lesson's single line
//! beside the latest's, each its original in thin see-through red with the copy as first drawn in
//! ink over it, the lesson named under each, and the seals earned, stamped in vermilion; on the
//! Progress page (S147) each skill's curve (`curves.rs`). Only what there is shows: no axes, no
//! legends.

mod curves;

use egui::{Align2, Pos2, Rect, Shape, Stroke, TextStyle, Vec2};
use openpaint_practice::Point;
use openpaint_practice_app::BookProjection;

use openpaint_ui::components::paint_sheet;
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE};

use crate::components::{paint_seal, seal_turn};
use crate::palette::CORRECTION;
use crate::text as t;

pub(super) use curves::{cards_wanted, curves};

/// A line's frame's side and a seal's, in points.
const FRAME: f32 = 148.0;
const SEAL: f32 = 60.0;

/// The lines and the seals beside the calendar, in `rect`.
pub(super) fn beside(ui: &egui::Ui, rect: Rect, book: &BookProjection) {
    let mut top = rect.top();
    if !book.book.lines.is_empty() {
        top = paint_lines(ui, Pos2::new(rect.left(), top), book) + 3.0 * SPACE;
    }
    let per_row = ((rect.width() / (SEAL + 2.0 * SPACE)).floor() as usize).max(1);
    let language = t::language(ui.ctx());
    for (place, earned) in book.book.seals.iter().enumerate() {
        let (column, row) = ((place % per_row) as f32, (place / per_row) as f32);
        let pitch = SEAL + 2.0 * SPACE;
        let centre = Pos2::new(
            rect.left() + SEAL / 2.0 + SPACE + column * pitch,
            top + SEAL / 2.0 + SPACE + row * pitch,
        );
        let lines = t::seal_lines(language, earned.seal);
        paint_seal(
            ui.painter(),
            (centre, SEAL),
            (&lines, seal_turn(place)),
            1.0,
        );
    }
}

/// The lines shown from `at`, side by side; returns the bottom of their row.
fn paint_lines(ui: &egui::Ui, at: Pos2, book: &BookProjection) -> f32 {
    let language = t::language(ui.ctx());
    let small = TextStyle::Small.resolve(ui.style());
    let mut bottom = at.y;
    for (place, shown) in book.book.lines.iter().enumerate() {
        let min = at + Vec2::new(place as f32 * (FRAME + 3.0 * SPACE), 0.0);
        let frame = Rect::from_min_size(min, Vec2::splat(FRAME));
        paint_sheet(ui.painter(), None, frame, egui::Color32::WHITE);
        let read = (book.read.get(&shown.record))
            .and_then(|read| read.lines.iter().find(|line| line.index == shown.index));
        if let Some(line) = read {
            let painter = ui.painter().with_clip_rect(frame);
            let to = |p: &Point| frame.min + Vec2::new(p[0] as f32, p[1] as f32) * FRAME;
            let red = Stroke::new(1.5, CORRECTION.gamma_multiply(0.5));
            for piece in &line.original {
                painter.add(Shape::line(piece.iter().map(to).collect(), red));
            }
            for piece in &line.copy {
                painter.add(Shape::line(
                    piece.iter().map(to).collect(),
                    Stroke::new(1.5, INK),
                ));
            }
        }
        let caption = t::lesson_number(language, shown.number);
        let under = frame.center_bottom() + Vec2::new(0.0, SPACE / 2.0);
        ui.painter().text(
            under,
            Align2::CENTER_TOP,
            caption,
            small.clone(),
            INK_SECONDARY,
        );
        bottom = frame.bottom() + 3.0 * SPACE;
    }
    bottom
}
