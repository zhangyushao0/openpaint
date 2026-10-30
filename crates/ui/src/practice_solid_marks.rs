//! A solid's marks over a practice sheet (S93), drawn by the screen through the view as every
//! original is (`practice_marks.rs`), never into the artwork: the box whole in ink in both frames
//! (its hidden edges dashed) and its horizon; once compared, the original's frame keeps V大's
//! construction in dashed pencil (the diagonals, the lines through the centres, the middles and a
//! cone's point dotted, a sphere's box halved and its middle section's ellipse) and the solid in
//! ink, its hidden part dashed, inside the frame.
//!
//! Compared, over the copy: the exact solid in the marking red, as thick as the pen and
//! see-through, its hidden part dashed; what was read of the copy dashed in `FOUND` blue: each
//! ellipse fitted (or its curves as drawn, while they go less than two thirds round) and each
//! side's pieces.

use egui::{Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Frame, Original, Pair, Point, Solid, SolidTask};

use crate::design::{CORRECTION, FOUND, INK, INK_SECONDARY};
use crate::practice_geometry_marks::{At, FAINT, MARK, dashed, horizon, segment};

/// An ellipse drawn in pieces; a middle's dot's radius, in points; how strongly the red answer
/// lies over the copy (as a point's does).
const PIECES: usize = 96;
const DOT: f32 = 2.5;
const RED: f32 = 0.5;

/// Exercise `index`'s box in its two frames and, once compared, the construction and the solid in
/// the original's; an original's line `width` points wide, `scale` points to a canvas pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let Original::Solid(task) = &practice.sheet.originals[index] else {
        return;
    };
    for frame in [&pair.original, &pair.copy] {
        horizon(painter, (frame, task.seen.horizon), at, scale);
        for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
            if hidden {
                dashed(painter, frame, &[a, b], at, Stroke::new(width / 2.0, INK));
            } else {
                segment(painter, frame, [a, b], at, Stroke::new(width, INK));
            }
        }
    }
    if practice.current == Some(index) && practice.result.is_some() {
        constructed(painter, &pair.original, task, at, width);
    }
}

/// V大's construction in dashed pencil and the solid in ink, inside `frame`.
fn constructed(painter: &egui::Painter, frame: &Frame, task: &SolidTask, at: At<'_>, width: f32) {
    let inside = egui::Rect::from_points(&frame.corners().map(at));
    let painter = &painter.with_clip_rect(inside.intersect(painter.clip_rect()));
    let pencil = Stroke::new(FAINT, INK_SECONDARY);
    for line in task.construction() {
        dashed(painter, frame, &line, at, pencil);
    }
    if task.solid == Solid::Sphere
        && let Some(section) = task.squares.first()
    {
        dashed(painter, frame, &section.exact.outline(PIECES), at, pencil);
    }
    for dot in task.middles().into_iter().chain(task.point) {
        painter.circle_filled(at(frame.to_canvas(dot)), DOT, INK_SECONDARY);
    }
    solid(painter, frame, task, at, Stroke::new(width, INK));
}

/// `task`'s solid in `frame` in `stroke`: its outlines, hidden stretches dashed, and its sides.
fn solid(painter: &egui::Painter, frame: &Frame, task: &SolidTask, at: At<'_>, stroke: Stroke) {
    for stretch in task.answer.outlines.iter().flatten() {
        if stretch.hidden {
            dashed(painter, frame, &stretch.points, at, stroke);
        } else {
            line(painter, frame, &stretch.points, at, stroke);
        }
    }
    for side in &task.answer.sides {
        segment(painter, frame, *side, at, stroke);
    }
}

/// A solid's comparison over the copy's frame; the answer's line `width` points.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (At<'_>, f32),
) {
    let Some(index) = practice.current else {
        return;
    };
    let (Overlay::Solid(check), Original::Solid(task)) =
        (overlay, &practice.sheet.originals[index])
    else {
        return;
    };
    let frame = practice.sheet.pairs[index].copy;
    // What was read first, under the answer: the red lies over it as over the copy.
    let blue = Stroke::new(MARK, FOUND);
    for curve in &check.curves {
        match &curve.fitted {
            Some(fitted) => dashed(painter, &frame, &fitted.outline(PIECES), at, blue),
            None => {
                for piece in &curve.pieces {
                    dashed(painter, &frame, piece, at, blue);
                }
            }
        }
    }
    for piece in check.sides.iter().flatten() {
        dashed(painter, &frame, piece, at, blue);
    }
    let red = Stroke::new(width, CORRECTION.gamma_multiply(RED));
    solid(painter, &frame, task, at, red);
}

/// The line through `points` (frame units) of `frame`.
fn line(painter: &egui::Painter, frame: &Frame, points: &[Point], at: At<'_>, stroke: Stroke) {
    let path = points.iter().map(|p| at(frame.to_canvas(*p))).collect();
    painter.add(Shape::line(path, stroke));
}
