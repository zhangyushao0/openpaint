//! A key shift's hue and chroma disc on the practice card (S107, `practice_card_key.rs`): Oklab's
//! `a`, `b` plane, grey in the middle and `RIM` chroma at the edge, its colours shown faintly at a
//! middle lightness; each area's crop colour a small ring, the copy's a dot in its colour, an
//! arrow from one to the other (the chosen area's heavier); the tendency a larger disc; and
//! Gurney's gamut mask, a triangle whose apex is the tendency at high chroma and whose base lies
//! past the grey centre on the other side, so the tendency's opposites are allowed only greyed,
//! everything outside it veiled in paper. It shows only on a compared key shift's card.

use egui::{Color32, Mesh, Pos2, Sense, Shape, Stroke, Vec2};
use openpaint_practice::{Point, Rgb8, Tendency, in_gamut, oklab};

use crate::design::{DIVIDER, INK, INK_SECONDARY, PANEL};
use crate::practice_marks::colour;

/// The disc's side, in points, and the chroma at its rim (Oklab).
const SIDE: f32 = 220.0;
const RIM: f64 = 0.2;
/// The disc's colours' lightness and how much paper softens them.
const SHOWN_LIGHTNESS: f64 = 0.75;
const SOFTENED: f32 = 0.35;
/// Its rings and sectors, and the veil's steps round.
const RINGS: usize = 5;
const SECTORS: usize = 48;
const VEIL_STEPS: usize = 96;
const VEIL: f32 = 0.82;
/// The mask's apex past the tendency's chroma, its base past grey and its base's half width, as
/// shares of the apex's chroma.
const APEX_PAST: f64 = 0.05;
const BASE_PAST: f64 = 0.4;
const BASE_HALF: f64 = 0.6;
/// A ring's and a dot's radius, and the tendency's, in points.
const MARK: f32 = 4.5;
const TENDENCY_MARK: f32 = 8.0;

/// The disc for the crop's `originals` and the copy's `colours` (none where not painted), its
/// tendency and gamut mask, the `chosen` area heavier.
pub(crate) fn disc(
    ui: &mut egui::Ui,
    (originals, colours): (&[Rgb8], &[Option<Rgb8>]),
    tendency: Tendency,
    chosen: usize,
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, SIDE), Sense::hover());
    let centre = rect.center();
    let radius = SIDE / 2.0 - 4.0;
    let at = |[a, b]: Point| -> Pos2 {
        let scale = f64::from(radius) / RIM;
        let [x, y] = [a * scale, -b * scale];
        let long = x.hypot(y).max(f64::MIN_POSITIVE);
        let kept = long.min(f64::from(radius)) / long;
        centre + Vec2::new((x * kept) as f32, (y * kept) as f32)
    };
    let painter = ui.painter();
    painter.add(Shape::mesh(colours_of(centre, radius)));
    veil(painter, (centre, radius), &mask(tendency));
    painter.circle_stroke(centre, radius, Stroke::new(1.0, DIVIDER));
    let ab = |c: Rgb8| {
        let [_, a, b] = oklab(c);
        [a, b]
    };
    for (area, (original, mine)) in originals.iter().zip(colours).enumerate() {
        let weight = if area == chosen { 2.5 } else { 1.25 };
        let from = at(ab(*original));
        if let Some(mine) = mine {
            let to = at(ab(*mine));
            arrow(painter, from, to, Stroke::new(weight, INK_SECONDARY));
            painter.circle(to, MARK, colour(*mine), Stroke::new(weight, INK));
        }
        painter.circle_stroke(from, MARK, Stroke::new(weight, INK));
    }
    let shown = at(tendency.ab());
    painter.circle(
        shown,
        TENDENCY_MARK,
        colour(tendency.colour()),
        Stroke::new(1.5, INK),
    );
}

