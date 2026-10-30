//! A box's, free boxes' or an ellipse's marks over a practice sheet (S84, S97), drawn by the screen
//! through the view as every original is (`practice_marks.rs`), never into the artwork. One frame,
//! the one drawn in (S97): what the exercise gives lies in it in `PRINTED` grey, apart from Draw's
//! ink and Sketch's pencil:
//!
//! - a box to complete: its horizon, labelled, and its front corner's three edges;
//! - free boxes: the horizon;
//! - an ellipse: its frame (a table's cell or a square in perspective).
//!
//! The original's frame beside it stays empty: room on the sheet for what Compare draws beyond the
//! frame. Compared, over the copy, the answer and only what is off: the exact box or ellipse in the
//! marking red as thick as the pen and see-through (a box's hidden edges dashed), a box's
//! vanishing points ringed red; an edge that misses its point drawn on dashed red past it, a
//! leaning vertical's true vertical; free boxes' meeting points ringed red, a front corner under
//! 90 degrees an arc; an ellipse's side middles it falls short of or runs past ringed. An edge
//! that heads for its point gets nothing. The card's Construction lays the way to see it faint:
//! every corner's line to its vanishing points and the horizon run on across the sheet, free
//! boxes' edges run on to the horizon, an ellipse's diagonals and middle lines, its middles dotted.

use egui::{Align2, Color32, FontId, Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{
    BoxCheck, Checked, Corner, Direction, Ellipse, EllipseCheck, EllipseGuide, Frame, FreeCheck,
    Original, Pair, PerspectiveBox, Point,
};

use crate::design::{CORRECTION, DIVIDER, INK_SECONDARY, PANEL, PRINTED};
use crate::text::{self as t, Words};

/// A ring's radius round a vanishing point or a middle, in points; a mark's line and the paper's
/// halo under it; a faint line's.
pub(crate) const RING: f32 = 7.0;
pub(crate) const MARK: f32 = 2.0;
const HALO: f32 = 3.0;
pub(crate) const FAINT: f32 = 1.25;
/// Dashes and gaps, in points.
const DASH: f32 = 7.0;
const GAP: f32 = 5.0;
/// A narrow corner's arc's radius, in points; an ellipse drawn in pieces; a middle's dot.
const ARC: f32 = 22.0;
const PIECES: usize = 96;
const DOT: f32 = 3.0;
/// The horizon's label's size in canvas pixels of a frame.
const LABEL: f32 = 11.0;
/// How far either side of the frame the construction runs the horizon on (frame units): past the
/// farthest vanishing point (`openpaint_practice::FAR`).
const RUN_ON: f64 = 3.5;
/// How strongly the answer's red lies over the copy.
const SEE_THROUGH: f32 = 0.6;

/// Canvas points to the screen.
pub(crate) type At<'a> = &'a dyn Fn([f64; 2]) -> Pos2;

/// Exercise `index`'s givens in its frame, printed grey; an original's line `width` points wide,
/// `scale` points to a canvas pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let frame = &pair.copy;
    let printed = Stroke::new(width, PRINTED);
    match &practice.sheet.originals[index] {
        Original::Box(exact) => {
            labelled(painter, (frame, exact.horizon), at, (scale, PRINTED));
            for (a, b, _) in exact.given() {
                segment(painter, frame, [a, b], at, printed);
            }
        }
        Original::FreeBoxes(example) => {
            labelled(painter, (frame, example.horizon), at, (scale, PRINTED));
        }
        Original::Ellipse(guide) => {
            let corners: Vec<Pos2> = (guide.corners.iter())
                .map(|p| at(frame.to_canvas(*p)))
                .collect();
            painter.add(Shape::closed_line(corners, printed));
        }
        _ => {}
    }
}

/// The horizon at `height` across `frame`, labelled small at its left (a point in space's, a
/// solid's and an object's, in quiet ink).
pub(crate) fn horizon(
    painter: &egui::Painter,
    (frame, height): (&Frame, f64),
    at: At<'_>,
    scale: f32,
) {
    labelled(painter, (frame, height), at, (scale, INK_SECONDARY));
}

