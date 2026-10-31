//! Stage 3's marks over a practice sheet (S90, S98), drawn by the screen through the view as every
//! original is (`practice_marks.rs`), never into the artwork, with the lines of
//! `practice_mark_lines.rs` (cut at the sheet's edge):
//!
//! - an estimate, copied by eye, keeps its original beside the copy: a point's dot in the
//!   original's frame; a proportion's two segments (the end the share counts from ringed, the
//!   other barred), the original's divided by a dot; an angle's line in the original's frame;
//!   what the copy's frame gives (its segment, the dot an angle's copy runs through) printed grey;
//! - a point in space, built from given parts, has the one frame (S98): its box as the eye sees
//!   it, its horizon, the face shaded and a mirror's given dot, printed grey;
//! - a solid in its box (S93): `practice_solid_marks.rs`.
//!
//! Compared, over the copy: the exact answer in the marking red as thick as the pen and
//! see-through (a cross for a point, a bar across a segment, the line, the uprights or the next
//! face) and only what is off (`Off`: the answer ringed where a mark misses it by more than a
//! pen's width, a line run on where it turns from where it heads, a point that fixes it ringed);
//! nothing blue. The card's Construction lays the way to see it faint: an estimate's in both
//! frames (the point run to the edges with their halves and thirds ticked, the segments' halves
//! and thirds, the lines run on and an upright cross through each), a trick's construction.

use egui::{Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{
    Flat, FlatAnswer, Frame, Original, Pair, PerspectiveBox, Point, SpaceAnswer, SpaceTask,
    across_frame,
};

use crate::design::{CORRECTION, INK, PANEL, PRINTED};
use crate::practice_mark_lines::{At, SEE_THROUGH, faint, horizon, marked, segment};
use crate::practice_solid_marks as solid_marks;

/// A given dot's radius in canvas pixels of a frame, and the least it shows at, in points.
const DOT: f32 = 4.5;
const LEAST_DOT: f32 = 2.0;
/// Frame units: a point's cross's arm, a bar across a segment's half, a tick's half (at a half,
/// at a third), a cross's arm through an angle's line.
const CROSS: f64 = 0.03;
const BAR: f64 = 0.035;
const HALF_TICK: f64 = 0.03;
const THIRD_TICK: f64 = 0.018;
const CROSS_ARM: f64 = 0.1;
/// How strongly the face is shaded, printed.
const SHADE: f32 = 0.22;

/// Exercise `index`'s givens: an estimate's in its two frames, a point in space's in its one;
/// an original's line `width` points wide, `scale` points to a canvas pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let radius = (DOT * scale).max(LEAST_DOT);
    let (ink, printed) = (Stroke::new(width, INK), Stroke::new(width, PRINTED));
    match &practice.sheet.originals[index] {
        Original::Flat(flat) => match *flat {
            Flat::Point { at: p } => dot(painter, at.of(pair.original.to_canvas(p)), (radius, INK)),
            Flat::Proportion {
                original,
                share,
                copy,
            } => {
                barred(painter, &pair.original, original, at, ink);
                barred(painter, &pair.copy, copy, at, printed);
                let divided = along(original, share);
                dot(
                    painter,
                    at.of(pair.original.to_canvas(divided)),
                    (radius, INK),
                );
            }
            Flat::Angle { original, through } => {
                segment(painter, &pair.original, original, at, ink);
                dot(
                    painter,
                    at.of(pair.copy.to_canvas(through)),
                    (radius, PRINTED),
                );
            }
        },
        Original::Space(task) => {
            let frame = &pair.copy;
            spaced(painter, frame, task, at, (width, scale));
            if let Some(given) = task.given_at() {
                dot(painter, at.of(frame.to_canvas(given)), (radius, PRINTED));
            }
        }
        Original::Solid(_) => {
            solid_marks::show_original(painter, practice, (index, pair), at, (width, scale));
        }
        _ => {}
    }
}

/// An estimate's or a point in space's comparison over the copy's frame (the way to see it under
/// it while Construction is on); the answer's line `width` points.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (At<'_>, f32),
) {
    let Some(index) = practice.current else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    let frame = pair.copy;
    let red = Stroke::new(width, CORRECTION.gamma_multiply(SEE_THROUGH));
    let off = match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Flat(check), Original::Flat(flat)) => {
            if practice.construction {
                way(painter, &pair.original, flat, at, false);
                way(painter, &frame, flat, at, true);
            }
            match (flat.answer(), flat) {
                (FlatAnswer::Point(p), Flat::Proportion { copy, .. }) => {
                    let [nx, ny] = normal(*copy);
                    let ends = [-BAR, BAR].map(|s| [p[0] + s * nx, p[1] + s * ny]);
                    segment(painter, &frame, ends, at, red);
                }
                (FlatAnswer::Point(p), _) => cross(painter, &frame, p, at, red),
                (FlatAnswer::Line(line), _) => segment(painter, &frame, line, at, red),
            }
            &check.off
        }
        (Overlay::Space(check), Original::Space(task)) => {
            if practice.construction {
                for line in task.construction() {
                    segment(painter, &frame, line, at, faint());
                }
            }
            match task.answer() {
                SpaceAnswer::Point(p) => cross(painter, &frame, p, at, red),
                SpaceAnswer::Lines(lines) => {
                    for line in lines {
                        segment(painter, &frame, line, at, red);
                    }
                }
            }
            &check.off
        }
        _ => return,
    };
    for mark in off {
        marked(painter, &frame, mark, at);
    }
}

