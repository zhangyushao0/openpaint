//! The brush editor's dynamics pages (S46): Size dynamics and Opacity dynamics each show three
//! curve squares stacked, over pressure, speed and tilt, each under its name with Reset (the
//! curve as the built-in's file has it; grey while it is) and over the ends of what it follows.
//! The square spans what the curve follows from 0 to its largest (1, 50 px/ms, 90 degrees) and
//! its factors up to the curve's top (4 for a size over tilt, else 1); a size curve's points
//! stop where the brush's stamps would no longer fit.

use egui::{Align, Align2, Layout, TextStyle, Vec2};
use openpaint_app::{Axis, BrushChange, BrushIntent, Dynamic, EditedBrush, Intent, curve_top};

use crate::brush_settings::push_once;
use crate::components::{DynamicsCurve, dynamics_editor, text_button};
use crate::design::{INK, INK_SECONDARY, SPACE, TOUCH_TARGET};

/// A curve square's side, in points.
const SQUARE: f32 = 28.0 * SPACE;

/// The page's heading.
pub(crate) const fn heading(dynamic: Dynamic) -> &'static str {
    match dynamic {
        Dynamic::Size => "Size dynamics",
        Dynamic::Opacity => "Opacity dynamics",
    }
}

/// What the ends of an axis read, from its start to its largest.
const fn ends(axis: Axis) -> [&'static str; 2] {
    match axis {
        Axis::Pressure => ["Light", "Firm"],
        Axis::Speed => ["Slow", "50 px/ms"],
        Axis::Tilt => ["Upright", "Flat"],
    }
}

/// The three curves of `dynamic`; their changes pushed to `taps`.
pub(crate) fn curves(
    ui: &mut egui::Ui,
    dynamic: Dynamic,
    edited: &EditedBrush,
    taps: &mut Vec<Intent>,
) {
    for axis in Axis::ALL {
        title(ui, edited, (dynamic, axis), taps);
        let top = curve_top(dynamic, axis);
        let most = axis.most();
        let curve = DynamicsCurve {
            points: edited
                .curve(dynamic, axis)
                .iter()
                .map(|[x, y]| [x / most, y / top])
                .collect(),
            limit: edited.curve_limit(dynamic, axis) / top,
            one: 1.0 / top,
        };
        let changed = ui
            .horizontal(|ui| {
                ui.add_space(((ui.available_width() - SQUARE) / 2.0).max(0.0));
                dynamics_editor(ui, (axis.label(), SQUARE), &curve)
            })
            .inner;
        if let Some((shaped, settled)) = changed {
            let points = shaped
                .points
                .iter()
                .map(|[x, y]| [x * most, y * top])
                .collect();
            let change = BrushChange::Curve {
                dynamic,
                axis,
                points,
            };
            push_once(taps, Intent::Brush(BrushIntent::Edit { change, settled }));
        }
        axis_ends(ui, axis, top);
    }
}

/// The axis's name at the left and Reset at the right.
fn title(
    ui: &mut egui::Ui,
    edited: &EditedBrush,
    (dynamic, axis): (Dynamic, Axis),
    taps: &mut Vec<Intent>,
) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.colored_label(INK, axis.label());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(SPACE);
            let changed = edited.curve_changed(dynamic, axis);
            if text_button(ui, "Reset", changed).clicked() {
                let change = BrushChange::ResetCurve { dynamic, axis };
                let settled = true;
                push_once(taps, Intent::Brush(BrushIntent::Edit { change, settled }));
            }
        });
    });
}

/// Under the square: where its axis starts and ends, and how far a factor may rise.
fn axis_ends(ui: &mut egui::Ui, axis: Axis, top: f32) {
    let size = Vec2::new(ui.available_width(), 3.0 * SPACE);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let inset = (rect.width() - SQUARE) / 2.0 + TOUCH_TARGET / 4.0;
    let font = TextStyle::Small.resolve(ui.style());
    let painter = ui.painter();
    let [start, end] = ends(axis);
    let left = rect.left_top() + Vec2::new(inset, 0.0);
    painter.text(left, Align2::LEFT_TOP, start, font.clone(), INK_SECONDARY);
    let right = rect.right_top() - Vec2::new(inset, 0.0);
    painter.text(right, Align2::RIGHT_TOP, end, font.clone(), INK_SECONDARY);
    if top > 1.0 {
        let middle = rect.center_top();
        let up_to = format!("Up to {top:.0}×");
        painter.text(middle, Align2::CENTER_TOP, up_to, font, INK_SECONDARY);
    }
}
