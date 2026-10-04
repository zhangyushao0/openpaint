//! Tone curves (S28, Procreate's Curves): a curve maps a channel's 8-bit value to a new one
//! through points the hand places, interpolated by a monotone cubic (Fritsch-Carlson), so the
//! curve never overshoots between its points. It always runs from x = 0 to x = 1.

use std::sync::Arc;

/// Most points a tone curve holds, its two ends included.
pub const MAX_TONE_POINTS: usize = 16;
/// Closest two points may lie along x: one 8-bit step.
pub const TONE_GAP: f32 = 1.0 / 255.0;

/// Points `[x, y]` in `0..=1`, x strictly rising from 0 to 1 by at least `TONE_GAP`. Clone is
/// O(1).
#[derive(Debug, Clone, PartialEq)]
pub struct ToneCurve {
    points: Arc<[[f32; 2]]>,
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self::identity()
    }
}

impl ToneCurve {
    /// The straight line from black to white: nothing changes.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            points: Arc::from([[0.0, 0.0], [1.0, 1.0]]),
        }
    }

    /// `None` unless there are 2 to `MAX_TONE_POINTS` points in `0..=1`, the first at x = 0,
    /// the last at x = 1 and x rising by at least `TONE_GAP` between neighbours.
    #[must_use]
    pub fn new(points: &[[f32; 2]]) -> Option<Self> {
        let unit = |v: f32| (0.0..=1.0).contains(&v);
        let valid = (2..=MAX_TONE_POINTS).contains(&points.len())
            && points.iter().flatten().all(|v| unit(*v))
            && points.first().is_some_and(|p| p[0] == 0.0)
            && points.last().is_some_and(|p| p[0] == 1.0)
            // A point moved against its neighbour lies one gap away, within rounding.
            && points.windows(2).all(|w| w[1][0] - w[0][0] >= TONE_GAP - 1e-6);
        valid.then(|| Self {
            points: Arc::from(points),
        })
    }

    #[must_use]
    pub fn points(&self) -> &[[f32; 2]] {
        &self.points
    }

    /// True for the straight line from black to white.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.points[..] == [[0.0, 0.0], [1.0, 1.0]]
    }

    /// The curve's value at `x` (clamped to `0..=1`), in `0..=1`.
    #[must_use]
    pub fn value(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let points = &self.points;
        let segment = points
            .windows(2)
            .position(|w| x <= w[1][0])
            .unwrap_or(points.len() - 2);
        let tangents = self.tangents();
        let ([x0, y0], [x1, y1]) = (points[segment], points[segment + 1]);
        let h = x1 - x0;
        let t = (x - x0) / h;
        let (t2, t3) = (t * t, t * t * t);
        let value = (2.0 * t3 - 3.0 * t2 + 1.0) * y0
            + (t3 - 2.0 * t2 + t) * h * tangents[segment]
            + (-2.0 * t3 + 3.0 * t2) * y1
            + (t3 - t2) * h * tangents[segment + 1];
        value.clamp(0.0, 1.0)
    }

    /// The curve for every 8-bit value: entry `i` is `value(i / 255)` rounded to 8 bits.
    #[must_use]
    pub fn lut(&self) -> [u8; 256] {
        let mut lut = [0u8; 256];
        for (index, entry) in lut.iter_mut().enumerate() {
            let value = self.value(index as f32 / 255.0);
            *entry = (value * 255.0).round() as u8;
        }
        lut
    }

    /// The curve with a point added on it at `x`, and that point's index; `None` when it is
    /// full or `x` lies closer than `TONE_GAP` to a point.
    #[must_use]
    pub fn with_point(&self, x: f32) -> Option<(Self, usize)> {
        let at = self.points.iter().position(|p| p[0] > x)?;
        let mut points = self.points.to_vec();
        points.insert(at, [x, self.value(x)]);
        Self::new(&points).map(|curve| (curve, at))
    }

    /// The curve with point `index` moved towards `[x, y]`: kept inside `0..=1`, between its
    /// neighbours along x (the ends stay at x = 0 and 1). `None` for an index it does not hold.
    #[must_use]
    pub fn moved(&self, index: usize, [x, y]: [f32; 2]) -> Option<Self> {
        let last = self.points.len() - 1;
        (index <= last).then_some(())?;
        let x = match index {
            0 => 0.0,
            _ if index == last => 1.0,
            _ => x.clamp(
                self.points[index - 1][0] + TONE_GAP,
                self.points[index + 1][0] - TONE_GAP,
            ),
        };
        let mut points = self.points.to_vec();
        points[index] = [x, y.clamp(0.0, 1.0)];
        Self::new(&points)
    }

    /// Fritsch-Carlson tangents: the mean of the neighbouring slopes, zero at a turn or flat
    /// stretch, scaled down where they would overshoot.
    fn tangents(&self) -> Vec<f32> {
        let points = &self.points;
        let slopes: Vec<f32> = points
            .windows(2)
            .map(|w| (w[1][1] - w[0][1]) / (w[1][0] - w[0][0]))
            .collect();
        let count = points.len();
        let mut tangents = vec![0.0f32; count];
        tangents[0] = slopes[0];
        tangents[count - 1] = slopes[count - 2];
        for k in 1..count - 1 {
            let (before, after) = (slopes[k - 1], slopes[k]);
            tangents[k] = if before * after <= 0.0 {
                0.0
            } else {
                (before + after) / 2.0
            };
        }
        for (k, slope) in slopes.iter().enumerate() {
            if *slope == 0.0 {
                tangents[k] = 0.0;
                tangents[k + 1] = 0.0;
                continue;
            }
            let (alpha, beta) = (tangents[k] / slope, tangents[k + 1] / slope);
            let length = alpha.hypot(beta);
            if length > 3.0 {
                tangents[k] = 3.0 / length * alpha * slope;
                tangents[k + 1] = 3.0 / length * beta * slope;
            }
        }
        tangents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_maps_every_value_to_itself() {
        let lut = ToneCurve::identity().lut();
        assert!(lut.iter().enumerate().all(|(i, v)| usize::from(*v) == i));
    }

    #[test]
    fn a_point_lifts_the_middle_without_overshooting_its_ends() {
        let curve = ToneCurve::new(&[[0.0, 0.0], [0.5, 0.75], [1.0, 1.0]]).expect("curve");
        assert!((curve.value(0.5) - 0.75).abs() < 1e-6);
        let lut = curve.lut();
        assert_eq!((lut[0], lut[255]), (0, 255));
        assert!(lut.windows(2).all(|w| w[0] <= w[1]), "monotone");
        // Between its points a rising curve stays between their values.
        assert!(lut[64] > 64 && lut[64] < 191);
    }

    #[test]
    fn points_are_added_on_the_curve_and_moved_between_their_neighbours() {
        let (curve, at) = ToneCurve::identity().with_point(0.25).expect("added");
        assert_eq!(at, 1);
        assert!((curve.points()[1][1] - 0.25).abs() < 1e-6);
        let moved = curve.moved(1, [2.0, 0.6]).expect("moved");
        assert_eq!(moved.points()[1], [1.0 - TONE_GAP, 0.6]);
        let end = moved.moved(0, [0.3, 0.2]).expect("moved end");
        assert_eq!(end.points()[0], [0.0, 0.2]);
        assert!(ToneCurve::identity().with_point(1.0).is_none());
        assert!(
            ToneCurve::new(&[[0.0, 0.0], [0.5, 0.5]]).is_none(),
            "ends at 1"
        );
    }
}
