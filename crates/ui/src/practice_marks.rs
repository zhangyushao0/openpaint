//! A practice sheet's exercises over its canvas (G9): each pair revealed so far as two frames,
//! the original's and the empty one for the copy, the shown pair's in ink and the earlier ones
//! faint; each original in its frame (a line or a group of lines in ink, a swatch filling the
//! frame). While compared, the original's lines lie over the copy, in place or aligned onto it,
//! thick and clear in a semi-transparent marking red the copy shows through; a colour shows as a
//! patch inside the other. All of it is drawn by the screen over the canvas through the view,
//! never into the artwork: the originals cannot be drawn on.

use egui::{Color32, Order, Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection, SessionProjection, ViewMapping};
use openpaint_practice::{Frame, Original, Rgb8};

use crate::components::canvas_point;
use crate::design::{CORRECTION, DIVIDER, INK, PANEL_RULE_WIDTH};

/// An original line's width, in canvas pixels, and the least it shows at, in points.
const LINE_WIDTH: f32 = 3.0;
const LEAST_LINE: f32 = 1.25;
/// The overlay's width, in canvas pixels (a frame is 400) and at least in points, and how
/// strongly it lies over the copy.
const OVERLAY_WIDTH: f32 = 12.0;
const LEAST_OVERLAY: f32 = 4.0;
const OVERLAY_SHOWN: f32 = 0.5;

/// The sheet of `projection`'s practice, if it is one, through `mapping` on a screen of
/// `pixels_per_point`.
pub(crate) fn show_marks(
    ctx: &egui::Context,
    projection: &SessionProjection,
    mapping: &ViewMapping,
    pixels_per_point: f32,
) {
    let Some(practice) = projection.practice() else {
        return;
    };
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("practice sheet"));
    let painter = ctx.layer_painter(layer);
    let screen = ctx.content_rect();
    let at = |point: [f64; 2]| canvas_point(mapping, pixels_per_point, point.map(|v| v as f32));
    let line_width = (LINE_WIDTH * mapping.scale() / pixels_per_point).max(LEAST_LINE);
    let sheet = &practice.sheet;
    for (index, pair) in sheet.pairs.iter().enumerate().take(practice.revealed) {
        let corners = |frame: &Frame| {
            let [x, y] = frame.origin;
            let side = frame.side;
            [[x, y], [x + side, y], [x + side, y + side], [x, y + side]].map(at)
        };
        let [original, copy] = [corners(&pair.original), corners(&pair.copy)];
        let shown = egui::Rect::from_points(&original).union(egui::Rect::from_points(&copy));
        if !shown.intersects(screen) {
            continue;
        }
        let frame = if practice.current == Some(index) {
            Stroke::new(PANEL_RULE_WIDTH, INK)
        } else {
            Stroke::new(1.0, DIVIDER)
        };
        match &sheet.originals[index] {
            Original::Lines(lines) => {
                for line in lines {
                    let points = (line.iter()).map(|p| at(pair.original.to_canvas(*p)));
                    painter.add(Shape::line(points.collect(), Stroke::new(line_width, INK)));
                }
            }
            Original::Colour(rgb) => {
                painter.add(Shape::convex_polygon(
                    original.to_vec(),
                    colour(*rgb),
                    Stroke::NONE,
                ));
            }
        }
        painter.add(Shape::closed_line(original.to_vec(), frame));
        painter.add(Shape::closed_line(copy.to_vec(), frame));
    }
    if let Some(overlay) = &practice.overlay {
        let width = (OVERLAY_WIDTH * mapping.scale() / pixels_per_point).max(LEAST_OVERLAY);
        show_overlay(&painter, practice, overlay, (&at, width));
    }
}

/// The shown exercise's comparison over its copy; a line's `width` points wide.
fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
) {
    let Some(index) = practice.current else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Lines(lines), Original::Lines(_)) => {
            let red = CORRECTION.gamma_multiply(OVERLAY_SHOWN);
            for line in lines {
                let points: Vec<Pos2> = (line.iter()).map(|p| at(p.map(f64::from))).collect();
                painter.add(Shape::line(points, Stroke::new(width, red)));
            }
        }
        (Overlay::Colour { original, last, .. }, Original::Colour(_)) => {
            // Each colour as a patch in the middle of the other's frame.
            let patch = |frame: &Frame| {
                let [x, y] = frame.origin;
                let (inset, side) = (frame.side / 4.0, frame.side / 2.0);
                let [x, y] = [x + inset, y + inset];
                vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side]]
                    .into_iter()
                    .map(at)
                    .collect::<Vec<Pos2>>()
            };
            painter.add(Shape::convex_polygon(
                patch(&pair.copy),
                colour(*original),
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                patch(&pair.original),
                colour(*last),
                Stroke::NONE,
            ));
        }
        _ => {}
    }
}

fn colour([r, g, b]: Rgb8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
