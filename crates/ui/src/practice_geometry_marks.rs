//! A box's or an ellipse's marks over a practice sheet (S84), drawn by the screen through the view
//! as every original is (`practice_marks.rs`), never into the artwork:
//!
//! - a box to complete: the horizon across both frames and the front corner's three edges in
//!   each; once compared, the original's frame shows the whole exact box (its hidden edges dashed)
//!   with faint lines on to its vanishing points, ringed;
//! - free boxes: the horizon across both frames; in the original's, an example box on it with its
//!   lines on to its vanishing points;
//! - an ellipse: its frame (a table's cell or a square in perspective) in both; once compared, the
//!   original's shows the exact ellipse, the diagonals and the lines through the centre that
//!   find each side's middle.
//!
//! Compared, over the copy: the answer in the marking red (the exact box or ellipse, its vanishing
//! points or the middles it touches ringed, the true vertical through a vertical edge that leans);
//! the original's answer and construction kept inside its frame, where it cannot be taken for the
//! copy's; what was
//! found in the copy's own lines dashed in `FOUND` blue (each edge drawn on towards its point or
//! the horizon, red where it misses the point; where a free box's directions meet ringed; the
//! fitted ellipse); a front corner under 90 degrees marked red.

use egui::{Align2, FontId, Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Checked, Corner, Ellipse, Frame, Original, Pair, PerspectiveBox, Point};

use crate::design::{CORRECTION, DIVIDER, FOUND, INK, INK_SECONDARY, PANEL};
use crate::text::{self as t, Words};

/// A ring's radius round a vanishing point or a middle, in points; a mark's line and the paper's
/// halo under it; a faint line's.
const RING: f32 = 7.0;
const MARK: f32 = 2.0;
const HALO: f32 = 3.0;
const FAINT: f32 = 1.25;
/// Dashes and gaps, in points.
const DASH: f32 = 7.0;
const GAP: f32 = 5.0;
/// A narrow corner's arc's radius, in points; an ellipse drawn in pieces.
const ARC: f32 = 22.0;
const PIECES: usize = 96;
/// The horizon's label's size in canvas pixels of a frame.
const LABEL: f32 = 11.0;

/// Canvas points to the screen.
type At<'a> = &'a dyn Fn([f64; 2]) -> Pos2;

/// Exercise `index`'s original and givens in its two frames; an original's line `width` points
/// wide, `scale` points to a canvas pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let answered = practice.current == Some(index) && practice.result.is_some();
    let ink = Stroke::new(width, INK);
    match &practice.sheet.originals[index] {
        Original::Box(exact) => {
            for frame in [&pair.original, &pair.copy] {
                horizon(painter, (frame, exact.horizon), at, scale);
                for (a, b, _) in exact.given() {
                    segment(painter, frame, [a, b], at, ink);
                }
            }
            if answered {
                whole_box(painter, &pair.original, exact, at, width);
            }
        }
        Original::FreeBoxes(example) => {
            for frame in [&pair.original, &pair.copy] {
                horizon(painter, (frame, example.horizon), at, scale);
            }
            whole_box(painter, &pair.original, example, at, width);
        }
        Original::Ellipse(guide) => {
            for frame in [&pair.original, &pair.copy] {
                let corners: Vec<Pos2> = guide
                    .corners
                    .iter()
                    .map(|p| at(frame.to_canvas(*p)))
                    .collect();
                painter.add(Shape::closed_line(corners, ink));
            }
            if answered {
                let frame = &pair.original;
                let faint = Stroke::new(FAINT, DIVIDER);
                let c = guide.corners;
                for line in [
                    [c[0], c[2]],
                    [c[1], c[3]],
                    [guide.touches[0], guide.touches[2]],
                    [guide.touches[1], guide.touches[3]],
                ] {
                    segment(painter, frame, line, at, faint);
                }
                ellipse(painter, frame, &guide.exact, at, ink);
                for touch in guide.touches {
                    painter.circle_filled(at(frame.to_canvas(touch)), MARK + 1.0, INK);
                }
            }
        }
        _ => {}
    }
}

/// The horizon at `height` across `frame`, labelled small at its left.
fn horizon(painter: &egui::Painter, (frame, height): (&Frame, f64), at: At<'_>, scale: f32) {
    let ends = [[0.0, height], [1.0, height]];
    segment(painter, frame, ends, at, Stroke::new(FAINT, INK_SECONDARY));
    let label = Words::from(t::HORIZON).shown(painter.ctx());
    let place = at(frame.to_canvas([0.01, height - 0.012]));
    let font = FontId::proportional((LABEL * scale).max(6.0));
    painter.text(place, Align2::LEFT_BOTTOM, label, font, INK_SECONDARY);
}

/// The whole of `exact` in `frame`: its edges in ink (the hidden ones dashed), each drawn on
/// faintly to its vanishing point, ringed; kept inside the frame.
fn whole_box(
    painter: &egui::Painter,
    frame: &Frame,
    exact: &PerspectiveBox,
    at: At<'_>,
    width: f32,
) {
    let inside = egui::Rect::from_points(&frame.corners().map(at));
    let painter = &painter.with_clip_rect(inside.intersect(painter.clip_rect()));
    let faint = Stroke::new(FAINT, DIVIDER);
    for ((a, b, direction), hidden) in exact.edges().into_iter().zip(exact.hidden()) {
        if let Some(point) = exact.vanishing[direction.index()] {
            let near = if distance2(a, point) < distance2(b, point) {
                a
            } else {
                b
            };
            segment(painter, frame, [near, point], at, faint);
        }
        if hidden {
            dashed(painter, frame, &[a, b], at, Stroke::new(width / 2.0, INK));
        } else {
            segment(painter, frame, [a, b], at, Stroke::new(width, INK));
        }
    }
    for point in exact.vanishing.iter().flatten() {
        ring(painter, at(frame.to_canvas(*point)), INK_SECONDARY);
    }
}

