//! Stage 3's marks over a practice sheet (S90), drawn by the screen through the view as every
//! original is (`practice_marks.rs`), never into the artwork:
//!
//! - an estimate: a point's dot in the original's frame; a proportion's two segments (the end the
//!   share counts from ringed, the other barred) in both frames, the original's divided by a dot;
//!   an angle's line in the original's frame and the dot its copy runs through in the copy's;
//!   once compared, the original's frame
//!   keeps the way to see it in dashed pencil: the point run across and down to the frame's edges
//!   with the halves and thirds ticked on them, the segment's halves and thirds, the line run on
//!   to the frame's edges with a cross through its middle;
//! - a point in space: the box as the eye sees it in ink in both frames, its horizon, the
//!   face shaded in pencil, a mirror's given dot; once compared, the original's frame shows the
//!   trick's construction in dashed pencil and its answer in ink, inside the frame.
//!
//! Compared, over the copy: the exact answer in the marking red as thick as the pen (a cross for a
//! point, a bar across a segment, the line, the uprights or the next face), an estimate's way to
//! see it in dashed pencil as in the original's frame, and what was read of the copy in `FOUND`
//! blue: the mark ringed, the lines dashed (an angle's run on to the frame's edges).

use egui::{Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{
    Flat, FlatAnswer, Frame, Original, Pair, PerspectiveBox, Point, SpaceAnswer, SpaceTask,
    across_frame,
};

use crate::design::{CORRECTION, FOUND, INK, INK_SECONDARY, PANEL, PENCIL};
use crate::practice_geometry_marks::{At, FAINT, MARK, RING, dashed, horizon, segment};

/// A given dot's radius in canvas pixels of a frame, and the least it shows at, in points.
const DOT: f32 = 4.5;
const LEAST_DOT: f32 = 2.0;
/// Frame units: a point's cross's arm, a bar across a segment's half, a tick's half (at a half,
/// at a third), a cross's arm through an angle's line; how strongly the face is shaded.
const CROSS: f64 = 0.03;
const BAR: f64 = 0.035;
const HALF_TICK: f64 = 0.03;
const THIRD_TICK: f64 = 0.018;
const CROSS_ARM: f64 = 0.1;
const SHADE: f32 = 0.45;
/// How strongly the red answer lies over the copy (as a line's original does).
const RED: f32 = 0.5;

/// Exercise `index`'s givens in its two frames; an original's line `width` points wide, `scale`
/// points to a canvas pixel.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: At<'_>,
    (width, scale): (f32, f32),
) {
    let answered = practice.current == Some(index) && practice.result.is_some();
    let radius = (DOT * scale).max(LEAST_DOT);
    let ink = Stroke::new(width, INK);
    match &practice.sheet.originals[index] {
        Original::Flat(flat) => {
            match *flat {
                Flat::Point { at: p } => dot(painter, at(pair.original.to_canvas(p)), radius),
                Flat::Proportion {
                    original,
                    share,
                    copy,
                } => {
                    barred(painter, &pair.original, original, at, ink);
                    barred(painter, &pair.copy, copy, at, ink);
                    let divided = along(original, share);
                    dot(painter, at(pair.original.to_canvas(divided)), radius);
                }
                Flat::Angle { original, through } => {
                    segment(painter, &pair.original, original, at, ink);
                    dot(painter, at(pair.copy.to_canvas(through)), radius);
                }
            }
            if answered {
                way(painter, &pair.original, flat, at, false);
            }
        }
        Original::Space(task) => {
            for frame in [&pair.original, &pair.copy] {
                spaced(painter, frame, task, at, (width, scale));
                if let Some(given) = task.given_at() {
                    dot(painter, at(frame.to_canvas(given)), radius);
                }
            }
            if answered {
                constructed(painter, &pair.original, task, at, width);
            }
        }
        _ => {}
    }
}

/// An estimate's or a point in space's comparison over the copy's frame; the answer's line
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
    let red = Stroke::new(width, CORRECTION.gamma_multiply(RED));
    let blue = Stroke::new(MARK, FOUND);
    // What was read first, under the answer: the red lies over it as over the copy.
    let read = |mark: Option<Point>| {
        if let Some(mark) = mark {
            painter.circle_stroke(at(frame.to_canvas(mark)), RING, blue);
        }
    };
    match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Flat(check), Original::Flat(flat)) => {
            way(painter, &frame, flat, at, true);
            read(check.mark);
            let run = (check.line).and_then(|edge| across_frame(edge.middle(), edge.direction()));
            if let Some(run) = run {
                dashed(painter, &frame, &run, at, blue);
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
        }
        (Overlay::Space(check), Original::Space(task)) => {
            read(check.mark);
            for edge in &check.lines {
                dashed(painter, &frame, &edge.ends, at, blue);
            }
            match task.answer() {
                SpaceAnswer::Point(p) => cross(painter, &frame, p, at, red),
                SpaceAnswer::Lines(lines) => {
                    for line in lines {
                        segment(painter, &frame, line, at, red);
                    }
                }
            }
        }
        _ => {}
    }
}

