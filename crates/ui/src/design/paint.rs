//! How the chrome is drawn, once: the frame of every island and panel, the hand's press (in the
//! control's own outline), the active mark (an ink ring, the same ring as a pill around a word,
//! or an outline; never a filled block), the divider in a bar, ink lines with round joins, and
//! the soft shadow a sheet of paper casts on the desk. Components draw with these; nothing else
//! picks a fill or a line.

use std::f32::consts::PI;

use egui::{Color32, Frame, Margin, Mesh, Pos2, Rect, Shape, Stroke, Vec2};

use super::palette::{
    DIVIDER, INK_ACTIVE, PANEL, PANEL_RULE, PANEL_RULE_WIDTH, PRESSED, SHEET_SHADOW,
};
use super::{CORNER, SPACE};

/// Weight of the active mark's line, in points.
pub const MARK_LINE: f32 = 1.5;
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

/// The frame of a surface whose paper the motion draws (D44): `panel_frame`'s layout, outline
/// width included, painting nothing itself.
pub fn surface_frame(margin: Margin) -> Frame {
    Frame::new()
        .stroke(Stroke::new(PANEL_RULE_WIDTH, Color32::TRANSPARENT))
        .corner_radius(CORNER)
        .inner_margin(margin)
}

/// The shape a control draws its active mark in: the ring of an icon, the pill round a word,
/// the outline of a row. Its press wash fills the very same shape, so the grey under the hand
/// never reaches past the line (the touch target around it stays invisible).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    pub rect: Rect,
    pub corner: f32,
}

impl Outline {
    /// The ring of an icon button or swatch whose touch target is `target`: a circle 0.8 of
    /// its side across.
    #[must_use]
    pub fn ring(target: Rect) -> Self {
        let across = 0.8 * target.width().min(target.height());
        Self::round(Rect::from_center_size(target.center(), Vec2::splat(across)))
    }

    /// The pill round a word whose place (the word and its margins, one touch target high) is
    /// `word`: the ring's height, the place's width less its outer margins, fully rounded.
    #[must_use]
    pub fn pill(word: Rect) -> Self {
        let side = word.width().min(word.height());
        let size = Vec2::new(word.width() - 1.5 * SPACE, 0.8 * side);
        Self::round(Rect::from_center_size(word.center(), size))
    }

    /// The outline of a row across a panel whose touch target is `row`.
    #[must_use]
    pub fn row(row: Rect) -> Self {
        Self {
            rect: row.shrink2(Vec2::new(MARK_LINE, SPACE / 4.0 + MARK_LINE)),
            corner: CORNER,
        }
    }

    /// `rect` with its ends fully rounded (a ring when square, a pill when wider).
    #[must_use]
    pub fn round(rect: Rect) -> Self {
        Self {
            rect,
            corner: rect.width().min(rect.height()) / 2.0,
        }
    }
}

/// Under the hand while it presses a control: a faint wash filling the control's `outline`.
pub fn paint_pressed(painter: &egui::Painter, outline: Outline) {
    painter.rect_filled(outline.rect, outline.corner, PRESSED);
}

/// The active mark in `outline` (an ink line of the ring's weight), `shown` from 0 (none) to 1
/// (whole). Coming and going it draws in from a little wider and fades, so the change is seen
/// where it happens (acknowledge, D44).
pub fn paint_mark(painter: &egui::Painter, outline: Outline, shown: f32) {
    if shown <= 0.0 {
        return;
    }
    let shown = shown.min(1.0);
    let short = outline.rect.width().min(outline.rect.height());
    let wider = 0.1 * short * (1.0 - shown);
    painter.rect_stroke(
        outline.rect.expand(wider),
        outline.corner + wider,
        Stroke::new(MARK_LINE, INK_ACTIVE.gamma_multiply(shown)),
        egui::StrokeKind::Middle,
    );
}

/// A thin upright line at `x` across `height` points centred on `middle_y`, setting a group of
/// controls apart from the next in a bar: the outline's weight in faint ink.
pub fn paint_divider(painter: &egui::Painter, x: f32, middle_y: f32, height: f32) {
    let half = height / 2.0;
    painter.vline(
        x,
        (middle_y - half)..=(middle_y + half),
        Stroke::new(PANEL_RULE_WIDTH, DIVIDER),
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