/// A box's or an ellipse's comparison over the copy's frame; the answer's line `width` points.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (At<'_>, f32),
) {
    let Some(index) = practice.current else {
        return;
    };
    let frame = practice.sheet.pairs[index].copy;
    let red = Stroke::new(width / 2.0, CORRECTION.gamma_multiply(0.6));
    match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Box(check), Original::Box(exact)) => {
            for (a, b, _) in exact.edges() {
                segment(painter, &frame, [a, b], at, red);
            }
            for point in exact.vanishing.iter().flatten() {
                ring(painter, at(frame.to_canvas(*point)), CORRECTION);
            }
            for edge in &check.edges {
                found_edge(painter, &frame, edge, at);
            }
        }
        (Overlay::FreeBoxes(check), Original::FreeBoxes(_)) => {
            for free in &check.boxes {
                for edge in &free.edges {
                    found_edge(painter, &frame, edge, at);
                }
                for (_, meet) in &free.meets {
                    ring(painter, at(frame.to_canvas(*meet)), FOUND);
                }
                for corner in &free.narrow {
                    narrow(painter, &frame, corner, at);
                }
            }
        }
        (Overlay::Ellipse(check), Original::Ellipse(guide)) => {
            ellipse(painter, &frame, &guide.exact, at, red);
            for touch in guide.touches {
                ring(painter, at(frame.to_canvas(touch)), CORRECTION);
            }
            if let Some(fitted) = &check.fitted {
                let outline: Vec<Point> = fitted.outline(PIECES);
                dashed(painter, &frame, &outline, at, Stroke::new(MARK, FOUND));
            }
        }
        _ => {}
    }
}

/// An edge found in the copy: drawn on dashed towards its point, blue where it heads for it, red
/// where it misses; a vertical that leans, the true vertical through it in red.
fn found_edge(painter: &egui::Painter, frame: &Frame, edge: &Checked, at: At<'_>) {
    if let Some([from, to]) = edge.extension {
        let colour = if edge.on { FOUND } else { CORRECTION };
        dashed(painter, frame, &[from, to], at, Stroke::new(FAINT, colour));
    }
    if let Some(plumb) = edge.plumb.filter(|_| !edge.on) {
        segment(painter, frame, plumb, at, Stroke::new(FAINT, CORRECTION));
    }
}

/// A front corner under 90 degrees: an arc between its edges and a ring, red.
fn narrow(painter: &egui::Painter, frame: &Frame, corner: &Corner, at: At<'_>) {
    let centre = at(frame.to_canvas(corner.at));
    let heading = |leg: Point| {
        let far = at(frame.to_canvas([corner.at[0] + leg[0] * 0.05, corner.at[1] + leg[1] * 0.05]));
        (far.y - centre.y).atan2(far.x - centre.x)
    };
    let (from, to) = (heading(corner.legs[0]), heading(corner.legs[1]));
    let mut sweep = to - from;
    while sweep > std::f32::consts::PI {
        sweep -= std::f32::consts::TAU;
    }
    while sweep < -std::f32::consts::PI {
        sweep += std::f32::consts::TAU;
    }
    let arc: Vec<Pos2> = (0..=16)
        .map(|k| {
            let angle = from + sweep * k as f32 / 16.0;
            centre + ARC * egui::vec2(angle.cos(), angle.sin())
        })
        .collect();
    painter.add(Shape::line(arc.clone(), Stroke::new(MARK + HALO, PANEL)));
    painter.add(Shape::line(arc, Stroke::new(MARK, CORRECTION)));
    ring(painter, centre, CORRECTION);
}

/// `e` in `frame`, as a closed line.
fn ellipse(painter: &egui::Painter, frame: &Frame, e: &Ellipse, at: At<'_>, stroke: Stroke) {
    let points: Vec<Pos2> = (e.outline(PIECES).into_iter())
        .map(|p| at(frame.to_canvas(p)))
        .collect();
    painter.add(Shape::line(points, stroke));
}

/// The line from `a` to `b` (frame units) of `frame`.
fn segment(painter: &egui::Painter, frame: &Frame, [a, b]: [Point; 2], at: At<'_>, stroke: Stroke) {
    painter.add(Shape::line_segment(
        [a, b].map(|p| at(frame.to_canvas(p))),
        stroke,
    ));
}

/// The line through `points` (frame units) of `frame`, dashed.
fn dashed(painter: &egui::Painter, frame: &Frame, points: &[Point], at: At<'_>, stroke: Stroke) {
    let path: Vec<Pos2> = points.iter().map(|p| at(frame.to_canvas(*p))).collect();
    painter.extend(Shape::dashed_line(&path, stroke, DASH, GAP));
}

/// A ring at `centre` (screen) over the paper's halo.
fn ring(painter: &egui::Painter, centre: Pos2, colour: egui::Color32) {
    painter.circle_stroke(centre, RING, Stroke::new(MARK + HALO, PANEL));
    painter.circle_stroke(centre, RING, Stroke::new(MARK, colour));
}

fn distance2(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