/// The horizon at `height` across `frame` in `colour`, labelled small at its left.
fn labelled(
    painter: &egui::Painter,
    (frame, height): (&Frame, f64),
    at: At<'_>,
    (scale, colour): (f32, Color32),
) {
    let ends = [[0.0, height], [1.0, height]];
    segment(painter, frame, ends, at, Stroke::new(FAINT, colour));
    let label = Words::from(t::HORIZON).shown(painter.ctx());
    let place = at(frame.to_canvas([0.01, height - 0.012]));
    let font = FontId::proportional((LABEL * scale).max(6.0));
    painter.text(place, Align2::LEFT_BOTTOM, label, font, colour);
}

/// A box's, free boxes' or an ellipse's comparison over the copy's frame; the answer's line
/// `width` points.
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
    let (on, construction) = ((&frame, at), practice.construction);
    match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Box(check), Original::Box(exact)) => {
            if construction {
                box_construction(painter, on, exact);
            }
            box_compared(painter, on, (exact, check), width);
        }
        (Overlay::FreeBoxes(check), Original::FreeBoxes(example)) => {
            if construction {
                free_construction(painter, on, (example.horizon, check));
            }
            free_compared(painter, on, check);
        }
        (Overlay::Ellipse(check), Original::Ellipse(guide)) => {
            if construction {
                ellipse_construction(painter, on, guide);
            }
            ellipse_compared(painter, on, (guide, check), width);
            // The middles dotted over the red, which runs through them.
            for touch in guide.touches.iter().filter(|_| construction) {
                painter.circle_filled(at(frame.to_canvas(*touch)), DOT, INK_SECONDARY);
            }
        }
        _ => {}
    }
}

/// A frame and how its canvas points reach the screen.
type On<'a> = (&'a Frame, At<'a>);

/// The exact box red over the copy (hidden edges dashed), its vanishing points ringed, and what
/// of the copy's edges is off.
fn box_compared(
    painter: &egui::Painter,
    (frame, at): On<'_>,
    (exact, check): (&PerspectiveBox, &BoxCheck),
    width: f32,
) {
    let red = Stroke::new(width, CORRECTION.gamma_multiply(SEE_THROUGH));
    for ((a, b, _), hidden) in exact.edges().into_iter().zip(exact.hidden()) {
        if hidden {
            dashed(painter, frame, &[a, b], at, red);
        } else {
            segment(painter, frame, [a, b], at, red);
        }
    }
    for point in exact.vanishing.iter().flatten() {
        ring(painter, at(frame.to_canvas(*point)), CORRECTION);
    }
    for edge in &check.edges {
        off(painter, (frame, at), edge);
    }
}

/// Free boxes: where each box's directions meet ringed red, what of the edges is off, a front
/// corner under 90 degrees an arc.
fn free_compared(painter: &egui::Painter, (frame, at): On<'_>, check: &FreeCheck) {
    for free in &check.boxes {
        for (_, meet) in &free.meets {
            ring(painter, at(frame.to_canvas(*meet)), CORRECTION);
        }
        for edge in &free.edges {
            off(painter, (frame, at), edge);
        }
        for corner in &free.narrow {
            narrow(painter, frame, corner, at);
        }
    }
}

/// An ellipse: the exact one red, a ring on each side's middle the copy misses.
fn ellipse_compared(
    painter: &egui::Painter,
    (frame, at): On<'_>,
    (guide, check): (&EllipseGuide, &EllipseCheck),
    width: f32,
) {
    let red = Stroke::new(width, CORRECTION.gamma_multiply(SEE_THROUGH));
    ellipse(painter, frame, &guide.exact, at, red);
    for missed in &check.sides {
        ring(painter, at(frame.to_canvas(missed.middle)), CORRECTION);
    }
}

