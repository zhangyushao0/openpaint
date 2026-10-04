//! The curve editor (Procreate's Curves): a square from black (bottom left) to white (top
//! right) with a faint grid, the straight line the curve starts as, the curve in ink and its
//! points as small rings. Pressing a point takes it; pressing on the curve away from the points
//! adds one there and takes it; the point taken follows the hand (the ends only up and down)
//! until it lifts, which settles the curve. A point's hit area is a 48-point circle.

use egui::{Pos2, Rect, Sense, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_core::ToneCurve;

use crate::components::controls;
use crate::design::{HAIRLINE, INK, INK_DISABLED, PANEL, TOUCH_TARGET, ink_line};

/// Radius of a point's ring, in points.
const POINT: f32 = 6.0;
/// The ink line of the curve, in points.
const LINE: f32 = 2.0;
/// Steps the curve is drawn in.
const STEPS: usize = 96;

/// A curve the hand set this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveChange {
    pub curve: ToneCurve,
    /// True when the hand lifted: the last curve of this touch.
    pub settled: bool,
}

/// Where in the square a value lies, and the reverse.
struct Square(Rect);

impl Square {
    fn at(&self, [x, y]: [f32; 2]) -> Pos2 {
        let rect = self.0;
        Pos2::new(
            rect.left() + x * rect.width(),
            rect.bottom() - y * rect.height(),
        )
    }

    fn value(&self, at: Pos2) -> [f32; 2] {
        let rect = self.0;
        [
            ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
            ((rect.bottom() - at.y) / rect.height()).clamp(0.0, 1.0),
        ]
    }

    /// The point a press at `at` takes: one of `curve`'s within reach, or a new one where the
    /// press lies on the curve, with the curve holding it.
    fn take(&self, curve: &ToneCurve, at: Pos2) -> Option<(ToneCurve, usize)> {
        let reach = TOUCH_TARGET / 2.0;
        let nearest = curve
            .points()
            .iter()
            .map(|point| self.at(*point).distance(at))
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = nearest.filter(|(_, d)| *d <= reach) {
            return Some((curve.clone(), index));
        }
        let [x, _] = self.value(at);
        let on = self.at([x, curve.value(x)]).distance(at) <= reach;
        on.then(|| curve.with_point(x)).flatten()
    }
}

/// Shows `curve` in a square as wide as the panel, named `label`; returns the curve the hand
/// set, if it set one this frame.
pub fn curve_editor(ui: &mut egui::Ui, label: &str, curve: &ToneCurve) -> Option<CurveChange> {
    let side = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, label));
    controls::register(ui, label, rect);
    let square = Square(rect.shrink(TOUCH_TARGET / 4.0));
    let held = response.is_pointer_button_down_on();
    let settled = response.drag_stopped() || response.clicked();
    let mut taken = ui.data(|data| data.get_temp::<usize>(response.id));
    let mut shown = curve.clone();
    if taken.is_none() && (held || settled) {
        // A tap can press and lift within one frame: then it starts where it lifted.
        let origin = ui
            .input(|input| input.pointer.press_origin())
            .or(response.interact_pointer_pos());
        if let Some((with, index)) = origin.and_then(|at| square.take(curve, at)) {
            shown = with;
            taken = Some(index);
            ui.data_mut(|data| data.insert_temp(response.id, index));
        }
    }
    if settled || !held {
        ui.data_mut(|data| data.remove::<usize>(response.id));
    }
    let moved = taken
        .zip(response.interact_pointer_pos())
        .filter(|_| held || settled)
        .and_then(|(index, at)| shown.moved(index, square.value(at)));
    if let Some(moved) = moved {
        shown = moved;
    }
    if ui.is_rect_visible(rect) {
        paint(ui, &square, &shown);
    }
    (taken.is_some() && (settled || shown != *curve)).then_some(CurveChange {
        curve: shown,
        settled,
    })
}

fn paint(ui: &egui::Ui, square: &Square, curve: &ToneCurve) {
    let painter = ui.painter();
    let faint = Stroke::new(HAIRLINE, INK_DISABLED.gamma_multiply(0.5));
    for quarter in 1..4 {
        let t = quarter as f32 / 4.0;
        painter.line_segment([square.at([t, 0.0]), square.at([t, 1.0])], faint);
        painter.line_segment([square.at([0.0, t]), square.at([1.0, t])], faint);
    }
    painter.rect_stroke(
        square.0,
        0.0,
        Stroke::new(HAIRLINE, INK_DISABLED),
        egui::StrokeKind::Middle,
    );
    painter.line_segment(
        [square.at([0.0, 0.0]), square.at([1.0, 1.0])],
        Stroke::new(HAIRLINE, INK_DISABLED),
    );
    let line: Vec<Pos2> = (0..=STEPS)
        .map(|step| {
            let x = step as f32 / STEPS as f32;
            square.at([x, curve.value(x)])
        })
        .collect();
    ink_line(painter, &line, Stroke::new(LINE, INK));
    for point in curve.points() {
        painter.circle(square.at(*point), POINT, PANEL, Stroke::new(LINE, INK));
    }
}
