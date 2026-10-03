//! The Transform tool's chrome (Procreate's Transform, with draweave's behaviour): its button in
//! the left island after Selection, carrying the ink ring while the tool is on; while it is on,
//! the transform bar in the middle of the top band: Flip Horizontal, Flip Vertical, Rotate 45°
//! and Reset (greyed while nothing moved). Over the canvas, the frame around the content: an
//! ink line on a paper line, a handle at each corner (scale) and one above the top edge on a
//! short stem (turn); it appears with the acknowledge pattern. The content itself is the
//! canvas's (the renderer shows the preview), so it follows the hand with no motion of its own.

use egui::{Order, Pos2, Stroke};
use openpaint_app::{Intent, SessionProjection, TransformFrame, TransformIntent, ViewMapping};

use crate::components::{Place, ScreenArea, animate_from, icon_toggle, pill, ring_of, text_button};
use crate::design::motion::ACKNOWLEDGE;
use crate::design::{INK, Icon, PANEL, ink_line};

/// Points from the frame's top edge to the turn handle (draweave's).
const STEM: f32 = 36.0;
/// Radius of a handle's mark and of the area that takes it, in points: a 48-point target.
const HANDLE: f32 = 7.0;
pub(crate) const HANDLE_REACH: f32 = 24.0;
/// The frame's ink line and the paper line under it, in points.
const LINE: f32 = 1.5;
const UNDER: f32 = 3.5;

/// The frame's handles on screen, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Handles {
    /// Top left, top right, bottom right, bottom left.
    pub(crate) corners: [[f32; 2]; 4],
    pub(crate) turn: [f32; 2],
    /// Where the turn handle's stem leaves the top edge.
    pub(crate) stem: [f32; 2],
}

impl Handles {
    /// Where `frame`'s handles show through `mapping` on a screen of `pixels_per_point`.
    pub(crate) fn of(frame: &TransformFrame, mapping: &ViewMapping, pixels_per_point: f32) -> Self {
        let corners = frame.corners().map(|corner| mapping.to_screen(corner));
        let centre = mapping.to_screen(frame.centre());
        let stem = [
            (corners[0][0] + corners[1][0]) / 2.0,
            (corners[0][1] + corners[1][1]) / 2.0,
        ];
        let out = [stem[0] - centre[0], stem[1] - centre[1]];
        let length = out[0].hypot(out[1]);
        // A frame too thin to tell its top from its middle turns from straight above.
        let out = if length > 1.0 {
            [out[0] / length, out[1] / length]
        } else {
            [0.0, -1.0]
        };
        let reach = STEM * pixels_per_point;
        Self {
            corners,
            turn: [stem[0] + out[0] * reach, stem[1] + out[1] * reach],
            stem,
        }
    }

