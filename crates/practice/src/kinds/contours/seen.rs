//! A curve in space as the eye sees it (S156k): its stretches seen and hidden through a view, each
//! change found to the last bit by halving (`curved_rotations/answer.rs`'s runs). A contour's
//! section, and the intersections' lines and curves.

use crate::kinds::curved_rotations::answer::runs;
use crate::kinds::solids::solid_answer::{Stretch, View};
use crate::maths::path::{distance, length};
use crate::maths::vector::Vector;

/// Steps along a curve (frame units: a pixel and a half of a 400-pixel frame), and at least.
const STEP: f64 = 0.004;
const LEAST_STEPS: usize = 32;
/// Pieces a whole curve is drawn in.
const PIECES: f64 = 160.0;
/// A stretch shorter than this (frame units, under a pixel of a 400-pixel frame) is a crumb where
/// a ray grazes an edge: left out.
const CRUMB: f64 = 0.002;
/// A point is seen when nothing lies nearer on its ray than this share of the way.
pub(crate) const SEEN: f64 = 1e-6;

/// How a point of a curve stands: not drawn, hidden, seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Off,
    Hidden,
    Seen,
}

/// The curve `at` over its parameter from 0 to `span` (closing on itself when `closed`) as `view`
/// shows it, in stretches seen and hidden as `state` says; where it is off, nothing.
pub(crate) fn stretches(
    at: &impl Fn(f64) -> Vector,
    (span, closed): (f64, bool),
    view: View,
    state: &impl Fn(f64) -> State,
) -> Vec<Stretch> {
    let wrapped = |t: f64| if closed { t % span } else { t.min(span) };
    let point = |t: f64| view.at(at(wrapped(t)));
    let long: f64 = (0..16)
        .map(|k| {
            let [a, b] = [k, k + 1].map(|i| point(span * f64::from(i) / 16.0));
            distance(a, b)
        })
        .sum();
    let steps = ((long / STEP) as usize).max(LEAST_STEPS);
    (runs(steps, (span, closed), |t| state(wrapped(t))).into_iter())
        .filter(|(_, _, state)| *state != State::Off)
        .map(|(from, to, state)| {
            let pieces = ((to - from) / span * PIECES).ceil().max(2.0) as usize;
            let points = (0..=pieces)
                .map(|k| point(from + (to - from) * k as f64 / pieces as f64))
                .collect();
            Stretch {
                points,
                hidden: state == State::Hidden,
            }
        })
        .filter(|stretch| length(&stretch.points) >= CRUMB)
        .collect()
}
