//! A hand drawing boxes, for the tests of the edges found and the perspective checks: each edge
//! a stroke as a hand makes it (overshooting its corners, bowing a little, wobbling slowly, a
//! tremor on every sample), some drawn twice, some corners drawn in one stroke with the next edge;
//! a box built on paper from its vanishing points; a box made smaller and moved along its horizon.

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

/// Every edge of `exact`, chained as `to_complete`'s.
pub fn all_edges(exact: &PerspectiveBox) -> Vec<(Point, Point, Direction)> {
    chained(&EDGES)
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

/// A two-point box built on paper from its vanishing points `left` and `right` (on one horizon):
/// its front edge `[top, bottom]`, its sides reaching `reach` of the way to each point. Corners as
/// `PerspectiveBox` numbers them; the front corner given the top one.
pub fn from_points(
    left: Point,
    right: Point,
    front: [Point; 2],
    reach: [f64; 2],
) -> PerspectiveBox {
    let toward = |p: Point, to: Point, share: f64| {
        [p[0] + (to[0] - p[0]) * share, p[1] + (to[1] - p[1]) * share]
    };
    let [top, bottom] = front;
    let l_bottom = toward(bottom, left, reach[0]);
    let r_bottom = toward(bottom, right, reach[1]);
    // The corners above them: where the verticals through them cross the lines from the top.
    let up = |at: Point, to: Point| {
        let t = (at[0] - top[0]) / (to[0] - top[0]);
        [at[0], top[1] + t * (to[1] - top[1])]
    };
    let (l_top, r_top) = (up(l_bottom, left), up(r_bottom, right));
    let back_bottom = crossing(r_bottom, left, l_bottom, right);
    let back_top = crossing(r_top, left, l_top, right);
    PerspectiveBox {
        corners: [
            bottom,
            r_bottom,
            top,
            r_top,
            l_bottom,
            back_bottom,
            l_top,
            back_top,
        ],
        horizon: left[1],
        vanishing: [Some(left), Some(right), None],
        front: 2,
    }
}

/// A three-point box built on paper from its vanishing points `left` and `right` (on one horizon)
/// and `third`: its front edge `[top, bottom]` (heading for `third`), its sides reaching `reach` of
/// the way to each point, its uprights on the lines to `third`.
pub fn from_three_points(
    [left, right, third]: [Point; 3],
    front: [Point; 2],
    reach: [f64; 2],
) -> PerspectiveBox {
    let toward = |p: Point, to: Point, share: f64| {
        [p[0] + (to[0] - p[0]) * share, p[1] + (to[1] - p[1]) * share]
    };
    let [top, bottom] = front;
    let l_bottom = toward(bottom, left, reach[0]);
    let r_bottom = toward(bottom, right, reach[1]);
    let l_top = crossing(l_bottom, third, top, left);
    let r_top = crossing(r_bottom, third, top, right);
    let back_bottom = crossing(r_bottom, left, l_bottom, right);
    let back_top = crossing(r_top, left, l_top, right);
    PerspectiveBox {
        corners: [
            bottom,
            r_bottom,
            top,
            r_top,
            l_bottom,
            back_bottom,
            l_top,
            back_top,
        ],
        horizon: left[1],
        vanishing: [Some(left), Some(right), Some(third)],
        front: 2,
    }
}

/// Where the line from `a` to `p` crosses the line from `b` to `q`.
fn crossing(a: Point, p: Point, b: Point, q: Point) -> Point {
    let r = [p[0] - a[0], p[1] - a[1]];
    let s = [q[0] - b[0], q[1] - b[1]];
    let across = r[0] * s[1] - r[1] * s[0];
    let t = ((b[0] - a[0]) * s[1] - (b[1] - a[1]) * s[0]) / across;
    [a[0] + t * r[0], a[1] + t * r[1]]
}

/// `exact` made `scale` as large about its horizon's point at the frame's middle and moved across
/// to `x`, its horizon kept: every point `p` to `[x + scale (p.x - 0.5), h + scale (p.y - h)]`.
pub fn placed(exact: &PerspectiveBox, x: f64, scale: f64) -> PerspectiveBox {
    let h = exact.horizon;
    let map = |p: Point| [x + scale * (p[0] - 0.5), h + scale * (p[1] - h)];
    PerspectiveBox {
        corners: exact.corners.map(map),
        horizon: h,
        vanishing: exact.vanishing.map(|v| v.map(map)),
        front: exact.front,
    }
}
