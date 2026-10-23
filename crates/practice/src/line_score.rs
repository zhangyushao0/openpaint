//! How a copy of a line compares with its original, every distance in frame sides (the frame's
//! side is 1; the summary shows them as percent):
//!
//! - Key points: the mean, over the original's key points (its ends and most bulging points,
//!   `LineExercise::key_points`), of the distance to the nearest point of the copy. V大 has the
//!   ends and the most obvious turns fixed first; these say how well they were.
//! - Frame: the symmetric mean distance between copy and original where each lies in its frame:
//!   half the mean distance of the original's points from the copy plus half the mean distance of
//!   the copy's points from the original, both lines taken at even steps of `SPACING`. Wrong
//!   place, size, turn or shape all count, as V大 judges a copy against its frame.
//! - Shape: the same after the copy is moved and sized onto the original: both lines centred on
//!   their centre along their length and scaled to the same spread (root mean square distance
//!   from it), the distance measured there and given back at the original's spread. A moved or
//!   evenly sized copy scores as an exact one; a turned one does not: angles count.
//! - Stroke: the paint strokes in the frame, eraser strokes, their speed (their length over the
//!   time the pen was down on them, frame sides a second) and wobble: the root mean square
//!   distance of the copy, taken at even steps of `WOBBLE_SPACING`, from its nine-point quadratic
//!   Savitzky-Golay smoothing (a window of about 4 % of the frame): a shaky line wobbles, a
//!   smooth one, however bent, hardly at all.
//!
//! The copy is what the pen laid (StreamLine is off in practice); what an eraser took away is
//! left out of every distance.

use crate::line::LineExercise;
use crate::metric::{Metric, Metrics};
use crate::path::{
    Point, centre_and_spread, mean_distance, nearest, normalised, resample, smoothing,
};

/// Steps the two lines are compared at, in frame sides.
const SPACING: f64 = 0.005;
/// Steps the wobble is measured at, in frame sides.
const WOBBLE_SPACING: f64 = 0.005;

/// A copy of a line as the pen drew it, in frame units.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineCopy {
    /// Each stretch of paint still there: the pen's positions along a paint stroke, cut where an
    /// eraser took it away.
    pub pieces: Vec<Vec<Point>>,
    /// Paint strokes and eraser strokes drawn in the frame.
    pub strokes: u32,
    pub erased: u32,
    /// The paint strokes' length (frame sides) and the time the pen was down on them (ms).
    pub length: f64,
    pub duration_ms: f64,
}

/// The numbers of one copy, distances in frame sides.
#[derive(Debug, Clone, PartialEq)]
pub struct LineScore {
    pub key_points: f64,
    pub frame: f64,
    pub shape: f64,
    /// Frame sides a second; `None` when the strokes took no time.
    pub speed: Option<f64>,
    pub wobble: f64,
    pub strokes: u32,
    pub erased: u32,
    pub seconds: f64,
}

impl LineScore {
    /// The score as stored.
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        let mut metrics = vec![
            (Metric::KeyPoints, self.key_points),
            (Metric::Frame, self.frame),
            (Metric::Shape, self.shape),
            (Metric::Wobble, self.wobble),
            (Metric::Strokes, f64::from(self.strokes)),
            (Metric::Erased, f64::from(self.erased)),
            (Metric::Seconds, self.seconds),
        ];
        if let Some(speed) = self.speed {
            metrics.push((Metric::Speed, speed));
        }
        metrics
    }
}

/// `copy` against `original`; `None` when the copy holds no paint.
#[must_use]
pub fn score_line(original: &LineExercise, copy: &LineCopy) -> Option<LineScore> {
    let pieces: Vec<&[Point]> = copy
        .pieces
        .iter()
        .map(Vec::as_slice)
        .filter(|piece| !piece.is_empty())
        .collect();
    if pieces.is_empty() {
        return None;
    }
    let originals: [&[Point]; 1] = [&original.points];
    let key_points = mean_distance(&original.key_points, &pieces)?;
    let frame = symmetric(&originals, &pieces)?;
    let shape = shape(&originals, &pieces).unwrap_or(frame);
    let seconds = copy.duration_ms / 1000.0;
    Some(LineScore {
        key_points,
        frame,
        shape,
        speed: (seconds > 0.0).then(|| copy.length / seconds),
        wobble: wobble(&pieces),
        strokes: copy.strokes,
        erased: copy.erased,
        seconds,
    })
}

/// Each key point of `original` and the point of the copy nearest it (for the overlay).
#[must_use]
pub fn key_point_matches(original: &LineExercise, copy: &LineCopy) -> Vec<(Point, Point)> {
    let pieces: Vec<&[Point]> = copy.pieces.iter().map(Vec::as_slice).collect();
    original
        .key_points
        .iter()
        .filter_map(|&key| nearest(&pieces, key).map(|(at, _)| (key, at)))
        .collect()
}

/// The symmetric mean distance between lines `a` and lines `b`.
fn symmetric(a: &[&[Point]], b: &[&[Point]]) -> Option<f64> {
    let even = |lines: &[&[Point]]| -> Vec<Point> {
        lines
            .iter()
            .flat_map(|line| resample(line, SPACING))
            .collect()
    };
    let there = mean_distance(&even(a), b)?;
    let back = mean_distance(&even(b), a)?;
    Some((there + back) / 2.0)
}

/// `symmetric` after both are centred and scaled to one spread, given at `original`'s spread;
/// `None` when either has no length.
fn shape(original: &[&[Point]], copy: &[&[Point]]) -> Option<f64> {
    let (original_centre, original_spread) = centre_and_spread(original)?;
    let (copy_centre, copy_spread) = centre_and_spread(copy)?;
    if original_spread <= 0.0 || copy_spread <= 0.0 {
        return None;
    }
    let moved = |lines: &[&[Point]], centre: Point, spread: f64| -> Vec<Vec<Point>> {
        lines
            .iter()
            .map(|line| normalised(line, centre, 1.0 / spread))
            .collect()
    };
    let a = moved(original, original_centre, original_spread);
    let b = moved(copy, copy_centre, copy_spread);
    let a: Vec<&[Point]> = a.iter().map(Vec::as_slice).collect();
    let b: Vec<&[Point]> = b.iter().map(Vec::as_slice).collect();
    Some(symmetric(&a, &b)? * original_spread)
}

/// The root mean square distance of the pieces, at even steps, from their smoothing.
fn wobble(pieces: &[&[Point]]) -> f64 {
    let (mut squares, mut count) = (0.0, 0usize);
    for piece in pieces {
        for (point, smooth) in smoothing(&resample(piece, WOBBLE_SPACING)) {
            squares += (point[0] - smooth[0]).powi(2) + (point[1] - smooth[1]).powi(2);
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        (squares / count as f64).sqrt()
    }
}