/// The disc's colours, softened by paper: rings of sectors round the grey middle.
fn colours_of(centre: Pos2, radius: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let shown = |chroma: f64, hue: f64| -> Color32 {
        colour(in_gamut([SHOWN_LIGHTNESS, chroma, hue])).lerp_to_gamma(PANEL, SOFTENED)
    };
    for ring in 0..=RINGS {
        let share = ring as f64 / RINGS as f64;
        for sector in 0..=SECTORS {
            let hue = 360.0 * sector as f64 / SECTORS as f64;
            let (sin, cos) = hue.to_radians().sin_cos();
            let r = radius * share as f32;
            let p = centre + Vec2::new(r * cos as f32, -r * sin as f32);
            mesh.colored_vertex(p, shown(RIM * share, hue));
        }
    }
    let across = (SECTORS + 1) as u32;
    for ring in 1..=RINGS as u32 {
        for sector in 0..SECTORS as u32 {
            let [inner, outer] = [(ring - 1) * across + sector, ring * across + sector];
            mesh.add_triangle(inner, outer, outer + 1);
            mesh.add_triangle(inner, outer + 1, inner + 1);
        }
    }
    mesh
}

/// Gurney's gamut mask for `tendency` (Oklab `a`, `b`): the apex at the tendency's direction past
/// its chroma, the base across the other side past grey.
fn mask(tendency: Tendency) -> [Point; 3] {
    let [u, v] = tendency.direction();
    let apex = tendency.chroma() + APEX_PAST;
    let (base, half) = (-BASE_PAST * apex, BASE_HALF * apex);
    [
        [u * apex, v * apex],
        [u * base - v * half, v * base + u * half],
        [u * base + v * half, v * base - u * half],
    ]
}

/// Paper over the disc (`centre`, `radius`) outside `triangle` (Oklab `a`, `b`, holding grey):
/// a quad a step round, from where the ray meets the triangle's side out to the rim.
fn veil(painter: &egui::Painter, (centre, radius): (Pos2, f32), triangle: &[Point; 3]) {
    let mut mesh = Mesh::default();
    let paper = PANEL.gamma_multiply(VEIL);
    let scale = f64::from(radius) / RIM;
    for step in 0..=VEIL_STEPS {
        let angle = std::f64::consts::TAU * step as f64 / VEIL_STEPS as f64;
        let d = [angle.cos(), angle.sin()];
        let inner = (reach(triangle, d) * scale).min(f64::from(radius)) as f32;
        let screen = Vec2::new(d[0] as f32, -d[1] as f32);
        mesh.colored_vertex(centre + screen * inner, paper);
        mesh.colored_vertex(centre + screen * radius, paper);
    }
    for step in 1..=VEIL_STEPS as u32 {
        let i = 2 * step;
        mesh.add_triangle(i - 2, i - 1, i);
        mesh.add_triangle(i - 1, i + 1, i);
    }
    painter.add(Shape::mesh(mesh));
}

/// How far from grey the ray along `d` (a unit direction) leaves `triangle`.
fn reach(triangle: &[Point; 3], d: Point) -> f64 {
    let mut nearest = f64::INFINITY;
    for k in 0..3 {
        let (p, q) = (triangle[k], triangle[(k + 1) % 3]);
        let e = [q[0] - p[0], q[1] - p[1]];
        let det = d[0] * -e[1] + e[0] * d[1];
        if det.abs() < 1e-12 {
            continue;
        }
        let t = (p[0] * -e[1] + e[0] * p[1]) / det;
        let s = (d[0] * p[1] - d[1] * p[0]) / det;
        if t > 0.0 && (-1e-9..=1.0 + 1e-9).contains(&s) {
            nearest = nearest.min(t);
        }
    }
    if nearest.is_finite() { nearest } else { 0.0 }
}

/// An arrow from `from` to `to`, its head at `to`'s mark.
fn arrow(painter: &egui::Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    let d = to - from;
    let long = d.length();
    if long <= 2.0 * MARK {
        return;
    }
    let unit = d / long;
    let tip = to - unit * MARK;
    painter.line_segment([from + unit * MARK, tip], stroke);
    let side = Vec2::new(-unit.y, unit.x) * 3.0;
    painter.line_segment([tip, tip - unit * 5.0 + side], stroke);
    painter.line_segment([tip, tip - unit * 5.0 - side], stroke);
}
