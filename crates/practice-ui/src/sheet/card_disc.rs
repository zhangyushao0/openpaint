//! A key shift's hue and chroma disc on the practice card (S107, `card_disc.rs`): Oklab's
//! `a`, `b` plane, grey in the middle and `RIM` chroma at the edge, its colours shown faintly at a
//! middle lightness; each area's crop colour a small ring, the copy's a dot in its colour, an
//! arrow from one to the other (the chosen area's heavier); the tendency a larger disc; and
//! Gurney's gamut mask, a triangle whose apex is the tendency at high chroma and whose base lies
//! past the grey centre on the other side, so the tendency's opposites are allowed only greyed,
//! everything outside it veiled in paper. It shows only on a compared key shift's card.
//!
//! A gamut mask's disc (S153) is the same disc with the mask's own shape, any convex polygon, so
//! grey may lie outside it (veiled from the middle out to where the mask begins), outlined in ink:
//! on the card (`gamut`, the areas outside it ringed red), beside its frame on the sheet and under
//! a How's frames (`masked_disc`). Colour under light's (S154, `mixed`): the colours it gave as
//! squares, the copy's regions as dots.
//!
//! A cold-warm copy's point compared (S154) lies on the same disc unveiled, beside its two
//! tendencies (`leaning`).

use egui::{Color32, Mesh, Pos2, Sense, Shape, Stroke, Vec2};
use openpaint_practice::{Point, Rgb8, gamut_span, in_gamut, oklab};

use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, PANEL};

use super::marks::colour;
use crate::palette::CORRECTION;

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
/// A ring's and a dot's radius, and the tendency's, in points; the red ring round a dot outside a
/// gamut mask.
const MARK: f32 = 4.5;
const TENDENCY_MARK: f32 = 8.0;
const OUTSIDE_RING: f32 = 9.0;
/// A given colour's square's half side, and a dash of the line from it, in points (S154).
const GIVEN: f32 = 6.0;
const DASH: f32 = 3.0;

/// The disc for the crop's `originals` and the copy's `colours` (none where not painted), the
/// tendency where it lies in its colour, Gurney's `mask`, the `chosen` area heavier.
pub(crate) fn disc(
    ui: &mut egui::Ui,
    (originals, colours): (&[Rgb8], &[Option<Rgb8>]),
    (tendency, tendency_colour): (Point, Rgb8),
    mask: &[Point; 3],
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
    veil(painter, (centre, radius), mask);
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
    let shown = at(tendency);
    painter.circle(
        shown,
        TENDENCY_MARK,
        colour(tendency_colour),
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

/// A gamut mask's disc on the card (S153): each area's crop colour a small ring, the copy's a dot
/// in its colour, an arrow from one to the other (the `chosen` area heavier), the `marked` areas'
/// dots (outside the mask) ringed red; the mask (`corners`) outlined, the disc veiled outside it.
pub(crate) fn gamut(
    ui: &mut egui::Ui,
    (originals, colours): (&[Rgb8], &[Option<Rgb8>]),
    corners: &[Point],
    (marked, chosen): (&[usize], usize),
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, SIDE), Sense::hover());
    let (centre, radius) = (rect.center(), SIDE / 2.0 - 4.0);
    let painter = ui.painter();
    masked_disc(painter, (centre, radius), corners);
    let at = |colour: Rgb8| {
        let [_, a, b] = oklab(colour);
        on_disc((centre, radius), [a, b])
    };
    for (area, (original, mine)) in originals.iter().zip(colours).enumerate() {
        let weight = if area == chosen { 2.5 } else { 1.25 };
        let from = at(*original);
        if let Some(mine) = mine {
            let to = at(*mine);
            arrow(painter, from, to, Stroke::new(weight, INK_SECONDARY));
            painter.circle(to, MARK, colour(*mine), Stroke::new(weight, INK));
            if marked.contains(&area) {
                painter.circle_stroke(to, OUTSIDE_RING, Stroke::new(2.0, CORRECTION));
            }
        }
        painter.circle_stroke(from, MARK, Stroke::new(weight, INK));
    }
}

