//! The curve editor (Procreate's Curves): a square with a faint grid, the line the curve starts
//! as, the curve in ink and its points as small rings. Pressing a point takes it; pressing on
//! the curve away from the points adds one there and takes it; the point taken follows the hand
//! until it lifts, which settles the curve. A point's hit area is a 48-point circle. It edits a
//! tone curve from black (bottom left) to white (top right), whose ends move only up and down,
//! and since S46 a brush's dynamics curve (`DynamicsCurve`): points joined by straight lines,
//! flat beyond the first and the last, the square spanning what the curve follows (pressure,
//! speed or tilt) and its factors up to its top.

use egui::{Pos2, Rect, Sense, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_core::{MAX_CURVE_POINTS, ToneCurve};

use crate::components::controls;
use crate::design::{HAIRLINE, INK, INK_DISABLED, PANEL, TOUCH_TARGET, ink_line};

/// Radius of a point's ring, in points.
const POINT: f32 = 6.0;
/// The ink line of the curve, in points.
const LINE: f32 = 2.0;
/// Steps the curve is drawn in.
const STEPS: usize = 96;
/// Least distance between two points of a dynamics curve along the square, as a share of it.
const GAP: f32 = 0.01;

/// A tone curve the hand set this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveChange {
    pub curve: ToneCurve,
    /// True when the hand lifted: the last curve of this touch.
    pub settled: bool,
}

/// A curve the square edits, in the square's units: x and y from 0 to 1.
trait Edited: Clone + PartialEq {
    fn points(&self) -> Vec<[f32; 2]>;
    fn value(&self, x: f32) -> f32;
    /// The curve with a point added at `x` on it, and that point's index.
    fn with_point(&self, x: f32) -> Option<(Self, usize)>;
    /// The curve with point `index` moved towards `to`, held where it may go.
    fn moved(&self, index: usize, to: [f32; 2]) -> Option<Self>;
    /// The faint line the curve starts as.
    fn baseline(&self) -> [[f32; 2]; 2];
}

impl Edited for ToneCurve {
    fn points(&self) -> Vec<[f32; 2]> {
        ToneCurve::points(self).to_vec()
    }

    fn value(&self, x: f32) -> f32 {
        ToneCurve::value(self, x)
    }

    fn with_point(&self, x: f32) -> Option<(Self, usize)> {
        ToneCurve::with_point(self, x)
    }

    fn moved(&self, index: usize, to: [f32; 2]) -> Option<Self> {
        ToneCurve::moved(self, index, to)
    }

    fn baseline(&self) -> [[f32; 2]; 2] {
        [[0.0, 0.0], [1.0, 1.0]]
    }
}

/// A brush's dynamics curve in the square's units: its points as shares of what it follows
/// and of its top factor, and how high its points may go (`limit`, at most 1) and where a factor
/// of 1 lies (`one`): the faint line, the curve that changes nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicsCurve {
    pub points: Vec<[f32; 2]>,
    pub limit: f32,
    pub one: f32,
}

impl Edited for DynamicsCurve {
    fn points(&self) -> Vec<[f32; 2]> {
        self.points.clone()
    }

    fn value(&self, x: f32) -> f32 {
        let [first, rest @ ..] = self.points.as_slice() else {
            return self.one;
        };
        let mut from = *first;
        if x <= from[0] {
            return from[1];
        }
        for to in rest {
            if x <= to[0] {
                let t = (x - from[0]) / (to[0] - from[0]).max(f32::MIN_POSITIVE);
                return from[1] + (to[1] - from[1]) * t;
            }
            from = *to;
        }
        from[1]
    }

    fn with_point(&self, x: f32) -> Option<(Self, usize)> {
        let at = self
            .points
            .iter()
            .position(|[px, _]| *px > x)
            .unwrap_or(self.points.len());
        let apart = self.points.iter().all(|[px, _]| (px - x).abs() >= GAP);
        if self.points.len() >= MAX_CURVE_POINTS || !apart {
            return None;
        }
        let mut points = self.points.clone();
        points.insert(at, [x, self.value(x)]);
        Some((
            Self {
                points,
                ..self.clone()
            },
            at,
        ))
    }

    fn moved(&self, index: usize, [x, y]: [f32; 2]) -> Option<Self> {
        let low = if index == 0 {
            0.0
        } else {
            self.points[index - 1][0] + GAP
        };
        let high = self.points.get(index + 1).map_or(1.0, |next| next[0] - GAP);
        if low > high {
            return None;
        }
        let mut points = self.points.clone();
        *points.get_mut(index)? = [x.clamp(low, high), y.clamp(0.0, self.limit)];
        Some(Self {
            points,
            ..self.clone()
        })
    }

    fn baseline(&self) -> [[f32; 2]; 2] {
        [[0.0, self.one], [1.0, self.one]]
    }
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
    fn take<C: Edited>(&self, curve: &C, at: Pos2) -> Option<(C, usize)> {
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
    edit(ui, (label, side), curve).map(|(curve, settled)| CurveChange { curve, settled })
}

/// Shows a dynamics `curve` in a square of `side` points, named `label`; returns the curve the
/// hand set this frame and whether the hand lifted.
pub fn dynamics_editor(
    ui: &mut egui::Ui,
    (label, side): (&str, f32),
    curve: &DynamicsCurve,
) -> Option<(DynamicsCurve, bool)> {
    edit(ui, (label, side), curve)
}

fn edit<C: Edited>(ui: &mut egui::Ui, (label, side): (&str, f32), curve: &C) -> Option<(C, bool)> {
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
    (taken.is_some() && (settled || shown != *curve)).then_some((shown, settled))
}

fn paint<C: Edited>(ui: &egui::Ui, square: &Square, curve: &C) {
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
    let [from, to] = curve.baseline();
    painter.line_segment(
        [square.at(from), square.at(to)],
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
        painter.circle(square.at(point), POINT, PANEL, Stroke::new(LINE, INK));
    }
}
