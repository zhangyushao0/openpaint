//! What the screen shows around the canvas itself: the canvas's soft shadow on the desk, which
//! screen is showing, and the view status while and briefly after the fingers move the view.

use openpaint_app::{ScreenProjection, ViewMapping};

use crate::components::{CanvasArea, view_status, view_text};
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
    let ppp = pixels_per_point;
    let [w, h] = [shown.canvas.width(), shown.canvas.height()].map(|side| side as f32);
    let corners = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]].map(|corner| {
        let [x, y] = mapping.to_screen(corner);
        egui::pos2(x / ppp, y / ppp)
    });
    let layer = egui::LayerId::new(egui::Order::Background, egui::Id::new("canvas sheet"));
    let painter = ctx.layer_painter(layer).with_clip_rect(rect);
    paint_sheet_shadow(&painter, corners);
    Some(egui::Rect::from_points(&corners))
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
    let taken = shell.drop.bar().is_some() || shown.selecting;
    view_status(ctx, &text, strength, taken);
}
