//! The icon set: ink-line drawings in one stroke weight with round joins and ends, drawn with
//! the painter so they stay sharp at every density. Curves are curves, not corners, so the set
//! reads as drawn by one pen. Our own shapes; no icon from another app is copied.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use egui::{Color32, Pos2, Rect, Stroke};

use super::icon_layers::layer_icons;
use super::icon_tools::tools;
use super::paint::ink_line;

/// Side of an icon's square, in points.
pub const ICON_SIZE: f32 = 26.0;
/// Line weight of every icon, in points.
const ICON_LINE: f32 = 1.5;
/// Points on a curve.
pub(super) const ARC_STEPS: usize = 12;

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
    /// A round brush held at a slant, its tip down to the left: the brushes.
    Brush,
    /// A fingertip at a slant, down to the left, over the trail it smeared: the smudge.
    Smudge,
    /// A block at a slant, its rubber end down to the left, on a line: the eraser.
    Eraser,
    /// A pipette at a slant, its tip down to the left: pick a colour from the canvas.
    Eyedropper,
    /// A spanner at a slant, its open ring up to the right: the Actions menu.
    Actions,
    /// A wand at a slant, a spark at its tip up to the right: the Adjustments menu.
    Adjustments,
    /// A lasso, its loop in dashes: the Selection tool.
    Selection,
    /// A pointer arrow, its tip up to the left: the Transform tool.
    Transform,
    /// A cross: close.
    Close,
    /// Four arrows out from the middle: move the window.
    Move,
    /// Two short slants in the bottom-right corner: resize from here.
    Resize,
    /// A check mark: done.
    Done,
    /// A small dot: the thin end of the size slider.
    SizeSmall,
    /// A large dot: the wide end of the size slider.
    SizeLarge,
    /// A faint disc in an outline: the see-through end of the opacity slider.
    OpacityLight,
    /// A solid disc: the opaque end of the opacity slider.
    OpacityDark,
    /// A folder: a group of layers.
    Group,
    /// A chevron pointing down: an open group; tapping closes it.
    Expanded,
    /// A chevron pointing right: a closed group; tapping opens it.
    Collapsed,
    /// An arrow bending down: the layer clips to the one under it.
    Clipped,
    /// A sheet with a round window in it: a layer mask.
    Mask,
    /// Three dots in a row: more for this item.
    More,
}

pub(super) type Line = Vec<[f32; 2]>;

/// Draws `icon` into `rect` (square, `ICON_SIZE` on a side) in `color`.
pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let at = |[x, y]: [f32; 2]| -> Pos2 { rect.min + egui::vec2(x, y) * rect.size() };
    let stroke = Stroke::new(ICON_LINE, color);
    for line in polylines(icon) {
        let points: Vec<Pos2> = line.into_iter().map(at).collect();
        ink_line(painter, &points, stroke);
    }
    match icon {
        Icon::Alert => {
            painter.circle_stroke(rect.center(), 0.38 * rect.width(), stroke);
            painter.circle_filled(at([0.5, 0.7]), ICON_LINE * 0.7, color);
        }
        // The pupil.
        Icon::Visible | Icon::Hidden => {
            painter.circle_stroke(rect.center(), 0.11 * rect.width(), stroke);
        }
        Icon::SizeSmall => {
            painter.circle_filled(rect.center(), 0.1 * rect.width(), color);
        }
        Icon::SizeLarge => {
            painter.circle_filled(rect.center(), 0.36 * rect.width(), color);
        }
        Icon::OpacityDark => {
            painter.circle_filled(rect.center(), 0.26 * rect.width(), color);
        }
        Icon::More => {
            for x in [0.26, 0.5, 0.74] {
                painter.circle_filled(at([x, 0.5]), ICON_LINE, color);
            }
        }
        Icon::OpacityLight => {
            painter.circle(
                rect.center(),
                0.26 * rect.width() - ICON_LINE / 2.0,
                color.gamma_multiply(0.2),
                stroke,
            );
        }
        _ => {}
    }
}

