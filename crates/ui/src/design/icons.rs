//! The icon set: simple line drawings in one stroke weight, drawn with the painter so they stay
//! sharp at every density. Our own shapes; no icon from another app is copied.

use std::f32::consts::{FRAC_PI_2, PI};

use egui::{Color32, Pos2, Rect, Shape, Stroke};

/// Side of an icon's square, in points.
pub const ICON_SIZE: f32 = 24.0;
/// Line weight of every icon, in points.
const ICON_LINE: f32 = 2.0;
/// Points on the curved part of an arrow or an eye.
const ARC_STEPS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// An arrow that turns back to the left.
    Undo,
    /// The undo arrow mirrored: it turns back to the right.
    Redo,
    /// An exclamation mark in a circle: something needs the user's attention.
    Alert,
    /// Three flat sheets stacked: the layers.
    Layers,
    /// A plus: add one.
    Add,
    /// An arrow pointing up: one place higher.
    Raise,
    /// An arrow pointing down: one place lower.
    Lower,
    /// A bin with a lid: throw away.
    Delete,
    /// An open eye: shown.
    Visible,
    /// The eye struck through: hidden.
    Hidden,
    /// Two sheets, one offset behind the other: make a copy.
    Duplicate,
    /// A brush held at a slant, its tip down to the left: the brushes.
    Brush,
}

/// Draws `icon` into `rect` (square, `ICON_SIZE` on a side) in `color`.
pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let at = |[x, y]: [f32; 2]| -> Pos2 { rect.min + egui::vec2(x, y) * rect.size() };
    let stroke = Stroke::new(ICON_LINE, color);
    for line in polylines(icon) {
        painter.add(Shape::line(line.into_iter().map(at).collect(), stroke));
    }
    match icon {
        Icon::Alert => paint_alert(painter, rect, color),
        // The pupil.
        Icon::Visible | Icon::Hidden => {
            painter.circle_stroke(rect.center(), 0.12 * rect.width(), stroke);
        }
        _ => {}
    }
}

/// The open lines of `icon` in a unit square, y down.
fn polylines(icon: Icon) -> Vec<Vec<[f32; 2]>> {
    let mirror_x = |line: Vec<[f32; 2]>| line.into_iter().map(|[x, y]| [1.0 - x, y]).collect();
    let mirror_y = |line: Vec<[f32; 2]>| line.into_iter().map(|[x, y]| [x, 1.0 - y]).collect();
    let up = || {
        vec![
            vec![[0.5, 0.18], [0.5, 0.82]],
            vec![[0.26, 0.42], [0.5, 0.18], [0.74, 0.42]],
        ]
    };
    match icon {
        Icon::Undo => {
            let (shaft, head) = turn_back_arrow();
            vec![shaft, head]
        }
        Icon::Redo => polylines(Icon::Undo).into_iter().map(mirror_x).collect(),
        Icon::Alert => Vec::new(),
        Icon::Layers => sheets(),
        Icon::Add => vec![vec![[0.5, 0.2], [0.5, 0.8]], vec![[0.2, 0.5], [0.8, 0.5]]],
        Icon::Raise => up(),
        Icon::Lower => up().into_iter().map(mirror_y).collect(),
        Icon::Delete => vec![
            vec![[0.18, 0.28], [0.82, 0.28]],
            vec![[0.4, 0.28], [0.4, 0.17], [0.6, 0.17], [0.6, 0.28]],
            vec![[0.26, 0.28], [0.31, 0.84], [0.69, 0.84], [0.74, 0.28]],
            vec![[0.44, 0.4], [0.44, 0.72]],
            vec![[0.56, 0.4], [0.56, 0.72]],
        ],
        Icon::Visible => vec![eye_outline()],
        Icon::Hidden => vec![eye_outline(), vec![[0.18, 0.82], [0.82, 0.18]]],
        Icon::Duplicate => vec![
            vec![
                [0.36, 0.36],
                [0.82, 0.36],
                [0.82, 0.82],
                [0.36, 0.82],
                [0.36, 0.36],
            ],
            vec![
                [0.64, 0.36],
                [0.64, 0.18],
                [0.18, 0.18],
                [0.18, 0.64],
                [0.36, 0.64],
            ],
        ],
        Icon::Brush => vec![
            // The handle, then the bristles closing to a point.
            vec![[0.84, 0.16], [0.56, 0.44]],
            vec![
                [0.49, 0.41],
                [0.59, 0.51],
                [0.5, 0.68],
                [0.36, 0.78],
                [0.16, 0.84],
                [0.22, 0.64],
                [0.32, 0.5],
                [0.49, 0.41],
            ],
        ],
    }
}

/// Three flat rhombi stacked; of the two below the top one only the front edges show.
fn sheets() -> Vec<Vec<[f32; 2]>> {
    let top = [
        [0.5, 0.2],
        [0.86, 0.38],
        [0.5, 0.56],
        [0.14, 0.38],
        [0.5, 0.2],
    ];
    let mut lines = vec![top.to_vec()];
    for drop in [0.16, 0.32] {
        lines.push(top[1..4].iter().map(|[x, y]| [*x, y + drop]).collect());
    }
    lines
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

/// An almond shape across the square: two arcs meeting at the corners of the eye.
fn eye_outline() -> Vec<[f32; 2]> {
    let mut points = Vec::new();
    for upper in [true, false] {
        for step in 0..=ARC_STEPS {
            let t = step as f32 / ARC_STEPS as f32;
            let x = if upper {
                0.12 + 0.76 * t
            } else {
                0.88 - 0.76 * t
            };
            let lift = 0.26 * (PI * t).sin();
            points.push([x, if upper { 0.5 - lift } else { 0.5 + lift }]);
        }
    }
    points
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
    fn every_icon_keeps_a_margin_inside_its_square() {
        let inside = |value: f32| (0.1..=0.9).contains(&value);
        let icons = [
            Icon::Undo,
            Icon::Redo,
            Icon::Layers,
            Icon::Add,
            Icon::Raise,
            Icon::Lower,
            Icon::Delete,
            Icon::Visible,
            Icon::Hidden,
            Icon::Duplicate,
            Icon::Brush,
        ];
        for icon in icons {
            let lines = polylines(icon);
            assert!(!lines.is_empty(), "{icon:?} draws nothing");
            for [x, y] in lines.iter().flatten() {
                assert!(inside(*x) && inside(*y), "{icon:?}: {x}, {y}");
            }
        }
    }
}
