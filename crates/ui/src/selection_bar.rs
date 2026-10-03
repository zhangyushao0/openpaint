//! The Selection tool's chrome (Procreate's Selection, with draweave's behaviour): its button
//! in the left island after Actions, carrying the ink ring while the tool is on; while it is on,
//! the selection bar in the middle of the top band: Freehand and Rectangle (the one in use
//! outlined in deeper ink), Close while a freehand path is open, Invert, and Deselect (greyed
//! with nothing selected). Over the canvas, the path being drawn: an ink line on a paper line,
//! its first point a dot, ringed once the path can close there. What is selected is marked by
//! the screen's blit (an outline and stripes outside), not here.

use egui::{Color32, Order, Pos2, Stroke};
use openpaint_app::{Intent, SelectionIntent, SelectionShape, SessionProjection, ViewMapping};

use crate::components::{Place, ScreenArea, icon_toggle, pill, ring_of, text_button, text_choice};
use crate::design::{INK, Icon, PANEL, SPACE, ink_line};

/// How near the path's first point, in points, a pen contact closes it (draweave's).
pub(crate) const CLOSE_POINTS: f32 = 14.0;
/// The path's ink line and the paper line under it, in points.
const PATH_LINE: f32 = 1.5;
const PATH_UNDER: f32 = 3.5;

#[derive(Debug, Default)]
pub(crate) struct SelectionBar {
    /// Where the bar lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the Selection button lies, in points: the bar comes out of it.
    button: Option<egui::Rect>,
}

impl SelectionBar {
    /// The Selection button. Returns whether a tap turned the tool on.
    pub(crate) fn button(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let on = projection.selection().tool;
        let response = icon_toggle(ui, Icon::Selection, "Selection", on);
        self.button = Some(response.rect);
        if response.clicked() {
            push(taps, Intent::Selection(SelectionIntent::Tool(!on)));
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
        let selection = projection.selection();
        if !selection.tool {
            self.area = None;
            return;
        }
        let source = ring_of(self.button.unwrap_or(egui::Rect::ZERO));
        let ((), rect) = pill(ctx, "selection bar", Place::top_middle(), source, |ui| {
            for (shape, label) in [
                (SelectionShape::Freehand, "Freehand"),
                (SelectionShape::Rectangle, "Rectangle"),
            ] {
                if text_choice(ui, label, selection.shape == shape).clicked() {
                    push(taps, Intent::Selection(SelectionIntent::Shape(shape)));
                }
            }
            ui.add_space(SPACE);
            if selection.closable && text_button(ui, "Close", true).clicked() {
                push(taps, Intent::Selection(SelectionIntent::Close));
            }
            if text_button(ui, "Invert", true).clicked() {
                push(taps, Intent::Selection(SelectionIntent::Invert));
            }
            let any = selection.mask.is_some();
            if text_button(ui, "Deselect", any).clicked() {
                push(taps, Intent::Selection(SelectionIntent::Deselect));
            }
        });
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the bar lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }
}

/// The freehand path or the rectangle being drawn, over the canvas and under the chrome,
/// through `mapping` on a screen of `pixels_per_point`.
pub(crate) fn show_path(
    ctx: &egui::Context,
    projection: &SessionProjection,
    mapping: &ViewMapping,
    pixels_per_point: f32,
) {
    let selection = projection.selection();
    if !selection.tool || selection.path.is_empty() {
        return;
    }
    let at = |point: [f32; 2]| {
        let [x, y] = mapping.to_screen(point);
        Pos2::new(x / pixels_per_point, y / pixels_per_point)
    };
    let points: Vec<Pos2> = match (selection.shape, &selection.path[..]) {
        (SelectionShape::Rectangle, [a, b]) => {
            [*a, [b[0], a[1]], *b, [a[0], b[1]], *a].map(at).to_vec()
        }
        (_, path) => path.iter().copied().map(at).collect(),
    };
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("selection path"));
    let painter = ctx.layer_painter(layer);
    ink_line(&painter, &points, Stroke::new(PATH_UNDER, PANEL));
    ink_line(&painter, &points, Stroke::new(PATH_LINE, INK));
    if selection.shape == SelectionShape::Freehand {
        let first = points[0];
        painter.circle(first, 4.0, INK, Stroke::new(1.5, PANEL));
        if selection.closable {
            let ring = Stroke::new(PATH_LINE, INK);
            painter.circle(first, CLOSE_POINTS * 0.7, Color32::TRANSPARENT, ring);
        }
    }
}

/// egui may run a pass twice; one tap is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