/// The open lines of `icon` in a unit square, y down.
fn polylines(icon: Icon) -> Vec<Line> {
    let mirror_x = |line: Line| line.into_iter().map(|[x, y]| [1.0 - x, y]).collect();
    let mirror_y = |line: Line| line.into_iter().map(|[x, y]| [x, 1.0 - y]).collect();
    let up = || {
        vec![
            vec![[0.5, 0.2], [0.5, 0.8]],
            vec![[0.28, 0.42], [0.5, 0.2], [0.72, 0.42]],
        ]
    };
    match icon {
        Icon::Undo => turn_back_arrow(),
        Icon::Redo => polylines(Icon::Undo).into_iter().map(mirror_x).collect(),
        Icon::Alert => vec![vec![[0.5, 0.3], [0.5, 0.54]]],
        Icon::Layers => sheets(),
        Icon::Add => vec![
            vec![[0.5, 0.22], [0.5, 0.78]],
            vec![[0.22, 0.5], [0.78, 0.5]],
        ],
        Icon::Raise => up(),
        Icon::Lower => up().into_iter().map(mirror_y).collect(),
        Icon::Delete => bin(),
        Icon::Visible => vec![eye_outline()],
        Icon::Hidden => vec![eye_outline(), vec![[0.2, 0.8], [0.8, 0.2]]],
        Icon::Duplicate => vec![rounded_rect([0.38, 0.38], [0.82, 0.82], 0.08), back_sheet()],
        Icon::Brush
        | Icon::Smudge
        | Icon::Eraser
        | Icon::Eyedropper
        | Icon::Selection
        | Icon::Adjustments
        | Icon::Transform => tools(icon),
        Icon::Actions => spanner(),
        Icon::Close => vec![
            vec![[0.27, 0.27], [0.73, 0.73]],
            vec![[0.73, 0.27], [0.27, 0.73]],
        ],
        Icon::Move => four_arrows(),
        Icon::Resize => vec![
            vec![[0.34, 0.82], [0.82, 0.34]],
            vec![[0.58, 0.82], [0.82, 0.58]],
        ],
        Icon::Done => vec![vec![[0.22, 0.52], [0.42, 0.72], [0.78, 0.32]]],
        Icon::More => [0.26, 0.5, 0.74]
            .map(|x| vec![[x - 0.01, 0.5], [x + 0.01, 0.5]])
            .to_vec(),
        Icon::Group | Icon::Expanded | Icon::Collapsed | Icon::Clipped | Icon::Mask => {
            layer_icons(icon)
        }
        // Discs, painted whole.
        Icon::SizeSmall | Icon::SizeLarge | Icon::OpacityLight | Icon::OpacityDark => Vec::new(),
    }
}

/// What shows of the sheet behind a copy: its top and left edges, round at the corners.
fn back_sheet() -> Line {
    let radius = 0.08;
    let mut line = vec![[0.62, 0.38]];
    line.extend(arc([0.54, 0.26], radius, 0.0, -FRAC_PI_2, 4));
    line.extend(arc([0.26, 0.26], radius, -FRAC_PI_2, -PI, 4));
    line.extend(arc([0.26, 0.54], radius, PI, FRAC_PI_2, 4));
    line.push([0.38, 0.62]);
    line
}

