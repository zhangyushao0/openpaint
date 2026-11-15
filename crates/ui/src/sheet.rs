//! What the screen shows around the canvas itself: the canvas's soft shadow on the desk, which
//! screen is showing, and the view status while and briefly after the fingers move the view.

use openpaint_app::{ScreenProjection, ViewMapping};
use openpaint_core::CanvasSize;

use crate::components::{CanvasArea, canvas_point, view_status, view_text};
use crate::design::paint_sheet_shadow;
use crate::shell::{Shell, Shown};

/// The view status shows this long after the view stopped, then fades for `STATUS_FADE_MS`.
pub(crate) const STATUS_HOLD_MS: i64 = 700;
pub(crate) const STATUS_FADE_MS: i64 = 300;

/// Which screen the last pass showed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Showing {
    #[default]
    Starting,
    Canvas,
    Gallery,
}

impl Showing {
    pub(crate) const fn of(projection: &ScreenProjection) -> Self {
        match projection {
            ScreenProjection::Starting => Self::Starting,
            ScreenProjection::Canvas(_) => Self::Canvas,
            ScreenProjection::Gallery(_) => Self::Gallery,
        }
    }
}

/// The canvas's soft shadow on the desk where `mapping` shows it, under everything else and
/// outside the canvas, so the white sheet stands apart from the paper chrome however the view
/// turns. Returns the rectangle (points) the canvas shows in.
pub(crate) fn show_sheet(
    ctx: &egui::Context,
    rect: egui::Rect,
    shown: Option<Shown>,
    mapping: &ViewMapping,
    pixels_per_point: f32,
) -> Option<egui::Rect> {
    let shown = shown?;
    let [w, h] = [shown.canvas.width(), shown.canvas.height()].map(|side| side as f32);
    let corners = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]
        .map(|corner| canvas_point(mapping, pixels_per_point, corner));
    let layer = egui::LayerId::new(egui::Order::Background, egui::Id::new("canvas sheet"));
    let painter = ctx.layer_painter(layer).with_clip_rect(rect);
    paint_sheet_shadow(&painter, corners);
    Some(egui::Rect::from_points(&corners))
}

/// Where the canvas leaves the screen from, given the rectangle it showed in (`whole`, points):
/// an artwork whole; a mode's sheet, seen a part at a time on paper laid ahead, as the part of it
/// `mapping` showed in `rect`, beside its canvas pixels (left, top, width, height), which its
/// picture is made of (S144). None when nothing of it shows.
pub(crate) fn left_from(
    whole: Option<egui::Rect>,
    (rect, shown): (egui::Rect, Option<Shown>),
    mapping: &ViewMapping,
    pixels_per_point: f32,
) -> Option<(egui::Rect, Option<[u32; 4]>)> {
    match shown.filter(|shown| shown.mode) {
        Some(sheet) => shown_part((rect, sheet.canvas), mapping, pixels_per_point),
        None => whole.map(|whole| (whole, None)),
    }
}

/// The part of `canvas` `mapping` shows in `rect` (points): where it lies (points) and its
/// pixels, whole ones within the canvas; none when nothing of it shows.
fn shown_part(
    (rect, canvas): (egui::Rect, CanvasSize),
    mapping: &ViewMapping,
    pixels_per_point: f32,
) -> Option<(egui::Rect, Option<[u32; 4]>)> {
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ]
    .map(|corner| mapping.to_canvas([corner.x, corner.y].map(|v| v * pixels_per_point)));
    // The canvas pixels the screen's corners fall on, whole pixels within the canvas.
    let span = |axis: usize, side: u32| {
        let along = corners.map(|corner| corner[axis]);
        let low = along.iter().copied().fold(f32::INFINITY, f32::min);
        let high = along.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let within = |value: f32| value.clamp(0.0, side as f32) as u32;
        (within(low.floor()), within(high.ceil()))
    };
    let (left, right) = span(0, canvas.width());
    let (top, bottom) = span(1, canvas.height());
    if right <= left || bottom <= top {
        return None;
    }
    let part = [[left, top], [right, top], [right, bottom], [left, bottom]]
        .map(|corner| canvas_point(mapping, pixels_per_point, corner.map(|v| v as f32)));
    let pixels = [left, top, right - left, bottom - top];
    Some((egui::Rect::from_points(&part), Some(pixels)))
}

/// The view status while the fingers move the view and briefly after.
pub(crate) fn show_view(shell: &Shell, ctx: &egui::Context, area: &CanvasArea) {
    let (Some(shown), Some(moved)) = (shell.shown(), shell.input.navigated()) else {
        return;
    };
    let since = shell.now_ms.saturating_sub(moved);
    let fading = since.saturating_sub(STATUS_HOLD_MS).max(0);
    let strength = 1.0 - fading as f32 / STATUS_FADE_MS as f32;
    if strength <= 0.0 {
        return;
    }
    let text = view_text(shell.scale(area), shown.view.rotation());
    let taken = shell.drop.bar().is_some() || shown.selecting || shown.mode;
    view_status(ctx, &text, strength, taken);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_leaves_from_the_part_of_it_the_screen_showed() {
        let canvas = CanvasSize::new(1000, 3000).expect("a tall sheet");
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 600.0));
        // Twice its size, its top left half a canvas width left of the screen and a third up.
        let zoomed = ViewMapping::placed(canvas, [-500.0, -1000.0], [2000.0, 6000.0]);
        let (at, part) = shown_part((screen, canvas), &zoomed, 2.0).expect("a part shows");
        assert_eq!(part, Some([250, 500, 750, 600]));
        let expected = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(750.0, 600.0));
        assert!((at.min - expected.min).length() < 0.01 && (at.max - expected.max).length() < 0.01);
        let away = ViewMapping::placed(canvas, [3000.0, 0.0], [1000.0, 3000.0]);
        assert_eq!(
            shown_part((screen, canvas), &away, 2.0),
            None,
            "nothing of it shows"
        );
    }
}
