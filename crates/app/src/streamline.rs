//! StreamLine: the drawn line follows the pen on a leash, smoothing its wobble (Procreate's
//! StreamLine, draweave's stabilisation, DECISIONS D71). The line moves only when the pen is
//! farther from it than the leash, so the smoothing depends on the distance travelled, never on
//! how many events the pen sends. Only positions are smoothed: pressure, tilt and time stay the
//! pen's. While drawing, the line visibly trails the pen; when the pen lifts, the rest of the way
//! is spread over the stroke's last stretch so the finished stroke ends where the pen left.

/// Leash at full StreamLine for a brush of diameter `size`: `size * LEASH_PER_SIZE`, within
/// `LEASH_RANGE` canvas pixels.
const LEASH_PER_SIZE: f32 = 3.0;
const LEASH_RANGE: [f32; 2] = [24.0, 96.0];
/// The catch-up at the lift spreads over this many leashes of the stroke's end.
const CATCH_UP_LEASHES: f32 = 2.0;
/// Closer than this, in canvas pixels, the line already is at the pen.
const AT_PEN: f32 = 1.0e-3;

/// How strongly the line is smoothed, `0..=1`; 0 draws exactly where the pen is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StreamLine(f32);

impl StreamLine {
    pub const OFF: Self = Self(0.0);

    /// `None` unless `amount` lies in `0..=1`.
    #[must_use]
    pub fn new(amount: f32) -> Option<Self> {
        (0.0..=1.0).contains(&amount).then_some(Self(amount))
    }

    #[must_use]
    pub const fn amount(self) -> f32 {
        self.0
    }

    /// How far the line may trail the pen for a brush of diameter `size`, in canvas pixels:
    /// the full leash times the amount squared, so the slider's lower half stays gentle.
    #[must_use]
    pub fn leash(self, size: f32) -> f32 {
        let [shortest, longest] = LEASH_RANGE;
        (size * LEASH_PER_SIZE).clamp(shortest, longest) * self.0 * self.0
    }
}

/// The end of the drawn line, following the pen on a leash.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Follower {
    leash: f32,
    at: [f32; 2],
}

impl Follower {
    /// Starts at the pen's first position.
    pub(crate) const fn new(leash: f32, first: [f32; 2]) -> Self {
        Self { leash, at: first }
    }

    /// Where the line is drawn once the pen is at `pen`: it stays until the pen is farther than
    /// the leash, then moves straight towards the pen to a leash's length from it.
    pub(crate) fn follow(&mut self, pen: [f32; 2]) -> [f32; 2] {
        let offset = [pen[0] - self.at[0], pen[1] - self.at[1]];
        let separation = offset[0].hypot(offset[1]);
        if separation > self.leash {
            let back = self.leash / separation;
            self.at = [pen[0] - offset[0] * back, pen[1] - offset[1] * back];
        }
        self.at
    }

    /// The length of the stretch the catch-up at the lift spreads over.
    pub(crate) fn catch_up_length(&self) -> f32 {
        self.leash * CATCH_UP_LEASHES
    }
}