/// Points on a circle about `centre` from angle `from` to `to` (radians, y down: positive
/// turns clockwise on screen), `steps` segments.
pub(super) fn arc(centre: [f32; 2], radius: f32, from: f32, to: f32, steps: usize) -> Line {
    (0..=steps)
        .map(|step| {
            let angle = from + (to - from) * step as f32 / steps as f32;
            [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

/// A closed rectangle from `min` to `max` with corners of `radius`.
fn rounded_rect(min: [f32; 2], max: [f32; 2], radius: f32) -> Line {
    let (l, t, r, b) = (
        min[0] + radius,
        min[1] + radius,
        max[0] - radius,
        max[1] - radius,
    );
    let mut line = arc([r, t], radius, -FRAC_PI_2, 0.0, 4);
    line.extend(arc([r, b], radius, 0.0, FRAC_PI_2, 4));
    line.extend(arc([l, b], radius, FRAC_PI_2, PI, 4));
    line.extend(arc([l, t], radius, PI, 1.5 * PI, 4));
    line.push(line[0]);
    line
}

/// Points on the quadratic curve from `a` through the pull of `c` to `b`.
pub(super) fn curve(a: [f32; 2], c: [f32; 2], b: [f32; 2]) -> Line {
    (0..=ARC_STEPS)
        .map(|step| {
            let t = step as f32 / ARC_STEPS as f32;
            let u = 1.0 - t;
            [
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]
        })
        .collect()
}

/// A bin: the lid with its handle, the body narrowing to a rounded foot, two ribs.
fn bin() -> Vec<Line> {
    let mut body = vec![[0.26, 0.3]];
    body.extend(arc([0.38, 0.76], 0.06, PI - 0.08, FRAC_PI_2, 3));
    body.extend(arc([0.62, 0.76], 0.06, FRAC_PI_2, 0.08, 3));
    body.push([0.74, 0.3]);
    vec![
        vec![[0.18, 0.3], [0.82, 0.3]],
        vec![[0.4, 0.3], [0.42, 0.2], [0.58, 0.2], [0.6, 0.3]],
        body,
        vec![[0.44, 0.42], [0.45, 0.68]],
        vec![[0.56, 0.42], [0.55, 0.68]],
    ]
}

/// A handle from the lower left up to a ring that is open towards the upper right.
fn spanner() -> Vec<Line> {
    let (centre, radius) = ([0.62, 0.38], 0.19);
    // The gap faces up and to the right (y down), 70 degrees wide.
    let open = -FRAC_PI_4;
    let gap = 35f32.to_radians();
    let ring = arc(
        centre,
        radius,
        open + gap,
        open + 2.0 * PI - gap,
        ARC_STEPS * 2,
    );
    let foot = [
        centre[0] - radius * FRAC_PI_4.cos(),
        centre[1] + radius * FRAC_PI_4.sin(),
    ];
    vec![ring, vec![foot, [0.18, 0.82]]]
}

/// A cross whose four ends are open arrow heads.
fn four_arrows() -> Vec<Line> {
    let head = |[x, y]: [f32; 2], [dx, dy]: [f32; 2]| {
        // Back from the tip along the arm, spread across it.
        let (bx, by) = (x - 0.1 * dx, y - 0.1 * dy);
        vec![
            [bx + 0.1 * dy, by + 0.1 * dx],
            [x, y],
            [bx - 0.1 * dy, by - 0.1 * dx],
        ]
    };
    vec![
        vec![[0.5, 0.16], [0.5, 0.84]],
        vec![[0.16, 0.5], [0.84, 0.5]],
        head([0.5, 0.16], [0.0, -1.0]),
        head([0.5, 0.84], [0.0, 1.0]),
        head([0.16, 0.5], [-1.0, 0.0]),
        head([0.84, 0.5], [1.0, 0.0]),
    ]
}

/// Three flat rhombi stacked; of the two below the top one only the front edges show.
fn sheets() -> Vec<Line> {
    let top = [
        [0.5, 0.2],
        [0.84, 0.37],
        [0.5, 0.54],
        [0.16, 0.37],
        [0.5, 0.2],
    ];
    let mut lines = vec![top.to_vec()];
    for drop in [0.14, 0.28] {
        lines.push(top[1..4].iter().map(|[x, y]| [*x, y + drop]).collect());
    }
    lines
}

/// An almond shape across the square: two arcs meeting at the corners of the eye.
fn eye_outline() -> Line {
    let mut points = Vec::new();
    for upper in [true, false] {
        for step in 0..=ARC_STEPS {
            let t = step as f32 / ARC_STEPS as f32;
            let x = if upper {
                0.14 + 0.72 * t
            } else {
                0.86 - 0.72 * t
            };
            let lift = 0.24 * (PI * t).sin();
            points.push([x, if upper { 0.5 - lift } else { 0.5 + lift }]);
        }
    }
    points
}

/// An arrow pointing left along the top, turning down and back along the bottom: the shaft
/// from the tip through the turn, and the open head at the tip.
fn turn_back_arrow() -> Vec<Line> {
    let tip = [0.2, 0.38];
    let (centre, radius) = ([0.6, 0.57], 0.19);
    let mut shaft = vec![tip];
    shaft.extend(arc(centre, radius, -FRAC_PI_2, FRAC_PI_2, ARC_STEPS));
    shaft.push([0.32, centre[1] + radius]);
    let head = vec![[0.34, 0.24], tip, [0.34, 0.52]];
    vec![shaft, head]
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
            Icon::Alert,
            Icon::Layers,
            Icon::Add,
            Icon::Raise,
            Icon::Lower,
            Icon::Delete,
            Icon::Visible,
            Icon::Hidden,
            Icon::Duplicate,
            Icon::Brush,
            Icon::Smudge,
            Icon::Eraser,
            Icon::Eyedropper,
            Icon::Actions,
            Icon::Adjustments,
            Icon::Selection,
            Icon::Transform,
            Icon::Close,
            Icon::Move,
            Icon::Resize,
            Icon::Done,
            Icon::Group,
            Icon::Expanded,
            Icon::Collapsed,
            Icon::Clipped,
            Icon::Mask,
            Icon::More,
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
