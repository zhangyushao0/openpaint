//! What the day's sheet says of its lessons (S122), over the canvas through the view: each
//! lesson's label above the row it starts ("Lesson 3 · Lines"), quiet, and each result of the
//! lessons before the one in hand, and its own once it ends, as a small mark under its pair (a dot,
//! a ring half filled, an empty ring). Never drawn into the artwork.

use egui::{Align2, FontId, Order, Pos2};
use openpaint_app::ViewMapping;
use openpaint_practice::Layout;
use openpaint_practice_app::PracticeProjection;

use openpaint_ui::components::canvas_point;
use openpaint_ui::design::{INK, INK_SECONDARY};

use crate::components::paint_result;
use crate::text as t;

/// A label's type size, how far above its row it stands, a result's mark's radius and how far
/// under its frames, in canvas pixels; a mark's least and most radius on screen, in points (the
/// most: an earlier lesson's marks stay small while the view is close on the pair in hand).
const LABEL: f32 = 26.0;
const ABOVE: f64 = 22.0;
const MARK: f32 = 11.0;
const UNDER: f64 = 36.0;
const LEAST_MARK: f32 = 3.0;
const MOST_MARK: f32 = 6.0;

/// The rows' labels and the results marked under their pairs, through `mapping` on a screen of
/// `pixels_per_point`.
pub(crate) fn show_rows(
    ctx: &egui::Context,
    practice: &PracticeProjection,
    (mapping, pixels_per_point): (&ViewMapping, f32),
) {
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("practice sheet rows"));
    let painter = ctx.layer_painter(layer);
    let at = |point: [f64; 2]| -> Pos2 {
        canvas_point(mapping, pixels_per_point, point.map(|v| v as f32))
    };
    let scale = mapping.scale() / pixels_per_point;
    let language = t::language(ctx);
    for row in &practice.sheet.rows {
        let pair = Layout::pair_at(usize::from(row.slot), None);
        let [x, y] = pair.original.origin;
        let font = FontId::proportional((LABEL * scale).max(1.0));
        let label = t::row_label(language, row);
        painter.text(
            at([x, y - ABOVE]),
            Align2::LEFT_BOTTOM,
            label,
            font,
            INK_SECONDARY,
        );
    }
    let radius = (MARK * scale).clamp(LEAST_MARK, MOST_MARK);
    for (index, outcome) in &practice.marked {
        let Some(pair) = practice.sheet.pairs.get(*index) else {
            continue;
        };
        let [middle, _] = pair.centre();
        let bottom = pair.copy.origin[1] + pair.copy.size[1].max(pair.original.size[1]);
        paint_result(
            &painter,
            (at([middle, bottom + UNDER]), radius),
            *outcome,
            INK,
        );
    }
}
