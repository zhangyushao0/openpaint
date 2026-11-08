//! The road (S89): the course drawn as one hand-drawn line, a stage's sessions as dots along it.
//! What is done is inked (the line up to the last session done, each done session a filled dot),
//! the next session is a ring with a dot in it, and what lies ahead is in pencil (small rings on
//! a dashed line). The Today card and a session's finish show its stage's stretch; the practice
//! book the whole road (`gallery/book.rs`). At a finish the session's dot inks in and two
//! faint rings spread from it, on the interface's spring (D44's acknowledge).

use std::f32::consts::TAU;

use egui::{Pos2, Rect, Stroke};

use openpaint_ui::design::INK;
use openpaint_ui::design::motion::lerp;

use crate::palette::PENCIL;

/// A done session's dot, the next one's ring and a session ahead's, radii in points.
const DONE: f32 = 4.5;
const NEXT: f32 = 7.5;
const AHEAD: f32 = 3.0;
/// The ink line's and the pencil's weights, and the pencil's dashes, in points.
const INK_LINE: f32 = 2.0;
const PENCIL_LINE: f32 = 1.25;
const DASH: (f32, f32) = (6.0, 5.0);
/// Points between the curve's samples.
const STEP: f32 = 4.0;

/// Where a session stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dot {
    Done,
    /// The next, ringed.
    Next,
    Ahead,
    /// Done just now, inked by its progress, 0 to 1.
    Inking(f32),
}

/// The road's height at `x` across `rect`: a gentle wave about its middle, as a hand draws it.
#[must_use]
pub fn curve_y(rect: Rect, x: f32) -> f32 {
    let t = (x - rect.left()) / rect.width().max(1.0);
    let amplitude = rect.height() * 0.16;
    let phase = TAU * 1.15 * t + 0.7;
    rect.center().y + amplitude * (phase.sin() + 0.25 * (2.7 * phase).sin())
}

/// The road from `from` to `to` (x) across `rect`, sampled every few points.
#[must_use]
pub fn curve(rect: Rect, from: f32, to: f32) -> Vec<Pos2> {
    let steps = ((to - from).abs() / STEP).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|step| {
            let x = lerp(from, to, step as f32 / steps as f32);
            Pos2::new(x, curve_y(rect, x))
        })
        .collect()
}

/// `points` in ink.
pub fn inked(painter: &egui::Painter, points: Vec<Pos2>) {
    painter.add(egui::Shape::line(points, Stroke::new(INK_LINE, INK)));
}

/// `points` dashed in pencil.
pub fn pencilled(painter: &egui::Painter, points: &[Pos2]) {
    let stroke = Stroke::new(PENCIL_LINE, PENCIL);
    painter.add(egui::Shape::dashed_line(points, stroke, DASH.0, DASH.1));
}

/// A session's dot at `at`.
pub fn paint_dot(painter: &egui::Painter, at: Pos2, dot: Dot) {
    match dot {
        Dot::Done => {
            painter.circle_filled(at, DONE, INK);
        }
        Dot::Next => {
            painter.circle_stroke(at, NEXT, Stroke::new(1.5, INK));
            painter.circle_filled(at, 2.0, INK);
        }
        Dot::Ahead => {
            painter.circle_stroke(at, AHEAD, Stroke::new(PENCIL_LINE, PENCIL));
        }
        Dot::Inking(progress) => {
            let p = progress.clamp(0.0, 1.0);
            for (reach, strength) in [(22.0, 0.45), (12.0, 0.3)] {
                let ring = Stroke::new(1.25, INK.gamma_multiply(strength * (1.0 - p)));
                painter.circle_stroke(at, NEXT + reach * p, ring);
            }
            painter.circle_stroke(at, lerp(NEXT, DONE, p), Stroke::new(1.5, INK));
            painter.circle_filled(at, DONE * p, INK);
        }
    }
}

/// The dots of a stretch of `count` sessions evenly across `rect` (a little in from its ends).
#[must_use]
pub fn stretch_dots(rect: Rect, count: u32) -> Vec<Pos2> {
    let pad = NEXT + 2.0;
    let count = count.max(2);
    let step = (rect.width() - 2.0 * pad) / (count - 1) as f32;
    (0..count)
        .map(|index| {
            let x = rect.left() + pad + step * index as f32;
            Pos2::new(x, curve_y(rect, x))
        })
        .collect()
}

/// A stage's stretch across `rect`: its sessions, about `of` (more once more are done), `done`
/// of them inked; with `fresh`, the last done inks in by that progress.
pub fn paint_stretch(
    painter: &egui::Painter,
    rect: Rect,
    (done, of): (u32, u32),
    fresh: Option<f32>,
) {
    let count = of.max(done + 1);
    let dots = stretch_dots(rect, count);
    let (first, last) = (dots[0].x, dots[dots.len() - 1].x);
    pencilled(painter, &curve(rect, first, last));
    let done = done as usize;
    if done > 0 {
        let reach = match fresh {
            Some(progress) if done > 1 => {
                lerp(dots[done - 2].x, dots[done - 1].x, progress.clamp(0.0, 1.0))
            }
            _ => dots[done - 1].x,
        };
        inked(painter, curve(rect, first, reach));
    }
    for (index, at) in dots.into_iter().enumerate() {
        let dot = match (index + 1).cmp(&done) {
            std::cmp::Ordering::Less => Dot::Done,
            std::cmp::Ordering::Equal => fresh.map_or(Dot::Done, Dot::Inking),
            std::cmp::Ordering::Greater if index == done => Dot::Next,
            std::cmp::Ordering::Greater => Dot::Ahead,
        };
        paint_dot(painter, at, dot);
    }
}