    /// True when the screen point `at` lies inside the frame.
    pub(crate) fn inside(&self, at: [f32; 2]) -> bool {
        let mut sign = 0.0f32;
        for index in 0..4 {
            let [a, b] = [self.corners[index], self.corners[(index + 1) % 4]];
            let cross = (b[0] - a[0]) * (at[1] - a[1]) - (b[1] - a[1]) * (at[0] - a[0]);
            if cross.abs() <= f32::EPSILON {
                continue;
            }
            if sign == 0.0 {
                sign = cross.signum();
            } else if sign != cross.signum() {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Default)]
pub(crate) struct TransformBar {
    /// Where the bar lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the Transform button lies, in points: the bar comes out of it.
    button: Option<egui::Rect>,
    /// Counts the frames that appeared, so each one fades in from nothing.
    appeared: u64,
    framed: bool,
}

impl TransformBar {
    /// The Transform button. Returns whether a tap turned the tool on.
    pub(crate) fn button(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let on = projection.transform().tool;
        let response = icon_toggle(ui, Icon::Transform, "Transform", on);
        self.button = Some(response.rect);
        if response.clicked() {
            push(taps, Intent::Transform(TransformIntent::Tool(!on)));
            return !on;
        }
        false
    }

    /// The bar while the tool is on, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let transform = projection.transform();
        if !transform.tool {
            self.area = None;
            return;
        }
        let source = ring_of(self.button.unwrap_or(egui::Rect::ZERO));
        let framed = transform.frame.is_some();
        let ((), rect) = pill(ctx, "transform bar", Place::top_middle(), source, |ui| {
            let taps_for = [
                (
                    "Flip Horizontal",
                    TransformIntent::Flip { horizontal: true },
                    framed,
                ),
                (
                    "Flip Vertical",
                    TransformIntent::Flip { horizontal: false },
                    framed,
                ),
                ("Rotate 45°", TransformIntent::Rotate, framed),
                ("Reset", TransformIntent::Reset, transform.placed),
            ];
            for (label, intent, enabled) in taps_for {
                if text_button(ui, label, enabled).clicked() {
                    push(taps, Intent::Transform(intent));
                }
            }
        });
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the bar lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    /// The frame and its handles over the canvas, under the chrome, through `mapping` on a
    /// screen of `pixels_per_point`; a new frame fades in (acknowledge).
    pub(crate) fn show_frame(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        mapping: &ViewMapping,
        pixels_per_point: f32,
    ) {
        let Some(frame) = projection.transform().frame else {
            self.framed = false;
            return;
        };
        if !self.framed {
            self.framed = true;
            self.appeared += 1;
        }
        let id = egui::Id::new(("transform frame", self.appeared));
        let shown = animate_from(ctx, id, 0.0, 1.0, ACKNOWLEDGE);
        let handles = Handles::of(&frame, mapping, pixels_per_point);
        let at = |[x, y]: [f32; 2]| Pos2::new(x / pixels_per_point, y / pixels_per_point);
        let layer = egui::LayerId::new(Order::Background, egui::Id::new("transform frame"));
        let painter = ctx.layer_painter(layer);
        let (ink, paper) = (INK.gamma_multiply(shown), PANEL.gamma_multiply(shown));
        let mut outline: Vec<Pos2> = handles.corners.iter().copied().map(at).collect();
        outline.push(outline[0]);
        let stem = [at(handles.stem), at(handles.turn)];
        for (width, color) in [(UNDER, paper), (LINE, ink)] {
            ink_line(&painter, &outline, Stroke::new(width, color));
            ink_line(&painter, &stem, Stroke::new(width, color));
        }
        for corner in handles.corners {
            painter.circle(at(corner), HANDLE, paper, Stroke::new(LINE, ink));
        }
        painter.circle(at(handles.turn), HANDLE, ink, Stroke::new(LINE, paper));
    }
}

/// egui may run a pass twice; one tap is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_app::{CanvasView, Viewport};
    use openpaint_core::{CanvasSize, Placement};

    #[test]
    fn the_turn_handle_stands_above_the_top_edge_and_the_frame_knows_its_inside() {
        let canvas = CanvasSize::new(400, 300).expect("size");
        let viewport = Viewport {
            origin: [0.0, 0.0],
            size: [400.0 / 0.92, 300.0 / 0.92],
            pixels_per_point: 2.0,
        };
        let mapping = CanvasView::FIT.mapping(canvas, &viewport);
        let frame = TransformFrame {
            origin: [100.0, 100.0],
            size: [100.0, 50.0],
            placement: Placement::IDENTITY,
        };
        let handles = Handles::of(&frame, &mapping, 2.0);
        let [x, y] = handles.turn;
        let top = mapping.to_screen([150.0, 100.0]);
        assert!((x - top[0]).abs() < 0.01 && (y - (top[1] - 72.0)).abs() < 0.01);
        assert!(handles.inside(mapping.to_screen([150.0, 125.0])));
        assert!(!handles.inside(mapping.to_screen([250.0, 125.0])));
    }
}
