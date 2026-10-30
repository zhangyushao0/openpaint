//! An object's marks over a practice sheet (S94), drawn by the screen through the view as every
//! original is (`practice_marks.rs`), never into the artwork: the object's picture filling the
//! original's frame (lit on paper, no lines, no horizon); in the copy's frame the horizon and the
//! box's front corner with its three edges (S84's given). Once compared, the original's frame
//! keeps over the picture the whole box in pencil (its hidden edges dashed) and V大's construction
//! in dashed pencil, the corners it finds ringed and a round part's middles dotted, inside the
//! frame.
//!
//! Compared, over the copy: the object's visible edges and outlines in the marking red, as thick
//! as the pen and see-through; what was read of the copy dashed in `FOUND` blue (each edge's
//! pieces, a round part's curves, fitted once drawn two thirds round).

use egui::{Color32, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Frame, ObjectTask, Original, Pair, Point};

use crate::design::{CORRECTION, FOUND, INK, INK_SECONDARY, PANEL};
use crate::practice_geometry_marks::{At, FAINT, MARK, dashed, horizon, segment};
use crate::practice_marks::quad;
use crate::practice_paint_marks::PaintedPictures;

/// A ring's radius round a corner found and a middle's dot's, in points; how strongly the red
/// answer lies over the copy (as a solid's does); an ellipse fitted drawn in pieces.
const RING: f32 = 4.5;
const DOT: f32 = 2.0;
const RED: f32 = 0.5;
const PIECES: usize = 96;

/// Exercise `index`'s picture and givens in its two frames and, once compared, the box and the
/// construction over the picture; a given line `width` points wide, `scale` points to a canvas
/// pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    (at, widths): (At<'_>, (f32, f32)),
    painted: &PaintedPictures,
) {
    let Original::Object(task) = &practice.sheet.originals[index] else {
        return;
    };
    let (width, scale) = widths;
    if let Some(texture) = painted.of(practice, index) {
        let corners = pair.original.corners().map(at);
        painter.add(quad(texture, corners, Color32::WHITE));
    }
    horizon(painter, (&pair.copy, task.seen.horizon), at, scale);
    for (a, b, _) in task.seen.given() {
        segment(painter, &pair.copy, [a, b], at, Stroke::new(width, INK));
    }
    if practice.current == Some(index) && practice.result.is_some() {
        constructed(painter, &pair.original, task, at);
    }
}

/// The box and V大's construction over the picture in `frame`, inside it.
fn constructed(painter: &egui::Painter, frame: &Frame, task: &ObjectTask, at: At<'_>) {
    let inside = egui::Rect::from_points(&frame.corners().map(at));
    let painter = &painter.with_clip_rect(inside.intersect(painter.clip_rect()));
    let pencil = Stroke::new(FAINT, INK_SECONDARY);
    for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
        if hidden {
            dashed(painter, frame, &[a, b], at, pencil);
        } else {
            segment(painter, frame, [a, b], at, pencil);
        }
    }
    for line in task.construction() {
        dashed(painter, frame, &line, at, pencil);
    }
    let (found, middles) = task.found();
    for point in found {
        let centre = at(frame.to_canvas(point));
        painter.circle_stroke(centre, RING, Stroke::new(MARK + 1.0, PANEL));
        painter.circle_stroke(centre, RING, Stroke::new(MARK, INK));
    }
    for middle in middles {
        painter.circle_filled(at(frame.to_canvas(middle)), DOT, INK_SECONDARY);
    }
}

/// An object's comparison over the copy's frame; the answer's line `width` points.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (At<'_>, f32),
) {
    let Some(index) = practice.current else {
        return;
    };
    let (Overlay::Object(check), Original::Object(task)) =
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
    for side in &task.answer.sides {
        segment(painter, &frame, *side, at, red);
    }
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    for stretch in seen {
        line(painter, &frame, &stretch.points, at, red);
    }
}

/// The line through `points` (frame units) of `frame`.
fn line(painter: &egui::Painter, frame: &Frame, points: &[Point], at: At<'_>, stroke: Stroke) {
    let path = points.iter().map(|p| at(frame.to_canvas(*p))).collect();
    painter.add(Shape::line(path, stroke));
}
