//! A hand drawing boxes, for the tests of the edges found and of what is off on an estimate or a
//! point in space: each edge a stroke as a hand makes it (overshooting its corners, bowing a
//! little, wobbling slowly, a tremor on every sample), some drawn twice, some corners drawn in one
//! stroke with the next edge.

use openpaint_practice::{Direction, EDGES, PerspectiveBox, Point, Random, distance};

/// How a hand draws: the most it overshoots a corner and bows (shares of the edge), wobbles and
/// trembles (frame units: 0.0025 is a pixel of a 400-pixel frame); how often it draws an edge
/// twice or a corner in one stroke with the next edge.
#[derive(Debug, Clone, Copy)]
pub struct Manner {
    pub overshoot: f64,
    pub bow: f64,
    pub wobble: f64,
    pub tremor: f64,
    pub twice: f64,
    pub corners: f64,
}

impl Manner {
    /// A careful hand ghosting its lines (Drawabox's way): bowing up to 1 % of an edge, wobbling
    /// up to 0.8 pixels, trembling 0.5.
    pub const STEADY: Self = Self {
        overshoot: 0.12,
        bow: 0.01,
        wobble: 0.002,
        tremor: 0.0012,
        twice: 0.25,
        corners: 0.3,
    };
    /// A quicker, shakier hand: bowing up to 2 %, wobbling 1.6 pixels, trembling 0.8.
    pub const SHAKY: Self = Self {
        overshoot: 0.15,
        bow: 0.02,
        wobble: 0.004,
        tremor: 0.002,
        twice: 0.35,
        corners: 0.35,
    };
}

pub struct Hand {
    random: Random,
    manner: Manner,
}

impl Hand {
    pub fn new(seed: u64, manner: Manner) -> Self {
        Self {
            random: Random::new(seed),
            manner,
        }
    }

    /// A stroke meant along `a` to `b`: past both ends, bowed, wobbling, trembling, a sample about
    /// every 0.006 of the frame (a pen at some 300 pixels a second, sampled every 8 ms).
    pub fn line(&mut self, a: Point, b: Point) -> Vec<Point> {
        let long = distance(a, b);
        let d = [(b[0] - a[0]) / long, (b[1] - a[1]) / long];
        let n = [-d[1], d[0]];
        let m = self.manner;
        let before = self.random.range(0.0, m.overshoot) * long;
        let after = self.random.range(0.0, m.overshoot) * long;
        let bow = self.random.range(-m.bow, m.bow) * long;
        let wobble = self.random.range(0.0, m.wobble);
        let (waves, phase) = (self.random.range(0.5, 2.0), self.random.range(0.0, 6.3));
        let total = long + before + after;
        let steps = (total / 0.006).ceil().max(2.0) as usize;
        (0..=steps)
            .map(|k| {
                let s = k as f64 / steps as f64;
                let along = -before + s * total;
                let side = bow * 4.0 * s * (1.0 - s)
                    + wobble * (std::f64::consts::TAU * waves * s + phase).sin()
                    + self.random.range(-m.tremor, m.tremor);
                [
                    a[0] + along * d[0] + side * n[0],
                    a[1] + along * d[1] + side * n[1],
                ]
            })
            .collect()
    }

    /// The edges `edges` (corner pairs) drawn: each a stroke, some twice, some corners drawn on
    /// into the next edge sharing it.
    pub fn draw(&mut self, edges: &[(Point, Point)]) -> Vec<Vec<Point>> {
        let mut strokes = Vec::new();
        let mut at = 0;
        while at < edges.len() {
            let (a, b) = edges[at];
            let next = edges.get(at + 1).copied();
            let joined = next.filter(|(c, _)| distance(*c, b) < 1e-9);
            if let Some((_, c)) = joined.filter(|_| self.random.unit() < self.manner.corners) {
                let mut stroke = self.line(a, b);
                // The pen turns at the corner: the first edge's overshoot is not drawn.
                stroke.retain(|p| {
                    (p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])
                        <= distance(a, b).powi(2)
                });
                stroke.extend(
                    self.line(b, c)
                        .into_iter()
                        .skip_while(|p| distance(*p, b) > 0.01),
                );
                strokes.push(stroke);
                at += 2;
                continue;
            }
            strokes.push(self.line(a, b));
            if self.random.unit() < self.manner.twice {
                let shift = self.random.range(0.001, 0.005) * self.random.sign();
                let long = distance(a, b);
                let n = [-(b[1] - a[1]) / long * shift, (b[0] - a[0]) / long * shift];
                strokes.push(self.line([a[0] + n[0], a[1] + n[1]], [b[0] + n[0], b[1] + n[1]]));
            }
            at += 1;
        }
        strokes
    }
}

/// `exact`'s edges but those from its front corner, in an order a hand goes round (each edge
/// after one it shares a corner with where it can), with their directions.
pub fn to_complete(exact: &PerspectiveBox) -> Vec<(Point, Point, Direction)> {
    let rest: Vec<(usize, usize, Direction)> = (EDGES.iter().copied())
        .filter(|(a, b, _)| *a != exact.front && *b != exact.front)
        .collect();
    chained(&rest)
        .into_iter()
        .map(|(a, b, d)| (exact.corners[a], exact.corners[b], d))
        .collect()
}

/// `edges` reordered (and turned) so each starts where the one before ended when one does.
fn chained(edges: &[(usize, usize, Direction)]) -> Vec<(usize, usize, Direction)> {
    let mut left = edges.to_vec();
    let mut out: Vec<(usize, usize, Direction)> = Vec::new();
    while !left.is_empty() {
        let end = out.last().map(|e| e.1);
        let at = end
            .and_then(|end| left.iter().position(|(a, b, _)| *a == end || *b == end))
            .unwrap_or(0);
        let (a, b, d) = left.remove(at);
        let turned = end.is_some_and(|end| b == end);
        out.push(if turned { (b, a, d) } else { (a, b, d) });
    }
    out
}
