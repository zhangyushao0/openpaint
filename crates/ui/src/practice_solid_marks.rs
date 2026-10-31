//! A solid's marks over a practice sheet (S93, S98), drawn by the screen through the view as every
//! original is (`practice_marks.rs`), never into the artwork, with the lines of
//! `practice_mark_lines.rs` (cut at the sheet's edge): in its one frame (S98) the box whole printed
//! grey (its hidden edges dashed) and its horizon.
//!
//! Compared, over the copy: the exact solid in the marking red, as thick as the pen and
//! see-through, its hidden part dashed, and only what is off (`Off`): a middle an end, a base or
//! a sphere's outline misses by more than a pen's width ringed, a side that turns from where it
//! heads run on past it (a cone's point ringed), one off its place its touches ringed; nothing
//! blue. The card's Construction lays V大's construction faint (the diagonals, the lines through
//! the centres, a sphere's box halved and its section's ellipse) with the middles and a cone's
//! point dotted.

use egui::Stroke;
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Frame, Original, Pair, Solid, SolidTask};

use crate::design::{CORRECTION, INK_SECONDARY, PRINTED};
use crate::practice_geometry_marks::{DOT, ellipse};
use crate::practice_mark_lines::{At, SEE_THROUGH, dashed, faint, horizon, line, marked, segment};

/// Exercise `index`'s box in its one frame, printed grey; an original's line `width` points wide,
/// `scale` points to a canvas pixel.
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
    let frame = &pair.copy;
    horizon(painter, (frame, task.seen.horizon), at, scale);
    for ((a, b, _), hidden) in task.seen.edges().into_iter().zip(task.seen.hidden()) {
        if hidden {
            dashed(
                painter,
                frame,
                &[a, b],
                at,
                Stroke::new(width / 2.0, PRINTED),
            );
        } else {
            segment(painter, frame, [a, b], at, Stroke::new(width, PRINTED));
        }
    }
}

/// V大's construction faint, its middles and a cone's point dotted.
fn construction(painter: &egui::Painter, frame: &Frame, task: &SolidTask, at: At<'_>) {
    for line in task.construction() {
        segment(painter, frame, line, at, faint());
    }
    if task.solid == Solid::Sphere
        && let Some(section) = task.squares.first()
    {
        ellipse(painter, frame, &section.exact, at, faint());
    }
    for dot in task.middles().into_iter().chain(task.point) {
        painter.circle_filled(at.of(frame.to_canvas(dot)), DOT, INK_SECONDARY);
    }
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

/// A solid's comparison over the copy's frame (its construction under it while Construction is
/// on); the answer's line `width` points.
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
    if practice.construction {
        construction(painter, &frame, task, at);
    }
    let red = Stroke::new(width, CORRECTION.gamma_multiply(SEE_THROUGH));
    solid(painter, &frame, task, at, red);
    for mark in &check.off {
        marked(painter, &frame, mark, at);
    }
}
