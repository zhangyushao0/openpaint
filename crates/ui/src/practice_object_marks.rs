//! An object's marks over a practice sheet (S94, S98), drawn by the screen through the view as
//! every original is (`practice_marks.rs`), never into the artwork, with the lines of
//! `practice_mark_lines.rs` (cut at the sheet's edge): the object's picture filling the
//! original's frame (lit on paper, no lines, no horizon), copied by eye; in the copy's frame the
//! horizon and the box's front corner with its three edges, printed grey (S98).
//!
//! Compared, over the copy: the object's visible edges and outlines in the marking red, as thick
//! as the pen and see-through, and only what is off (`Off`: an edge that turns from the vanishing
//! point it runs to run on past it, one off its place its corners ringed, a round end's middles it
//! misses ringed), kept off the picture's frame (S88's rule for a box's marks beside an original);
//! nothing blue. The card's Construction lays over the picture the whole box in pencil (its hidden
//! edges dashed) and V大's construction in dashed pencil, the corners it finds ringed and a round
//! part's middles dotted, inside the frame.

use egui::{Color32, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Frame, ObjectTask, Original, Pair};

use crate::design::{CORRECTION, INK, INK_SECONDARY, PANEL, PRINTED};
use crate::practice_mark_lines::{
    At, FAINT, MARK, SEE_THROUGH, dashed, horizon, line, marked, segment,
};
use crate::practice_marks::quad;
use crate::practice_paint_marks::PaintedPictures;

/// A ring's radius round a corner found and a middle's dot's, in points.
const RING: f32 = 4.5;
const DOT: f32 = 2.0;

/// Exercise `index`'s picture and givens in its two frames; a given line `width` points wide,
/// `scale` points to a canvas pixel.
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
        let corners = pair.original.corners().map(|p| at.of(p));
        painter.add(quad(texture, corners, Color32::WHITE));
    }
    horizon(painter, (&pair.copy, task.seen.horizon), at, scale);
    for (a, b, _) in task.seen.given() {
        segment(painter, &pair.copy, [a, b], at, Stroke::new(width, PRINTED));
    }
}

/// The box and V大's construction over the picture in `frame`, inside it.
fn constructed(painter: &egui::Painter, frame: &Frame, task: &ObjectTask, at: At<'_>) {
    let inside = egui::Rect::from_points(&frame.corners().map(|p| at.of(p)));
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
        let centre = at.of(frame.to_canvas(point));
        painter.circle_stroke(centre, RING, Stroke::new(MARK + 1.0, PANEL));
        painter.circle_stroke(centre, RING, Stroke::new(MARK, INK));
    }
    for middle in middles {
        painter.circle_filled(at.of(frame.to_canvas(middle)), DOT, INK_SECONDARY);
    }
}

/// An object's comparison over the copy's frame (the box and its construction over the picture
/// while Construction is on); the answer's line `width` points.
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
    let pair = practice.sheet.pairs[index];
    if practice.construction {
        constructed(painter, &pair.original, task, at);
    }
    let frame = pair.copy;
    let red = Stroke::new(width, CORRECTION.gamma_multiply(SEE_THROUGH));
    for side in &task.answer.sides {
        segment(painter, &frame, *side, at, red);
    }
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    for stretch in seen {
        line(painter, &frame, &stretch.points, at, red);
    }
    let kept = at.keeping_off(&pair.original);
    for mark in &check.off {
        marked(painter, &frame, mark, kept);
    }
}