/// The hue and chroma disc with the colours an exercise gave to mix from (S154, colour under light):
/// each given colour a small square, each of the copy's regions a dot in its colour with a thin
/// dashed line from the given colour it was mixed from, the `marked` dots ringed red.
pub(crate) fn mixed(
    ui: &mut egui::Ui,
    given: &[Rgb8],
    copy: &[(Option<Rgb8>, Option<usize>)],
    marked: &[usize],
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, SIDE), Sense::hover());
    let (centre, radius) = (rect.center(), SIDE / 2.0 - 4.0);
    let painter = ui.painter();
    painter.add(Shape::mesh(colours_of(centre, radius)));
    painter.circle_stroke(centre, radius, Stroke::new(1.0, DIVIDER));
    let at = |colour: Rgb8| {
        let [_, a, b] = oklab(colour);
        on_disc((centre, radius), [a, b])
    };
    for (mine, from) in copy {
        let (Some(mine), Some(from)) = (mine, from.and_then(|k| given.get(k))) else {
            continue;
        };
        let line = [at(*from), at(*mine)];
        let stroke = Stroke::new(1.0, INK_SECONDARY);
        painter.extend(Shape::dashed_line(&line, stroke, DASH, DASH));
    }
    for given in given {
        let square = egui::Rect::from_center_size(at(*given), Vec2::splat(2.0 * GIVEN));
        painter.rect(
            square,
            0.0,
            colour(*given),
            Stroke::new(1.0, INK),
            egui::StrokeKind::Middle,
        );
    }
    for (region, (mine, _)) in copy.iter().enumerate() {
        let Some(mine) = mine else { continue };
        painter.circle(at(*mine), MARK, colour(*mine), Stroke::new(1.25, INK));
        if marked.contains(&region) {
            painter.circle_stroke(at(*mine), OUTSIDE_RING, Stroke::new(2.0, CORRECTION));
        }
    }
}

/// A point compared on the disc (S154, a cold-warm copy's): the original's colour there a small
/// ring, the copy's a dot in its colour with an arrow from one to the other, and the two
/// `tendencies` (Oklab `a`, `b`, each with its colour) as larger discs.
pub(crate) fn leaning(
    ui: &mut egui::Ui,
    (original, copy): (Rgb8, Rgb8),
    tendencies: &[(Point, Rgb8); 2],
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, SIDE), Sense::hover());
    let (centre, radius) = (rect.center(), SIDE / 2.0 - 4.0);
    let painter = ui.painter();
    painter.add(Shape::mesh(colours_of(centre, radius)));
    painter.circle_stroke(centre, radius, Stroke::new(1.0, DIVIDER));
    for (ab, rgb) in tendencies {
        let at = on_disc((centre, radius), *ab);
        painter.circle(at, TENDENCY_MARK, colour(*rgb), Stroke::new(1.5, INK));
    }
    let at = |rgb: Rgb8| {
        let [_, a, b] = oklab(rgb);
        on_disc((centre, radius), [a, b])
    };
    let (from, to) = (at(original), at(copy));
    arrow(painter, from, to, Stroke::new(2.5, INK_SECONDARY));
    painter.circle(to, MARK, colour(copy), Stroke::new(2.5, INK));
    painter.circle_stroke(from, MARK, Stroke::new(2.5, INK));
}

/// A hue and chroma disc at `centre` of `radius` points with a gamut mask: its colours softened,
/// paper over it outside the convex polygon `corners` (Oklab `a`, `b`), the polygon outlined in
/// ink (the card's, the sheet's beside a frame, a How's strip).
pub(crate) fn masked_disc(
    painter: &egui::Painter,
    (centre, radius): (Pos2, f32),
    corners: &[Point],
) {
    painter.add(Shape::mesh(colours_of(centre, radius)));
    let mut veil = Mesh::default();
    let paper = PANEL.gamma_multiply(VEIL);
    let scale = f64::from(radius) / RIM;
    let rim = f64::from(radius);
    for step in 0..=VEIL_STEPS {
        let angle = std::f64::consts::TAU * step as f64 / VEIL_STEPS as f64;
        let d = [angle.cos(), angle.sin()];
        let (near, far) = gamut_span(corners, d).map_or((0.0, 0.0), |(near, far)| {
            ((near * scale).min(rim), (far * scale).min(rim))
        });
        let screen = Vec2::new(d[0] as f32, -d[1] as f32);
        for at in [0.0, near, far, rim] {
            veil.colored_vertex(centre + screen * at as f32, paper);
        }
    }
    for step in 1..=VEIL_STEPS as u32 {
        let (was, now) = (4 * (step - 1), 4 * step);
        // Grey to where the ray enters the mask, and from where it leaves to the rim.
        for (inner, outer) in [(0, 1), (2, 3)] {
            veil.add_triangle(was + inner, was + outer, now + inner);
            veil.add_triangle(was + outer, now + outer, now + inner);
        }
    }
    painter.add(Shape::mesh(veil));
    painter.circle_stroke(centre, radius, Stroke::new(1.0, DIVIDER));
    let mut outline: Vec<Pos2> = (corners.iter())
        .map(|p| on_disc((centre, radius), *p))
        .collect();
    outline.extend(outline.first().copied());
    painter.add(Shape::line(outline, Stroke::new(1.25, INK)));
}

/// The point of the disc at `centre` of `radius` points where the Oklab `a`, `b` point `ab` lies,
/// held inside its rim.
fn on_disc((centre, radius): (Pos2, f32), [a, b]: Point) -> Pos2 {
    let scale = f64::from(radius) / RIM;
    let [x, y] = [a * scale, -b * scale];
    let long = x.hypot(y).max(f64::MIN_POSITIVE);
    let kept = long.min(f64::from(radius)) / long;
    centre + Vec2::new((x * kept) as f32, (y * kept) as f32)
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
