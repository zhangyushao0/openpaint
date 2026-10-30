//! Compare for solids (S93): the copy's Draw and Correct lines (Sketch's construction never
//! reaches here) read against the exact answer (`solid_answer.rs`). Each line is split into its
//! straight pieces (`edges.rs`) and its curves; a piece or a curve's point goes to the part of the
//! answer nearest it (an end's, a base's or the outline's ellipse, or a side), none farther than
//! `FAR`. What an ellipse holds is fitted as one (Fitzgibbon's direct fit, `ellipse_fit.rs`) once
//! it goes `COVERED` round it, else shown as drawn; a side's pieces as drawn. A correction counts
//! in place of what it redraws: a part the corrections reach reads them alone.
//!
//! The first Compare keeps the read lines' mean distance from the answer, a share of the box
//! (`Metric::SolidOff`), never shown. Frame units.
//!
//! An object's copy (S94) is read alike against its visible edges and outlines (`ObjectOff`): its
//! round part's ends only where seen, as only those are drawn.

use crate::edges::edges;
use crate::ellipse::Ellipse;
use crate::ellipse_fit::fit_ellipse;
use crate::metric::{Metric, Metrics};
use crate::object::ObjectTask;
use crate::path::{Point, distance, length, nearest, nearest_on_segment, resample};
use crate::solid::SolidTask;
use crate::solid_answer::SolidAnswer;

/// The even step lines are read at.
const SPACING: f64 = 0.004;
/// A point within this of one of its line's straight pieces lies on it.
const ON_EDGE: f64 = 0.008;
/// What lies farther than this from every part of the answer is left out (40 pixels of a
/// 400-pixel frame): a scribble, a box's edge traced with Draw.
const FAR: f64 = 0.1;
/// What is read for an ellipse is fitted once it goes this share round it (two thirds, where the
/// direct fit finds a hand's ellipse within a few hundredths, S84), counted in `BINS` arcs.
pub const COVERED: f64 = 2.0 / 3.0;
const BINS: usize = 24;
/// Corrections redraw a part once they hold this much of it (20 pixels of a 400-pixel frame).
const REDRAWN: f64 = 0.05;

/// What was read for one of the answer's ellipses: the curves drawn nearest it, and the ellipse
/// fitted to them once they go far enough round.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReadCurve {
    pub pieces: Vec<Vec<Point>>,
    pub fitted: Option<Ellipse>,
}

/// A solid's copy as Compare read it: per ellipse of the answer and per side, and how far off.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidCheck {
    pub curves: Vec<ReadCurve>,
    /// Each side's straight pieces (or curves) drawn nearest it.
    pub sides: Vec<Vec<Vec<Point>>>,
    /// The read lines' mean distance from the answer, a share of the box; none when nothing was
    /// read.
    pub metrics: Metrics,
}

/// The part of the answer a piece belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Ellipse(usize),
    Side(usize),
}

/// The pieces read per part.
#[derive(Debug, Default)]
struct Parts {
    curves: Vec<Vec<Vec<Point>>>,
    sides: Vec<Vec<Vec<Point>>>,
}

impl SolidCheck {
    /// `task`'s copy drawn as `drawn` before the first Compare and `corrections` after it (Draw
    /// and Correct, oldest first) read and measured.
    #[must_use]
    pub fn of(task: &SolidTask, lines: (&[&[Point]], &[&[Point]])) -> Self {
        Self::against(&task.answer, (task.size(), Metric::SolidOff), lines)
    }

    /// An object's copy (S94) read so against its visible edges and outlines.
    #[must_use]
    pub fn of_object(task: &ObjectTask, lines: (&[&[Point]], &[&[Point]])) -> Self {
        Self::against(&task.answer, (task.size(), Metric::ObjectOff), lines)
    }

    /// The copy read against `answer`, its error `metric`, a share of `size`.
    fn against(
        answer: &SolidAnswer,
        (size, metric): (f64, Metric),
        (drawn, corrections): (&[&[Point]], &[&[Point]]),
    ) -> Self {
        let (first, later) = (read(answer, drawn), read(answer, corrections));
        // A part the corrections redraw reads them alone; a bit of a correction near a part it
        // only passes (where a side touches an ellipse) redraws nothing.
        let latest = |a: Vec<Vec<Point>>, b: Vec<Vec<Point>>| {
            let long: f64 = b.iter().map(|piece| length(piece)).sum();
            if long >= REDRAWN { b } else { a }
        };
        let curves: Vec<ReadCurve> = (first.curves.into_iter().zip(later.curves))
            .zip(&answer.ellipses)
            .map(|((a, b), exact)| {
                let pieces = latest(a, b);
                let fitted = covered(exact, &pieces).then(|| fitted(&pieces)).flatten();
                ReadCurve { pieces, fitted }
            })
            .collect();
        let sides: Vec<Vec<Vec<Point>>> = (first.sides.into_iter().zip(later.sides))
            .map(|(a, b)| latest(a, b))
            .collect();
        let metrics = off(answer, &curves, &sides).map_or_else(Vec::new, |off| {
            vec![(metric, off / size.max(f64::MIN_POSITIVE))]
        });
        Self {
            curves,
            sides,
            metrics,
        }
    }
}

