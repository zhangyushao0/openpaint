//! What the day's sheet says of its lessons (S122), over the canvas through the view: each
//! lesson's label above the row it starts ("Lesson 3 · Lines"), quiet, and each result of the
//! lessons before the one in hand, and its own once it ends, as a small mark under its pair (a dot,
//! a ring half filled, an empty ring). The two share the gap between rows and never meet: where
//! it is short (the whole day in view) both close in on their own rows and the label shrinks only
//! as far as it must. Never drawn into the artwork.

use egui::{Align2, FontId, Order, Pos2, Vec2};
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
const ABOVE: f32 = 22.0;
const MARK: f32 = 11.0;
const UNDER: f32 = 36.0;
const LEAST_MARK: f32 = 3.0;
const MOST_MARK: f32 = 6.0;
/// A label's least type size on screen, in points: still read when the view shows the whole day.
const LEAST_LABEL: f32 = 10.0;
/// A label's line as a share of its type size, and the least room between a label and the marks
/// above it, and between either and its row when the gap is short, in points.
const LINE: f32 = 1.25;
const MARGIN: f32 = 2.0;
const HUG: f32 = 1.0;

/// How a row's label and the marks above it lie in a gap between rows, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Spacing {
    /// A mark's centre under its frames.
    under: f32,
    /// The label's bottom above its row.
    above: f32,
    /// The label's type size.
    size: f32,
}

/// The layer the rows' labels and results are drawn in, over the canvas.
pub(super) fn layer() -> egui::LayerId {
    egui::LayerId::new(Order::Background, egui::Id::new("practice sheet rows"))
}

/// The rows' labels and the results marked under their pairs, through `mapping` on a screen of
/// `pixels_per_point`.
pub(crate) fn show_rows(
    ctx: &egui::Context,
    practice: &PracticeProjection,
    (mapping, pixels_per_point): (&ViewMapping, f32),
) {
    let painter = ctx.layer_painter(layer());
    let at = |point: [f64; 2]| -> Pos2 {
        canvas_point(mapping, pixels_per_point, point.map(|v| v as f32))
    };
    let scale = mapping.scale() / pixels_per_point;
    let language = t::language(ctx);
    let radius = (MARK * scale).clamp(LEAST_MARK, MOST_MARK);
    let between = spacing(row_gap() * scale, scale, radius);
    for row in &practice.sheet.rows {
        let [x, y] = Layout::pair_at(usize::from(row.slot), None).original.origin;
        // The first row has the sheet's edge above it, no marks.
        let first = spacing(y as f32 * scale, scale, 0.0);
        let shown = if row.slot == 0 { first } else { between };
        let font = FontId::proportional(shown.size);
        let bottom = at([x, y]) - Vec2::new(0.0, shown.above);
        let label = t::row_label(language, row);
        painter.text(bottom, Align2::LEFT_BOTTOM, label, font, INK_SECONDARY);
    }
    for (index, outcome) in &practice.marked {
        let Some(pair) = practice.sheet.pairs.get(*index) else {
            continue;
        };
        let [middle, _] = pair.centre();
        let bottom = pair.copy.origin[1] + pair.copy.size[1].max(pair.original.size[1]);
        let centre = at([middle, bottom]) + Vec2::new(0.0, between.under);
        paint_result(&painter, (centre, radius), *outcome, INK);
    }
}

/// The label and the marks in a gap `gap` points tall at `scale` points a canvas pixel, the marks
/// `radius` points (none: 0): as drawn close up while they fit, else hugging their rows, the
/// label no larger than the room left.
fn spacing(gap: f32, scale: f32, radius: f32) -> Spacing {
    let wanted = (LABEL * scale).max(LEAST_LABEL);
    let marks = if radius > 0.0 { radius + MARGIN } else { 0.0 };
    let room =
        |under: f32, above: f32| (gap - under * f32::from(radius > 0.0) - marks - above) / LINE;
    let (under, above) = (UNDER * scale, ABOVE * scale);
    if room(under, above) >= wanted {
        return Spacing {
            under,
            above,
            size: wanted,
        };
    }
    let (under, above) = (under.min(radius + HUG), above.min(HUG));
    Spacing {
        under,
        above,
        size: wanted.min(room(under, above)).max(1.0),
    }
}

/// The canvas pixels between one row's frames and the next's.
fn row_gap() -> f32 {
    let first = Layout::pair_at(0, None).original;
    let below = (1..16)
        .map(|slot| Layout::pair_at(slot, None).original)
        .find(|frame| frame.origin[1] > first.origin[1]);
    below.map_or(0.0, |frame| {
        (frame.origin[1] - first.origin[1] - first.size[1]) as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_never_reaches_the_marks_above_it() {
        let gap = row_gap();
        assert!(gap > 0.0);
        for scale in [0.1, 0.15, 0.2, 0.3, 0.5, 1.0, 2.0] {
            let shown = spacing(
                gap * scale,
                scale,
                (MARK * scale).clamp(LEAST_MARK, MOST_MARK),
            );
            let radius = (MARK * scale).clamp(LEAST_MARK, MOST_MARK);
            let used = shown.under + radius + MARGIN + shown.size * LINE + shown.above;
            assert!(used <= gap * scale + 0.01, "{scale}: {shown:?}");
        }
        let close = spacing(gap, 1.0, MARK);
        assert_eq!(
            (close.under, close.size),
            (UNDER, LABEL),
            "as drawn close up"
        );
    }
}
