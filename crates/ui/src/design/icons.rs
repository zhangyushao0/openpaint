//! The icon set: simple line drawings in one stroke weight, drawn with the painter so they stay
//! sharp at every density. Our own shapes; no icon from another app is copied.

use std::f32::consts::{FRAC_PI_2, PI};

use egui::{Color32, Pos2, Rect, Shape, Stroke};

/// Side of an icon's square, in points.
pub const ICON_SIZE: f32 = 24.0;
/// Line weight of every icon, in points.
const ICON_LINE: f32 = 2.0;
/// Points on the curved part of an arrow.
const ARC_STEPS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// An arrow that turns back to the left.
    Undo,
    /// The undo arrow mirrored: it turns back to the right.
    Redo,
    /// An exclamation mark in a circle: something needs the user's attention.
    Alert,
}

/// Draws `icon` into `rect` (square, `ICON_SIZE` on a side) in `color`.
pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    if icon == Icon::Alert {
        paint_alert(painter, rect, color);
        return;
    }
    let mirrored = icon == Icon::Redo;
    let at = |[x, y]: [f32; 2]| -> Pos2 {
        let x = if mirrored { 1.0 - x } else { x };
        rect.min + egui::vec2(x, y) * rect.size()
    };
    let stroke = Stroke::new(ICON_LINE, color);
    let (shaft, head) = turn_back_arrow();
    painter.add(Shape::line(shaft.into_iter().map(at).collect(), stroke));
    painter.add(Shape::line(head.into_iter().map(at).collect(), stroke));
}

/// A circle with a bar and a dot inside, all in the icon line weight.
fn paint_alert(painter: &egui::Painter, rect: Rect, color: Color32) {
    let at = |x: f32, y: f32| rect.min + egui::vec2(x, y) * rect.size();
    let unit = rect.width();
    let stroke = Stroke::new(ICON_LINE, color);
    painter.circle_stroke(rect.center(), 0.4 * unit, stroke);
    painter.line_segment([at(0.5, 0.27), at(0.5, 0.56)], stroke);
    painter.circle_filled(at(0.5, 0.72), ICON_LINE * 0.75, color);
}

/// An arrow pointing left along the top, turning down and back along the bottom, in a unit
/// square: the shaft from the tip through the turn, and the open head at the tip.
fn turn_back_arrow() -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
    let tip = [0.18, 0.36];
    let (centre, radius) = ([0.6, 0.56], 0.2);
    let mut shaft = vec![tip];
    for step in 0..=ARC_STEPS {
        let angle = -FRAC_PI_2 + PI * step as f32 / ARC_STEPS as f32;
        shaft.push([
            centre[0] + radius * angle.cos(),
            centre[1] + radius * angle.sin(),
        ]);
    }
    shaft.push([0.3, centre[1] + radius]);
    let head = vec![[0.36, 0.18], tip, [0.36, 0.54]];
    (shaft, head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_keeps_a_margin_inside_its_square() {
        let (shaft, head) = turn_back_arrow();
        let inside = |value: f32| (0.1..=0.9).contains(&value);
        for [x, y] in shaft.iter().chain(&head) {
            assert!(inside(*x) && inside(*y), "{x}, {y}");
        }
    }
}
