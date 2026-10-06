//! QuickShape's and the Drawing Guide's chrome (S42). After a stroke snaps to a shape, a pill in
//! the middle of the top band offers Edit shape; tapped, the shape's handles show over the
//! canvas (the Transform tool's handle marks) and the pill says Done; anything else done ends
//! it, and the pill goes. While the Drawing Guide is on, its lines show faintly over the canvas
//! through the guide's centre (upright, level, both, or four 45 degrees apart), with a faint
//! handle at the centre that a finger moves. Lines and handles are chrome, never artwork.

use egui::{Order, Pos2, Stroke};
use openpaint_app::{Guide, Intent, SessionProjection, StrokeIntent, Symmetry, ViewMapping};
use openpaint_core::CanvasSize;

use crate::components::{Place, ScreenArea, Source, UNDER_ISLANDS, pill, text_button};
use crate::design::{INK_SECONDARY, PANEL};
use crate::transform_bar::handle_mark;

/// The guide's line and the paper line under it, in points, and how strongly they show.
const GUIDE_LINE: f32 = 1.0;
const GUIDE_UNDER: f32 = 2.5;
const GUIDE_SHOWN: f32 = 0.55;

#[derive(Debug, Default)]
pub(crate) struct ShapeBar {
    /// Where the pill lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl ShapeBar {
    /// The pill while a shape just snapped can be edited, its tap pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        let Some(shape) = projection.shape() else {
            return;
        };
        let mut place = Place::top_middle();
        if projection.selection().tool || projection.transform().tool {
            place.offset.y = UNDER_ISLANDS;
        }
        let source = Source::Within(egui::Align::Center);
        let label = if shape.editing { "Done" } else { "Edit shape" };
        let (tapped, rect) = pill(ctx, "shape bar", place, source, |ui| {
            text_button(ui, label, true).clicked()
        });
        if tapped {
            let intent = Intent::Stroke(StrokeIntent::EditShape(!shape.editing));
            if taps.last() != Some(&intent) {
                taps.push(intent);
            }
        }
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the pill lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }
}

/// The guide's lines and centre, then the shape's handles while it is edited, over the canvas
/// through `mapping` on a screen of `pixels_per_point`.
pub(crate) fn show_marks(
    ctx: &egui::Context,
    projection: &SessionProjection,
    mapping: &ViewMapping,
    pixels_per_point: f32,
) {
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("shape marks"));
    let painter = ctx.layer_painter(layer);
    let at = |point: [f32; 2]| {
        let [x, y] = mapping.to_screen(point);
        Pos2::new(x / pixels_per_point, y / pixels_per_point)
    };
    let guide = projection.guide();
    if let (true, Some(canvas)) = (guide.on, projection.canvas()) {
        let (ink, paper) = (
            INK_SECONDARY.gamma_multiply(GUIDE_SHOWN),
            PANEL.gamma_multiply(GUIDE_SHOWN),
        );
        for [from, to] in guide_lines(&guide, canvas) {
            let line = [at(from), at(to)];
            painter.line_segment(line, Stroke::new(GUIDE_UNDER, paper));
            painter.line_segment(line, Stroke::new(GUIDE_LINE, ink));
        }
        handle_mark(&painter, at(guide.centre), GUIDE_SHOWN);
    }
    if let Some(shape) = projection.shape().filter(|shape| shape.editing) {
        for handle in &shape.handles {
            handle_mark(&painter, at(*handle), 1.0);
        }
    }
}

/// The guide's lines through its centre, cut to the canvas, in canvas pixels.
pub(crate) fn guide_lines(guide: &Guide, canvas: CanvasSize) -> Vec<[[f32; 2]; 2]> {
    let degrees: &[f32] = match guide.symmetry {
        Symmetry::Vertical => &[90.0],
        Symmetry::Horizontal => &[0.0],
        Symmetry::Quadrant => &[0.0, 90.0],
        Symmetry::Radial => &[0.0, 45.0, 90.0, 135.0],
    };
    let size = [canvas.width() as f32, canvas.height() as f32];
    degrees
        .iter()
        .filter_map(|angle| {
            let (sin, cos) = angle.to_radians().sin_cos();
            across(guide.centre, [cos, sin], size)
        })
        .collect()
}

/// The stretch of the line through `centre` along `direction` inside `[0, size]`.
fn across(centre: [f32; 2], direction: [f32; 2], size: [f32; 2]) -> Option<[[f32; 2]; 2]> {
    let (mut low, mut high) = (f32::MIN, f32::MAX);
    for axis in 0..2 {
        let d = direction[axis];
        if d.abs() < 1e-6 {
            if !(0.0..=size[axis]).contains(&centre[axis]) {
                return None;
            }
            continue;
        }
        let (a, b) = ((0.0 - centre[axis]) / d, (size[axis] - centre[axis]) / d);
        low = low.max(a.min(b));
        high = high.min(a.max(b));
    }
    let point = |t: f32| [centre[0] + direction[0] * t, centre[1] + direction[1] * t];
    (low < high).then(|| [point(low), point(high)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_lines_cross_the_canvas_through_the_centre() {
        let canvas = CanvasSize::new(400, 200).expect("size");
        let guide = |symmetry| Guide {
            on: true,
            symmetry,
            centre: [100.0, 100.0],
        };
        let vertical = guide_lines(&guide(Symmetry::Vertical), canvas);
        assert_eq!(vertical.len(), 1);
        let [a, b] = vertical[0];
        assert!((a[0] - 100.0).abs() < 1e-3 && (b[0] - 100.0).abs() < 1e-3);
        assert!((a[1] - b[1]).abs() > 199.9, "top to bottom");
        let radial = guide_lines(&guide(Symmetry::Radial), canvas);
        assert_eq!(radial.len(), 4);
        // The 45 degree line meets the canvas's corner (0, 0) and the bottom edge at (200, 200).
        let [a, b] = radial[1];
        let ends = [a, b].map(|p| [p[0].round(), p[1].round()]);
        assert!(
            ends.contains(&[0.0, 0.0]) && ends.contains(&[200.0, 200.0]),
            "{ends:?}"
        );
    }
}