/// An edge of the copy that misses its point drawn on dashed red past it; a vertical that leans,
/// the true vertical through it. An edge that heads for its point gets nothing.
fn off(painter: &egui::Painter, (frame, at): On<'_>, edge: &Checked) {
    if edge.on {
        return;
    }
    if let Some([from, to]) = edge.extension {
        dashed(
            painter,
            frame,
            &[from, to],
            at,
            Stroke::new(MARK, CORRECTION),
        );
    }
    if let Some(plumb) = edge.plumb {
        segment(painter, frame, plumb, at, Stroke::new(MARK, CORRECTION));
    }
}

/// A box's construction, faint: every corner's line to each vanishing point, the horizon run on
/// across the sheet.
fn box_construction(painter: &egui::Painter, (frame, at): On<'_>, exact: &PerspectiveBox) {
    let faint = Stroke::new(FAINT, DIVIDER);
    run_on(painter, (frame, at), exact.horizon);
    for point in exact.vanishing.iter().flatten() {
        for corner in exact.corners {
            segment(painter, frame, [corner, *point], at, faint);
        }
    }
}

/// Free boxes' construction, faint: each receding edge run on to where its direction meets the
/// horizon, the horizon run on across the sheet.
fn free_construction(
    painter: &egui::Painter,
    (frame, at): On<'_>,
    (horizon, check): (f64, &FreeCheck),
) {
    let faint = Stroke::new(FAINT, DIVIDER);
    run_on(painter, (frame, at), horizon);
    for free in &check.boxes {
        for edge in free
            .edges
            .iter()
            .filter(|e| e.direction != Direction::Vertical)
        {
            let meet = free.meets.iter().find(|(d, _)| *d == edge.direction);
            if let Some((_, meet)) = meet {
                let [a, b] = edge.edge.ends;
                let near = if distance2(a, *meet) < distance2(b, *meet) {
                    a
                } else {
                    b
                };
                segment(painter, frame, [near, *meet], at, faint);
            }
        }
    }
}

/// An ellipse's construction, faint: its frame's diagonals and middle lines (its middles are
/// dotted over the red answer).
fn ellipse_construction(painter: &egui::Painter, (frame, at): On<'_>, guide: &EllipseGuide) {
    let faint = Stroke::new(FAINT, DIVIDER);
    let (c, t) = (guide.corners, guide.touches);
    for line in [[c[0], c[2]], [c[1], c[3]], [t[0], t[2]], [t[1], t[3]]] {
        segment(painter, frame, line, at, faint);
    }
}

/// The horizon at `height` run on faint far either side of `frame`.
fn run_on(painter: &egui::Painter, (frame, at): On<'_>, height: f64) {
    let ends = [[-RUN_ON, height], [1.0 + RUN_ON, height]];
    segment(painter, frame, ends, at, Stroke::new(FAINT, DIVIDER));
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
pub(crate) fn segment(
    painter: &egui::Painter,
    frame: &Frame,
    [a, b]: [Point; 2],
    at: At<'_>,
    stroke: Stroke,
) {
    painter.add(Shape::line_segment(
        [a, b].map(|p| at(frame.to_canvas(p))),
        stroke,
    ));
}

/// The line through `points` (frame units) of `frame`, dashed.
pub(crate) fn dashed(
    painter: &egui::Painter,
    frame: &Frame,
    points: &[Point],
    at: At<'_>,
    stroke: Stroke,
) {
    let path: Vec<Pos2> = points.iter().map(|p| at(frame.to_canvas(*p))).collect();
    painter.extend(Shape::dashed_line(&path, stroke, DASH, GAP));
}

/// A ring at `centre` (screen) over the paper's halo.
pub(crate) fn ring(painter: &egui::Painter, centre: Pos2, colour: egui::Color32) {
    painter.circle_stroke(centre, RING, Stroke::new(MARK + HALO, PANEL));
    painter.circle_stroke(centre, RING, Stroke::new(MARK, colour));
}

fn distance2(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
