//! How the chrome is drawn, once: the frame of every island and panel, the hand's press, the
//! active mark (an ink ring or outline, never a filled block), ink lines with round joins, and
//! the soft shadow a sheet of paper casts on the desk. Components draw with these; nothing else
//! picks a fill or a line.

use std::f32::consts::PI;

use egui::{Color32, Frame, Margin, Mesh, Pos2, Rect, Shape, Stroke, Vec2};

use super::palette::{INK_ACTIVE, PANEL, PANEL_RULE, PANEL_RULE_WIDTH, PRESSED, SHEET_SHADOW};
use super::{CORNER, SPACE};

/// Weight of the active mark's line, in points.
const MARK_LINE: f32 = 1.5;
/// Points on a sheet shadow's rounded corner.
const CORNER_STEPS: usize = 6;

/// The frame of every island and panel over the canvas: paper inside an ink outline, with
/// `margin`.
pub fn panel_frame(margin: Margin) -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE))
        .corner_radius(CORNER)
        .inner_margin(margin)
}

/// Under the hand while it presses the control in `rect`.
pub fn paint_pressed(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CORNER, PRESSED);
}

/// The active mark of an icon button or swatch whose touch target is `rect`: an ink ring.
pub fn paint_icon_mark(painter: &egui::Painter, rect: Rect) {
    let side = rect.width().min(rect.height());
    painter.circle_stroke(
        rect.center(),
        0.4 * side,
        Stroke::new(MARK_LINE, INK_ACTIVE),
    );
}

/// The active mark of a row across a panel: an ink outline around it.
pub fn paint_row_mark(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(
        rect.shrink2(Vec2::new(MARK_LINE, SPACE / 4.0 + MARK_LINE)),
        CORNER,
        Stroke::new(MARK_LINE, INK_ACTIVE),
        egui::StrokeKind::Middle,
    );
}

/// An open line through `points` with round joins and round ends, as a pen would draw it.
pub fn ink_line(painter: &egui::Painter, points: &[Pos2], stroke: Stroke) {
    if points.len() > 1 {
        painter.add(Shape::line(points.to_vec(), stroke));
    }
    for point in points {
        painter.circle_filled(*point, stroke.width / 2.0, stroke.color);
    }
}

/// The soft shadow of a sheet whose corners are `corners` (a rectangle, turned any way, corners
/// in order) on the desk: it lies wholly outside the sheet, a little deeper below it.
pub fn paint_sheet_shadow(painter: &egui::Painter, corners: [Pos2; 4]) {
    let alpha = f32::from(SHEET_SHADOW);
    // A tight contact shadow, then a wide soft one dropped a little.
    for (size, share, drop) in [(2.0, 0.7, 0.0), (3.0 * SPACE, 0.45, SPACE / 2.0)] {
        let strength = (alpha * share).round() as u8;
        shadow_ring(painter, corners, size, Vec2::new(0.0, drop), strength);
    }
}

fn shadow_ring(painter: &egui::Painter, corners: [Pos2; 4], size: f32, drop: Vec2, alpha: u8) {
    let centre = corners.iter().fold(Vec2::ZERO, |sum, c| sum + c.to_vec2()) / 4.0;
    // The edge from `a` to `b`'s normal, pointing away from the middle.
    let outward = |a: Pos2, b: Pos2| {
        let along = (b - a).normalized();
        let normal = Vec2::new(along.y, -along.x);
        let mid = (a.to_vec2() + b.to_vec2()) / 2.0;
        if normal.dot(mid - centre) < 0.0 {
            -normal
        } else {
            normal
        }
    };
    let (dark, clear) = (Color32::from_black_alpha(alpha), Color32::TRANSPARENT);
    let mut mesh = Mesh::default();
    for i in 0..4 {
        let (before, here, after) = (corners[(i + 3) % 4], corners[i], corners[(i + 1) % 4]);
        let (from, to) = (outward(before, here), outward(here, after));
        let start = from.y.atan2(from.x);
        let turn = (to.y.atan2(to.x) - start + PI).rem_euclid(2.0 * PI) - PI;
        for step in 0..=CORNER_STEPS {
            let angle = start + turn * step as f32 / CORNER_STEPS as f32;
            let out = here + size * Vec2::angled(angle) + drop;
            mesh.colored_vertex(here, dark);
            mesh.colored_vertex(out, clear);
        }
    }
    let count = mesh.vertices.len() as u32;
    for pair in 0..count / 2 {
        let (a, b) = (2 * pair, 2 * pair + 1);
        let (c, d) = ((a + 2) % count, (b + 2) % count);
        mesh.add_triangle(a, b, d);
        mesh.add_triangle(a, d, c);
    }
    painter.add(Shape::mesh(mesh));
}