/// The way to see an estimate's answer in `frame`, faint: in the copy's (`copy`) at its answer, in
/// the original's at its givens.
fn way(painter: &egui::Painter, frame: &Frame, flat: &Flat, at: At<'_>, copy: bool) {
    match (*flat, flat.answer()) {
        (Flat::Point { at: p }, _) => {
            segment(painter, frame, [[0.0, p[1]], [1.0, p[1]]], at, faint());
            segment(painter, frame, [[p[0], 0.0], [p[0], 1.0]], at, faint());
            for share in [0.5, 1.0 / 3.0, 2.0 / 3.0] {
                let tick = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
                segment(painter, frame, [[share, 0.0], [share, tick]], at, faint());
                segment(painter, frame, [[0.0, share], [tick, share]], at, faint());
            }
        }
        (
            Flat::Proportion {
                original,
                copy: given,
                ..
            },
            _,
        ) => ticked(painter, frame, if copy { given } else { original }, at),
        (Flat::Angle { original, through }, FlatAnswer::Line(line)) => {
            let (middle, [a, b]) = if copy {
                (through, line)
            } else {
                (along(original, 0.5), original)
            };
            if let Some(run) = across_frame(a, [b[0] - a[0], b[1] - a[1]]) {
                segment(painter, frame, run, at, faint());
            }
            for d in [[CROSS_ARM, 0.0], [0.0, CROSS_ARM]] {
                let ends = [-1.0, 1.0].map(|s| [middle[0] + s * d[0], middle[1] + s * d[1]]);
                segment(painter, frame, ends, at, faint());
            }
        }
        (Flat::Angle { .. }, FlatAnswer::Point(_)) => {}
    }
}

/// The box of `task` in `frame`, printed grey: its face shaded, the horizon, the edges the eye
/// sees (the hidden ones left out: they would cross the face a point is fixed on).
fn spaced(
    painter: &egui::Painter,
    frame: &Frame,
    task: &SpaceTask,
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let face: Vec<Pos2> = task
        .corners
        .iter()
        .map(|p| at.of(frame.to_canvas(*p)))
        .collect();
    let shade = PRINTED.gamma_multiply(SHADE);
    painter.add(Shape::convex_polygon(face, shade, Stroke::NONE));
    horizon(painter, (frame, task.seen.horizon), at, scale);
    box_edges(painter, frame, &task.seen, at, width);
}

/// `seen`'s edges the eye sees, in `frame`, printed grey.
fn box_edges(
    painter: &egui::Painter,
    frame: &Frame,
    seen: &PerspectiveBox,
    at: At<'_>,
    width: f32,
) {
    for ((a, b, _), hidden) in seen.edges().into_iter().zip(seen.hidden()) {
        if !hidden {
            segment(painter, frame, [a, b], at, Stroke::new(width, PRINTED));
        }
    }
}

/// `segment` in `frame` with a short bar across its last end and its first ringed (the share is
/// counted from it, in both frames alike).
fn barred(painter: &egui::Painter, frame: &Frame, line: [Point; 2], at: At<'_>, stroke: Stroke) {
    segment(painter, frame, line, at, stroke);
    let [nx, ny] = normal(line);
    let [first, last] = line;
    let bar = [-1.0, 1.0].map(|s| [last[0] + s * THIRD_TICK * nx, last[1] + s * THIRD_TICK * ny]);
    segment(painter, frame, bar, at, stroke);
    let centre = at.of(frame.to_canvas(first));
    let radius = centre.distance(at.of(frame.to_canvas([first[0] + THIRD_TICK, first[1]])));
    painter.circle(centre, radius, PANEL, stroke);
}

/// `line`'s halves and thirds ticked across it, faint.
fn ticked(painter: &egui::Painter, frame: &Frame, line: [Point; 2], at: At<'_>) {
    let [nx, ny] = normal(line);
    for share in [0.5, 1.0 / 3.0, 2.0 / 3.0] {
        let half = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
        let p = along(line, share);
        let tick = [-1.0, 1.0].map(|s| [p[0] + s * half * nx, p[1] + s * half * ny]);
        segment(painter, frame, tick, at, faint());
    }
}

/// A cross at `p` (frame units) in `frame`.
fn cross(painter: &egui::Painter, frame: &Frame, p: Point, at: At<'_>, stroke: Stroke) {
    let arms = [[CROSS, 0.0], [0.0, CROSS]];
    for [dx, dy] in arms {
        segment(
            painter,
            frame,
            [[p[0] - dx, p[1] - dy], [p[0] + dx, p[1] + dy]],
            at,
            stroke,
        );
    }
}

/// A dot of `radius` points at `centre`, in `colour`.
fn dot(painter: &egui::Painter, centre: Pos2, (radius, colour): (f32, egui::Color32)) {
    painter.circle_filled(centre, radius, colour);
}

/// The point `share` of the way along `line`.
fn along([a, b]: [Point; 2], share: f64) -> Point {
    [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share]
}

/// `line`'s unit normal.
fn normal([a, b]: [Point; 2]) -> Point {
    let d = [b[0] - a[0], b[1] - a[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    [-d[1] / long, d[0] / long]
}