/// The drawn line `points` moved so it ends exactly at `pen`: each point within `length` of the
/// line's end (measured along it) moves by the remaining offset times a smoothstep weight, 1 at
/// the end and 0 at `length`. The first point never moves. Returns the index of the first point
/// moved and the moved points from there, or `None` when the line already ends at the pen.
pub(crate) fn catch_up(
    points: &[[f32; 2]],
    pen: [f32; 2],
    length: f32,
) -> Option<(usize, Vec<[f32; 2]>)> {
    let (&end, _) = points.split_last()?;
    let correction = [pen[0] - end[0], pen[1] - end[1]];
    if points.len() < 2 || correction[0].hypot(correction[1]) <= AT_PEN {
        return None;
    }
    let mut first = points.len() - 1;
    let mut along = 0.0;
    let mut next = end;
    for index in (1..points.len()).rev() {
        let point = points[index];
        along += (next[0] - point[0]).hypot(next[1] - point[1]);
        next = point;
        let linear = (1.0 - along / length.max(AT_PEN)).clamp(0.0, 1.0);
        if linear * linear * (3.0 - 2.0 * linear) == 0.0 {
            break;
        }
        first = index;
    }
    let mut moved: Vec<[f32; 2]> = points[first..].to_vec();
    let mut along = 0.0;
    let mut next = end;
    for point in moved.iter_mut().rev() {
        along += (next[0] - point[0]).hypot(next[1] - point[1]);
        next = *point;
        let linear = (1.0 - along / length.max(AT_PEN)).clamp(0.0, 1.0);
        let weight = linear * linear * (3.0 - 2.0 * linear);
        *point = [
            point[0] + correction[0] * weight,
            point[1] + correction[1] * weight,
        ];
    }
    if let Some(last) = moved.last_mut() {
        *last = pen;
    }
    Some((first, moved))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size draweave's tests used: the full leash is 54 px.
    const SIZE: f32 = 18.0;

    fn smoothed(points: &[[f32; 2]], amount: f32) -> Vec<[f32; 2]> {
        let streamline = StreamLine::new(amount).expect("amount");
        let mut follower = Follower::new(streamline.leash(SIZE), points[0]);
        points.iter().map(|point| follower.follow(*point)).collect()
    }

    /// A hand's wobble of 8 px every 80 px, sampled every 2 px (dense, ~300 Hz input).
    fn dense_wobble() -> Vec<[f32; 2]> {
        (0..=160u16)
            .map(|index| {
                let x = f32::from(index) * 2.0;
                [x, 8.0 * (x * std::f32::consts::TAU / 80.0).sin()]
            })
            .collect()
    }

    fn wobble_rms(points: &[[f32; 2]]) -> f32 {
        let measured = &points[20..points.len() - 20];
        let squares: f32 = measured.iter().map(|point| point[1] * point[1]).sum();
        (squares / measured.len() as f32).sqrt()
    }

    #[test]
    fn stabilization_strength_is_perceptible_on_a_dense_realistic_wobble() {
        let points = dense_wobble();
        let raw = wobble_rms(&smoothed(&points, 0.0));
        let medium = wobble_rms(&smoothed(&points, 0.5));
        let strong = wobble_rms(&smoothed(&points, 1.0));
        assert!(medium < raw * 0.8, "{medium} against {raw}");
        assert!(strong < raw * 0.3, "{strong} against {raw}");
        assert!(strong < medium);
        assert_eq!(smoothed(&points, 0.0), points, "off draws where the pen is");
    }

    #[test]
    fn active_stabilization_is_distance_based_and_bounded_across_sample_densities() {
        let sparse = [[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]];
        let dense: Vec<[f32; 2]> = (0..=100u16).map(|i| [f32::from(i) * 2.0, 0.0]).collect();
        let sparse_end = *smoothed(&sparse, 1.0).last().expect("end");
        let dense_end = *smoothed(&dense, 1.0).last().expect("end");
        assert_eq!(sparse_end, dense_end);
        assert!((200.0 - dense_end[0] - SIZE * LEASH_PER_SIZE).abs() < AT_PEN);
        // The leash: 24 px at least, 96 px at most, a quarter of it at half StreamLine.
        let half = StreamLine::new(0.5).expect("amount");
        assert_eq!(StreamLine::new(1.0).expect("amount").leash(2.0), 24.0);
        assert_eq!(StreamLine::new(1.0).expect("amount").leash(500.0), 96.0);
        assert_eq!(half.leash(SIZE), SIZE * LEASH_PER_SIZE / 4.0);
        assert_eq!(StreamLine::new(1.5), None);
    }

    #[test]
    fn completed_stabilization_catches_up_smoothly() {
        let points = dense_wobble();
        let streamline = StreamLine::new(1.0).expect("amount");
        let mut follower = Follower::new(streamline.leash(SIZE), points[0]);
        let active: Vec<[f32; 2]> = points.iter().map(|p| follower.follow(*p)).collect();
        let pen = *points.last().expect("pen");
        let end = *active.last().expect("end");
        assert!(
            (pen[0] - end[0]).hypot(pen[1] - end[1]) > 50.0,
            "the line trails"
        );
        let (first, moved) = catch_up(&active, pen, follower.catch_up_length()).expect("moved");
        assert!(first > 0 && moved.len() > 10, "{first} {}", moved.len());
        assert_eq!(*moved.last().expect("end"), pen);
        // Smooth: no step between moved points is longer than the leash's tenth plus the step
        // the line already had there.
        let mut complete = active[..first].to_vec();
        complete.extend(&moved);
        for (index, pair) in complete.windows(2).enumerate() {
            let before = &active[index..=index + 1];
            let step = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
            let was = (before[1][0] - before[0][0]).hypot(before[1][1] - before[0][1]);
            assert!(
                step <= was + 0.1 * SIZE * LEASH_PER_SIZE,
                "at {index}: {step} {was}"
            );
        }
        assert_eq!(
            catch_up(&[[1.0, 1.0]], [5.0, 5.0], 10.0),
            None,
            "a dot stays"
        );
        assert_eq!(
            catch_up(&active[..2], active[1], 10.0),
            None,
            "at the pen already"
        );
    }
}