/// The way to see an estimate's answer in `frame`, in dashed pencil: in the copy's (`copy`) at
/// its answer, in the original's at its givens.
fn way(painter: &egui::Painter, frame: &Frame, flat: &Flat, at: At<'_>, copy: bool) {
    let pencil = Stroke::new(FAINT, INK_SECONDARY);
    match (*flat, flat.answer()) {
        (Flat::Point { at: p }, _) => {
            dashed(painter, frame, &[[0.0, p[1]], [1.0, p[1]]], at, pencil);
            dashed(painter, frame, &[[p[0], 0.0], [p[0], 1.0]], at, pencil);
            for share in [0.5, 1.0 / 3.0, 2.0 / 3.0] {
                let tick = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
                let solid = Stroke::new(FAINT, INK_SECONDARY);
                segment(painter, frame, [[share, 0.0], [share, tick]], at, solid);
                segment(painter, frame, [[0.0, share], [tick, share]], at, solid);
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
                dashed(painter, frame, &run, at, pencil);
            }
            for d in [[CROSS_ARM, 0.0], [0.0, CROSS_ARM]] {
                let ends = [-1.0, 1.0].map(|s| [middle[0] + s * d[0], middle[1] + s * d[1]]);
                dashed(painter, frame, &ends, at, pencil);
            }
        }
        (Flat::Angle { .. }, FlatAnswer::Point(_)) => {}
    }
}

/// The box of `task` in `frame`: its face shaded in pencil, the horizon, the edges the eye sees in
/// ink (the hidden ones left out: they would cross the face a point is fixed on).
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
        .map(|p| at(frame.to_canvas(*p)))
        .collect();
    let shade = PENCIL.gamma_multiply(SHADE);
    painter.add(Shape::convex_polygon(face, shade, Stroke::NONE));
    horizon(painter, (frame, task.seen.horizon), at, scale);
    box_edges(painter, frame, &task.seen, at, width);
}

/// `seen`'s edges the eye sees, in `frame`.
fn box_edges(
    painter: &egui::Painter,
    frame: &Frame,
    seen: &PerspectiveBox,
    at: At<'_>,
    width: f32,
) {
    for ((a, b, _), hidden) in seen.edges().into_iter().zip(seen.hidden()) {
        if !hidden {
            segment(painter, frame, [a, b], at, Stroke::new(width, INK));
        }
    }
}

/// A trick's construction in dashed pencil and its answer in ink, inside `frame`.
fn constructed(painter: &egui::Painter, frame: &Frame, task: &SpaceTask, at: At<'_>, width: f32) {
    let inside = egui::Rect::from_points(&frame.corners().map(at));
    let painter = &painter.with_clip_rect(inside.intersect(painter.clip_rect()));
    let pencil = Stroke::new(FAINT, INK_SECONDARY);
    for line in task.construction() {
        dashed(painter, frame, &line, at, pencil);
    }
    let ink = Stroke::new(width, INK);
    match task.answer() {
        SpaceAnswer::Point(p) => cross(painter, frame, p, at, ink),
        SpaceAnswer::Lines(lines) => {
            for line in lines {
                segment(painter, frame, line, at, ink);
            }
        }
    }
}

/// `segment` in `frame` with a short bar across each end.
fn barred(painter: &egui::Painter, frame: &Frame, line: [Point; 2], at: At<'_>, stroke: Stroke) {
    segment(painter, frame, line, at, stroke);
    let [nx, ny] = normal(line);
    let [first, last] = line;
    let bar = [-1.0, 1.0].map(|s| [last[0] + s * THIRD_TICK * nx, last[1] + s * THIRD_TICK * ny]);
    segment(painter, frame, bar, at, stroke);
    // The first end ringed, in both frames alike: the share is counted from it.
    let centre = at(frame.to_canvas(first));
    let radius = centre.distance(at(frame.to_canvas([first[0] + THIRD_TICK, first[1]])));
    painter.circle(centre, radius, PANEL, stroke);
}

/// `line`'s halves and thirds ticked across it in pencil.
fn ticked(painter: &egui::Painter, frame: &Frame, line: [Point; 2], at: At<'_>) {
    let [nx, ny] = normal(line);
    for share in [0.5, 1.0 / 3.0, 2.0 / 3.0] {
        let half = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
        let p = along(line, share);
        let tick = [-1.0, 1.0].map(|s| [p[0] + s * half * nx, p[1] + s * half * ny]);
        segment(painter, frame, tick, at, Stroke::new(FAINT, INK_SECONDARY));
    }
}

/// A cross at `p` (frame units) in `frame`.
fn cross(painter: &egui::Painter, frame: &Frame, p: Point, at: At<'_>, stroke: Stroke) {
    segment(
        painter,
        frame,
        [[p[0] - CROSS, p[1]], [p[0] + CROSS, p[1]]],
        at,
        stroke,
    );
    segment(
        painter,
        frame,
        [[p[0], p[1] - CROSS], [p[0], p[1] + CROSS]],
        at,
        stroke,
    );
}

/// A dot of `radius` points at `centre`, in ink.
fn dot(painter: &egui::Painter, centre: Pos2, radius: f32) {
    painter.circle_filled(centre, radius, INK);
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
