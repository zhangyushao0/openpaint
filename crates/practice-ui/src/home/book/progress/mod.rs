//! Progress seen in the practice book: beside the calendar (S89) the first lesson's single line
//! beside the latest's, each its original in thin see-through red with the copy as first drawn in
//! ink over it, the lesson named under each; under them, once a drawing from life was begun
//! (S151), the first lesson's drawing of what lay in front of the learner beside the first and
//! the latest drawing from life, each as first drawn in a frame of its shape, named under it; and
//! the seals earned, stamped in vermilion; on the Progress page (S147) each skill's curve
//! (`curves.rs`). Only what there is shows: no axes, no legends.

mod curves;

use egui::{Align2, Pos2, Rect, Shape, Stroke, TextStyle, Vec2};
use openpaint_practice::{LineShown, Point};
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

/// The lines, the drawings from life after the first lesson's drawing (S151) and the seals beside
/// the calendar, in `rect`.
pub(super) fn beside(ui: &egui::Ui, rect: Rect, book: &BookProjection) {
    let mut top = rect.top();
    for row in [&book.book.lines, &book.book.life] {
        if !row.is_empty() {
            top = paint_lines(ui, (Pos2::new(rect.left(), top), row), book) + 3.0 * SPACE;
        }
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

/// The lines or drawings `row` shows from `at`, side by side, each in a frame of its shape;
/// returns the bottom of their row.
fn paint_lines(ui: &egui::Ui, (at, row): (Pos2, &[LineShown]), book: &BookProjection) -> f32 {
    let small = TextStyle::Small.resolve(ui.style());
    let mut bottom = at.y;
    for (place, shown) in row.iter().enumerate() {
        let min = at + Vec2::new(place as f32 * (FRAME + 3.0 * SPACE), 0.0);
        let read = (book.read.get(&shown.record))
            .and_then(|read| read.lines.iter().find(|line| line.index == shown.index));
        // A drawing from life's frame is its photo's shape, at the slot's top.
        let extent = read.map_or([1.0, 1.0], |line| line.extent.map(|side| side as f32));
        let frame = Rect::from_min_size(min, Vec2::new(extent[0], extent[1]) * FRAME);
        paint_sheet(ui.painter(), None, frame, egui::Color32::WHITE);
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
        let caption = caption(ui.ctx(), book, shown);
        let under = frame.center_bottom() + Vec2::new(0.0, SPACE / 2.0);
        ui.painter().text(
            under,
            Align2::CENTER_TOP,
            caption,
            small.clone(),
            INK_SECONDARY,
        );
        bottom = bottom.max(frame.bottom() + 3.0 * SPACE);
    }
    bottom
}

/// What names `shown` under its frame: a project's name (a drawing from life's, S151), else its
/// lesson's number.
fn caption(ctx: &egui::Context, book: &BookProjection, shown: &LineShown) -> String {
    let named = book
        .record(shown.record)
        .and_then(|record| record.copy.as_ref());
    match named {
        Some(title) => t::shown_name(ctx, title.as_str()).into_owned(),
        None => t::lesson_number(t::language(ctx), shown.number),
    }
}