/// How far `p` lies from `part` of `answer`: an ellipse whole, or where seen when only that is
/// drawn (an object's).
fn apart(answer: &SolidAnswer, part: Part, p: Point) -> f64 {
    match part {
        Part::Ellipse(k) if answer.whole => answer.ellipses[k].distance(p),
        Part::Ellipse(k) => (answer.outlines[k].iter())
            .filter(|stretch| !stretch.hidden)
            .filter_map(|stretch| nearest(&[&stretch.points], p).map(|(_, d)| d))
            .fold(f64::INFINITY, f64::min),
        Part::Side(k) => {
            let [a, b] = answer.sides[k];
            distance(p, nearest_on_segment(a, b, p))
        }
    }
}

/// The part of `answer` nearest by `how_far`, within `FAR`.
fn nearest_part(answer: &SolidAnswer, how_far: impl Fn(Part) -> f64) -> Option<Part> {
    let parts = (0..answer.ellipses.len())
        .map(Part::Ellipse)
        .chain((0..answer.sides.len()).map(Part::Side));
    parts
        .map(|part| (part, how_far(part)))
        .filter(|(_, d)| *d <= FAR)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(part, _)| part)
}

/// `lines` split into straight pieces and curves, each given to the part of `answer` it
/// lies nearest.
fn read(answer: &SolidAnswer, lines: &[&[Point]]) -> Parts {
    let mut parts = Parts {
        curves: vec![Vec::new(); answer.ellipses.len()],
        sides: vec![Vec::new(); answer.sides.len()],
    };
    let mut give = |part: Part, piece: Vec<Point>| match part {
        Part::Ellipse(k) => parts.curves[k].push(piece),
        Part::Side(k) => parts.sides[k].push(piece),
    };
    for line in lines {
        let even = resample(line, SPACING);
        let straight = edges(&[line]);
        for edge in &straight {
            let points = resample(&edge.ends, SPACING);
            // A straight piece goes whole to the part its points lie nearest on the whole.
            let mean = |part: Part| {
                points.iter().map(|p| apart(answer, part, *p)).sum::<f64>() / points.len() as f64
            };
            if let Some(part) = nearest_part(answer, mean) {
                give(part, edge.ends.to_vec());
            }
        }
        // The rest, point by point, in runs going to one part.
        let on_edge = |p: Point| {
            (straight.iter())
                .any(|e| distance(p, nearest_on_segment(e.ends[0], e.ends[1], p)) <= ON_EDGE)
        };
        let mut run: Option<(Part, Vec<Point>)> = None;
        for p in even {
            let part = (!on_edge(p))
                .then(|| nearest_part(answer, |part| apart(answer, part, p)))
                .flatten();
            match (&mut run, part) {
                (Some((held, points)), Some(part)) if *held == part => points.push(p),
                _ => {
                    if let Some((held, points)) = run.take().filter(|(_, points)| points.len() > 1)
                    {
                        give(held, points);
                    }
                    run = part.map(|part| (part, vec![p]));
                }
            }
        }
        if let Some((held, points)) = run.filter(|(_, points)| points.len() > 1) {
            give(held, points);
        }
    }
    parts
}

/// Whether `pieces` go `COVERED` round `exact`: the share of its `BINS` arcs (by the angle round
/// its own centre, as on the circle it is drawn from) that hold a point.
fn covered(exact: &Ellipse, pieces: &[Vec<Point>]) -> bool {
    let (sin, cos) = exact.angle.sin_cos();
    let mut held = [false; BINS];
    for p in pieces.iter().flat_map(|piece| resample(piece, SPACING)) {
        let [dx, dy] = [p[0] - exact.centre[0], p[1] - exact.centre[1]];
        let along = (dx * cos + dy * sin) / exact.axes[0].max(f64::MIN_POSITIVE);
        let across = (-dx * sin + dy * cos) / exact.axes[1].max(f64::MIN_POSITIVE);
        let turn = across.atan2(along).rem_euclid(std::f64::consts::TAU);
        let bin = (turn / std::f64::consts::TAU * BINS as f64) as usize;
        held[bin.min(BINS - 1)] = true;
    }
    held.iter().filter(|h| **h).count() as f64 >= COVERED * BINS as f64
}

/// The ellipse `pieces` make.
fn fitted(pieces: &[Vec<Point>]) -> Option<Ellipse> {
    let lines: Vec<&[Point]> = pieces.iter().map(Vec::as_slice).collect();
    fit_ellipse(&lines)
}

/// The mean distance of everything read from the part it was read for; none when nothing was.
fn off(answer: &SolidAnswer, curves: &[ReadCurve], sides: &[Vec<Vec<Point>>]) -> Option<f64> {
    let parts = (curves.iter().enumerate())
        .map(|(k, curve)| (Part::Ellipse(k), &curve.pieces))
        .chain(
            sides
                .iter()
                .enumerate()
                .map(|(k, side)| (Part::Side(k), side)),
        );
    let (mut sum, mut count) = (0.0, 0usize);
    for (part, pieces) in parts {
        for p in pieces.iter().flat_map(|piece| resample(piece, SPACING)) {
            sum += apart(answer, part, p);
            count += 1;
        }
    }
    (count > 0).then(|| sum / count as f64)
}
