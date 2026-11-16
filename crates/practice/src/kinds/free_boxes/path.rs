//! A path page (S156l, Drawabox's organic perspective; free boxes' 稳): the eye looking level, the
//! horizon a little above the middle, and a path on the ground winding back to it, printed grey:
//! its two edges, exact ground curves seen, narrowing as they go back. The learner sends boxes
//! back along it freely, near ones big and far ones small, each turned its own way, a nearer box
//! hiding what lies behind it. Judged by eye as every free page (S114, D123): nothing is read from
//! the strokes, Compare runs the horizon on.
//!
//! The ground lies `EYE` below the eye (y up, z away, the eye at the world's origin); the path is a
//! smooth path through a few points on it (`geometry/space_path`), weaving side to side as it goes
//! back, its edges a fixed half width either side of it along the ground.

use crate::geometry::perspective::Horizon;
use crate::geometry::space_path::SpacePath;
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, times};

/// Pages laid before the generator keeps the last one whatever it is.
const TRIES: usize = 400;
/// The samples along the path.
const SAMPLES: usize = 241;
/// Up, and the eye's height over the ground (world).
const UP: Vector = [0.0, 1.0, 0.0];
pub(crate) const EYE: f64 = 1.0;
/// The path's points, its near end's depth, its far end's over its near's, how far it weaves
/// either side and drifts from end to end, and its half width (world).
const POINTS: usize = 5;
const NEAR: [f64; 2] = [2.4, 2.8];
const BACK: [f64; 2] = [8.0, 12.0];
const REACH: [f64; 2] = [0.45, 0.8];
const DRIFT: f64 = 0.4;
const HALF: [f64; 2] = [0.25, 0.35];
/// Where the near end's middle lies down the frame, how far the page's middle lies off the
/// frame's, and how far inside the frame the path keeps.
const BOTTOM: [f64; 2] = [0.86, 0.9];
const SHIFT: f64 = 0.06;
const INSIDE: f64 = 0.06;

/// How the level eye's picture of the world lies on the page: made `scale` frame units an image
/// unit, its middle `shift` right of the frame's, the horizon at `horizon`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ground {
    pub(crate) scale: f64,
    pub(crate) shift: f64,
    pub(crate) horizon: f64,
}

impl Ground {
    /// Where the world point `p` lies on the page (or a direction's vanishing point).
    pub(crate) fn frame(&self, p: Vector) -> Point {
        [
            0.5 + self.shift + self.scale * p[0] / p[2],
            self.horizon - self.scale * p[1] / p[2],
        ]
    }
}

/// A path page: its horizon, the path's two edges in the frame from near to far; the ground it is
/// seen on, the path's middle at its samples and its half width (world).
#[derive(Debug, Clone, PartialEq)]
pub struct PathPage {
    pub horizon: f64,
    pub edges: [Vec<Point>; 2],
    pub(crate) ground: Ground,
    pub(crate) middle: Vec<Vector>,
    pub(crate) half: f64,
}

impl PathPage {
    /// The page `seed` makes.
    #[must_use]
    pub fn generate(seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            let Some(page) = laid(&mut random) else {
                continue;
            };
            if page.inside() {
                return page;
            }
            last = Some(page);
        }
        last.unwrap_or_else(|| Self {
            horizon: Horizon::Path.height(),
            edges: [vec![[0.4, 0.9], [0.5, 0.45]], vec![[0.6, 0.9], [0.5, 0.45]]],
            ground: Ground {
                scale: 1.0,
                shift: 0.0,
                horizon: Horizon::Path.height(),
            },
            middle: Vec::new(),
            half: 0.0,
        })
    }

    /// Whether its edges keep inside the frame.
    fn inside(&self) -> bool {
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        (self.edges.iter().flatten()).all(|p| inside(p[0]) && inside(p[1]))
    }
}

/// One page laid from `random`; none were its path to hold no point.
fn laid(random: &mut Random) -> Option<PathPage> {
    let near = random.range(NEAR[0], NEAR[1]);
    let back = random.range(BACK[0], BACK[1]);
    let reach = random.range(REACH[0], REACH[1]);
    let side = random.sign();
    let drift = [0; 2].map(|_| random.range(-DRIFT, DRIFT));
    let last = (POINTS - 1) as f64;
    let points: Vec<Vector> = (0..POINTS)
        .map(|k| {
            let along = k as f64 / last;
            let way = if k % 2 == 0 { side } else { -side };
            let x =
                way * reach * random.range(0.8, 1.15) + drift[0] + (drift[1] - drift[0]) * along;
            [x, -EYE, near * back.powf(along)]
        })
        .collect();
    let path = SpacePath::through(&points)?;
    let samples = path.samples(SAMPLES, UP, &[]);
    let half = random.range(HALF[0], HALF[1]);
    let horizon = Horizon::Path.height();
    let bottom = random.range(BOTTOM[0], BOTTOM[1]);
    let ground = Ground {
        scale: (bottom - horizon) * near / EYE,
        shift: random.range(-SHIFT, SHIFT),
        horizon,
    };
    let edges = [half, -half].map(|by| {
        (samples.iter())
            .map(|s| ground.frame(add(s.at, times(s.binormal, by))))
            .collect()
    });
    Some(PathPage {
        horizon,
        edges,
        ground,
        middle: samples.iter().map(|s| s.at).collect(),
        half,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::vector::{norm, sub};

    /// The ground point the page shows at `p`.
    fn on_ground(ground: &Ground, p: Point) -> Vector {
        let depth = ground.scale * EYE / (p[1] - ground.horizon);
        let x = (p[0] - 0.5 - ground.shift) * depth / ground.scale;
        [x, -EYE, depth]
    }

    #[test]
    fn the_path_s_edges_are_the_ground_s_at_its_width_either_side_and_inside_the_frame() {
        for seed in 0..200 {
            let page = PathPage::generate(seed);
            assert_eq!(page, PathPage::generate(seed), "{seed}: per seed");
            assert!((page.horizon - 0.4).abs() < 1e-12);
            assert!(page.inside(), "{seed}");
            for (i, middle) in page.middle.iter().enumerate() {
                let [a, b] = [0, 1].map(|side| on_ground(&page.ground, page.edges[side][i]));
                let width = norm(sub(a, b));
                assert!(
                    (width - 2.0 * page.half).abs() < 1e-7,
                    "{seed} {i}: {width}"
                );
                let between = times(add(a, b), 0.5);
                assert!(
                    norm(sub(between, *middle)) < 1e-7,
                    "{seed} {i}: about the path"
                );
            }
            // It goes back to the horizon: its far end near it, its near end low on the page.
            let far = page.edges[0].last().copied().unwrap_or_default();
            assert!(far[1] - page.horizon < 0.08, "{seed}: {far:?}");
            assert!(page.edges[0][0][1] > 0.75, "{seed}");
        }
    }
}
