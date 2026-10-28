//! Progress seen in the practice book (S89): the colour's first pick per session (ΔE × 100, the
//! eye's own level before any correction) as an ink line over a faint wash, the first and the
//! latest written at their points, a pencil baseline; the first session's single line beside the
//! latest's, each its original in thin see-through red with the copy as first drawn in ink over
//! it, the session named under each; and the seals earned, stamped in vermilion. Only what there
//! is shows: no axes, no legends.

use egui::{Align2, Pos2, Rect, Shape, Stroke, TextStyle, Vec2};
use openpaint_app::BookProjection;
use openpaint_practice::Point;

use crate::components::{paint_seal, paint_sheet, seal_turn};
use crate::design::{CORRECTION, INK, INK_SECONDARY, PENCIL, SPACE};
use crate::text as t;

/// The colour chart's size, a line's frame's side and a seal's, in points.
const CHART: Vec2 = Vec2::new(520.0, 132.0);
const FRAME: f32 = 148.0;
const SEAL: f32 = 60.0;

/// Progress in `rect`.
pub(super) fn progress(ui: &egui::Ui, rect: Rect, book: &BookProjection) {
    let mut top = rect.top();
    let colour = &book.book.colour;
    if !colour.is_empty() {
        let size = Vec2::new(CHART.x.min(rect.width()), CHART.y);
        let chart = Rect::from_min_size(Pos2::new(rect.left(), top), size);
        paint_chart(ui, chart, colour);
        top = chart.bottom() + 3.0 * SPACE;
    }
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

/// The first picks (Oklab units) as an ink line across `chart`, the first and the latest written
/// × 100 at their points.
fn paint_chart(ui: &egui::Ui, chart: Rect, picks: &[f64]) {
    let painter = ui.painter();
    let small = TextStyle::Small.resolve(ui.style());
    let room = 4.0 * SPACE;
    let base = chart.bottom() - SPACE;
    let highest = picks.iter().copied().fold(f64::MIN_POSITIVE, f64::max) as f32;
    let span = chart.width() - 2.0 * room;
    let points: Vec<Pos2> = (picks.iter().enumerate())
        .map(|(index, pick)| {
            let share = if picks.len() == 1 {
                0.5
            } else {
                index as f32 / (picks.len() - 1) as f32
            };
            let height = (*pick as f32 / highest) * (base - chart.top() - room);
            Pos2::new(chart.left() + room + share * span, base - height)
        })
        .collect();
    for pair in points.windows(2) {
        let quad = vec![
            pair[0],
            pair[1],
            Pos2::new(pair[1].x, base),
            Pos2::new(pair[0].x, base),
        ];
        painter.add(Shape::convex_polygon(
            quad,
            INK.gamma_multiply(0.06),
            Stroke::NONE,
        ));
    }
    let baseline = [
        Pos2::new(chart.left(), base),
        Pos2::new(chart.right(), base),
    ];
    painter.line_segment(baseline, Stroke::new(1.0, PENCIL));
    painter.add(Shape::line(points.clone(), Stroke::new(1.75, INK)));
    for point in &points {
        painter.circle_filled(*point, 2.5, INK);
    }
    let written = [0, points.len() - 1];
    for index in written.into_iter().take(points.len().min(2)) {
        let value = format!("{:.1}", picks[index] * 100.0);
        let above = points[index] - Vec2::new(0.0, SPACE);
        painter.text(
            above,
            Align2::CENTER_BOTTOM,
            value,
            small.clone(),
            INK_SECONDARY,
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
        let caption = t::session_number(language, shown.number);
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
