//! How a demonstration's hand marks a line (`demo.rs`): a line through two points, a short tick
//! across it, and a copy of it that strays a little as a hand's does; how far it lands off a part
//! on purpose and how Compare marks it (S98); how far an example lies off the frame's middle, and
//! which example of a fixed seed's sequence is clearest; what a lone example shows.

use super::{Drawn, Mark, Marker, On};
use crate::geometry::off::Off;
use crate::maths::path::{Point, distance, length};

/// What a hand draws in the examples a little off a part on purpose (frame units): twice the Draw
/// pen's width, so Compare marks it as the exercise would (S98).
pub(crate) const DRAWN_OFF: f64 = 2.0 * crate::geometry::off::PEN_WIDTH;
/// The half length of a tick that marks how far a line or a silhouette reaches (frame units).
pub(crate) const TICK: f64 = 0.035;

/// The line through `a` and `b`, two points.
pub(crate) fn line(a: Point, b: Point) -> Drawn {
    Drawn::Line(vec![a, b])
}

/// The rectangle from `[left, top]` to `[right, bottom]` as two rows.
pub(crate) fn rect([left, top]: Point, [right, bottom]: Point) -> Vec<[Point; 2]> {
    vec![
        [[left, top], [right, top]],
        [[left, bottom], [right, bottom]],
    ]
}

/// The area between `rows` in `colour`.
pub(crate) fn band(rows: Vec<[Point; 2]>, colour: crate::maths::colour::Rgb8) -> Drawn {
    Drawn::Band {
        rows,
        colour: Some(colour),
    }
}

/// Compare's marks of what is off (`off.rs`) on the copy's frame: a ring in the answer's red, an
/// edge run on dashed red (its point ringed), a true vertical.
pub(crate) fn off_marks(off: &[Off]) -> Vec<(On, Marker, Drawn)> {
    let mut marks = Vec::new();
    for mark in off {
        match *mark {
            Off::Ring(p) => marks.push((On::Copy, Marker::Answer, Drawn::Ring(p))),
            Off::RunOn { line: [a, b], past } => {
                marks.push((On::Copy, Marker::Missed, line(a, b)));
                marks.extend(past.map(|p| (On::Copy, Marker::Answer, Drawn::Ring(p))));
            }
            Off::Plumb([a, b]) => marks.push((On::Copy, Marker::Missed, line(a, b))),
        }
    }
    marks
}

/// How far the middle of `points`' bounds lies from the frame's middle.
pub(crate) fn off_middle(points: &[Point]) -> f64 {
    bounds(points).map_or(0.0, |[low, high]| {
        distance(
            [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0],
            [0.5, 0.5],
        )
    })
}

/// The example a kind shows: of the first `tries` seeds of `seed`'s sequence, the one whose
/// example `score` finds clearest on a small card (the first of the best), so it is fixed.
pub(crate) fn clearest<T>(
    (seed, tries): (u64, u64),
    made: impl Fn(u64) -> T,
    score: impl Fn(&T) -> f64,
) -> T {
    let mut best = made(crate::maths::random::derived(seed, 0));
    let mut best_score = score(&best);
    for k in 1..tries {
        let example = made(crate::maths::random::derived(seed, k));
        let scored = score(&example);
        if scored > best_score {
            (best, best_score) = (example, scored);
        }
    }
    best
}

/// A short tick across the point `at` of a line heading `towards`, `half` long either side.
pub(crate) fn tick(at: Point, towards: Point, half: f64) -> Vec<Point> {
    let d = [towards[0] - at[0], towards[1] - at[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    let n = [-d[1] / long * half, d[0] / long * half];
    vec![[at[0] - n[0], at[1] - n[1]], [at[0] + n[0], at[1] + n[1]]]
}

/// `points` as a hand copies them: moved off sideways by up to `amount` (frame units) in one
/// smooth swell along the line, the ends near their places.
pub(crate) fn wobbled(points: &[Point], amount: f64, phase: f64) -> Vec<Point> {
    let total = length(points).max(f64::MIN_POSITIVE);
    let mut walked = 0.0;
    let mut out = Vec::with_capacity(points.len());
    for (index, p) in points.iter().enumerate() {
        if index > 0 {
            walked += distance(points[index - 1], *p);
        }
        let (before, after) = (
            points[index.saturating_sub(1)],
            points[(index + 1).min(points.len() - 1)],
        );
        let d = [after[0] - before[0], after[1] - before[1]];
        let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
        let s = walked / total;
        let off = amount * (std::f64::consts::PI * s).sin() * (phase + 2.0 * s).cos();
        out.push([p[0] - d[1] / long * off, p[1] + d[0] / long * off]);
    }
    out
}

/// What a lone example shows: on the copy's frame what is given, inked or answered, and every
/// ring (a box's vanishing points).
pub(crate) fn seen(marks: &[Mark]) -> Option<[Point; 2]> {
    let mut points: Vec<Point> = Vec::new();
    for mark in marks.iter().filter(|mark| mark.on == On::Copy) {
        let example = matches!(
            mark.pen,
            Marker::Given
                | Marker::Printed
                | Marker::PrintedHidden
                | Marker::Ink
                | Marker::Answer
                | Marker::Shade
        );
        match &mark.drawn {
            Drawn::Ring(p) => points.push(*p),
            Drawn::Line(line) if example => points.extend(line),
            Drawn::Band { rows, .. } if example => points.extend(rows.iter().flatten()),
            Drawn::Cross(p) | Drawn::Dot(p) if example => points.push(*p),
            _ => {}
        }
    }
    bounds(&points)
}

/// The top left and bottom right of `points`; none when there are none.
pub(crate) fn bounds(points: &[Point]) -> Option<[Point; 2]> {
    let first = *points.first()?;
    Some(points.iter().fold([first, first], |[low, high], p| {
        [
            [low[0].min(p[0]), low[1].min(p[1])],
            [high[0].max(p[0]), high[1].max(p[1])],
        ]
    }))
}
