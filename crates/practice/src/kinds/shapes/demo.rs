//! A single shape's demonstration (`demos/demo.rs`): a generated vase, so no pack is needed, its
//! extremes ticked, blocked in with long straight lines, a negative shape looked at, the outline
//! in ink, then compared.

use crate::demos::demo::{Demo, Drawn, Layout, Marker, On, Script, TICK, tick, wobbled};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const SHAPE_SEED: u64 = 0xDE30_5EED_0003;
/// The vase's colour in the original's frame.
const CLAY: Rgb8 = [176, 118, 88];

/// A single shape: a vase, its silhouette in the original's frame.
pub(crate) fn shape() -> Demo {
    let vase = Vase::generate(SHAPE_SEED);
    let outline = vase.outline();
    let mut script = Script::new();
    let body = Drawn::Band {
        rows: vase.rows.clone(),
        colour: Some(CLAY),
    };
    script.given(On::Original, Marker::Given, body);
    script.step();
    // Top and bottom, then left and right, each pair looked at together.
    for pair in vase.extremes().chunks(2) {
        let ticks = pair.iter().map(|(at, towards)| {
            (
                On::Copy,
                Marker::Pencil,
                Drawn::Line(tick(*at, *towards, TICK)),
            )
        });
        script.together(ticks.collect());
    }
    script.step();
    for side in vase.blocked() {
        script.draw(On::Copy, Marker::Pencil, Drawn::Line(side));
    }
    script.step();
    let pocket = Drawn::Band {
        rows: vase.pocket(),
        colour: None,
    };
    script.draw(On::Copy, Marker::Shade, pocket);
    script.step();
    script.draw(
        On::Copy,
        Marker::Ink,
        Drawn::Line(wobbled(&outline, 0.012, 1.3)),
    );
    let faint = Drawn::Band {
        rows: vase.rows,
        colour: Some(CLAY),
    };
    let compared = vec![
        (On::Copy, Marker::Faint, faint),
        (On::Copy, Marker::Answer, Drawn::Line(outline)),
    ];
    script.compare(Family::Shape, Layout::Pair, compared)
}

/// A vase seen from the side: its rows from the lip down, each its left and right edge.
struct Vase {
    rows: Vec<[Point; 2]>,
}

/// The vase's profile, top to bottom: height and half width (a lip, a neck, a shoulder, a belly,
/// a foot).
const PROFILE: [Point; 10] = [
    [0.12, 0.15],
    [0.16, 0.16],
    [0.21, 0.08],
    [0.3, 0.065],
    [0.41, 0.17],
    [0.56, 0.26],
    [0.71, 0.22],
    [0.8, 0.12],
    [0.85, 0.09],
    [0.9, 0.14],
];
/// Its middle across the frame; the rows of its lip, from the top.
const MIDDLE: f64 = 0.5;
const LIP: usize = 8;

impl Vase {
    /// The vase `seed` makes: the profile's widths a little varied, joined smoothly.
    fn generate(seed: u64) -> Self {
        let mut random = Random::new(seed);
        let profile: Vec<Point> = (PROFILE.iter())
            .map(|[y, w]| [*y, w * random.range(0.92, 1.08)])
            .collect();
        let mut rows = Vec::new();
        for k in 0..profile.len() - 1 {
            let at = |i: usize| profile[i.clamp(0, profile.len() - 1)];
            let (p0, p1, p2, p3) = (at(k.saturating_sub(1)), at(k), at(k + 1), at(k + 2));
            for s in 0..8 {
                let t = f64::from(s) / 8.0;
                let y = p1[0] + (p2[0] - p1[0]) * t;
                let w = catmull(p0[1], p1[1], p2[1], p3[1], t).max(0.02);
                rows.push([[MIDDLE - w, y], [MIDDLE + w, y]]);
            }
        }
        let [y, w] = profile[profile.len() - 1];
        rows.push([[MIDDLE - w, y], [MIDDLE + w, y]]);
        Self { rows }
    }

    /// Its outline round: down the right side, back up the left, closed at the lip.
    fn outline(&self) -> Vec<Point> {
        let right = self.rows.iter().map(|[_, r]| *r);
        let left = self.rows.iter().rev().map(|[l, _]| *l);
        let mut points: Vec<Point> = right.chain(left).collect();
        points.push(self.rows[0][1]);
        points
    }

    /// Its top, bottom, left and right, each with a point inwards from it: a tick across the way
    /// in marks how far the silhouette reaches (level at the top and bottom, upright at the
    /// sides).
    fn extremes(&self) -> [(Point, Point); 4] {
        let widest = (0..self.rows.len())
            .max_by(|a, b| self.rows[*a][1][0].total_cmp(&self.rows[*b][1][0]))
            .unwrap_or(0);
        let [top, bottom] = [self.rows[0][0][1], self.rows[self.rows.len() - 1][0][1]];
        let [left, right] = self.rows[widest];
        let inwards = |p: Point, d: Point| (p, [p[0] + d[0], p[1] + d[1]]);
        [
            inwards([MIDDLE, top], [0.0, 0.1]),
            inwards([MIDDLE, bottom], [0.0, -0.1]),
            inwards(left, [0.1, 0.0]),
            inwards(right, [-0.1, 0.0]),
        ]
    }

    /// The silhouette blocked in with long straight lines through its turning points, the right
    /// side and the left as the hand draws them, then the lip and the foot.
    fn blocked(&self) -> Vec<Vec<Point>> {
        let turns: Vec<usize> = (0..PROFILE.len())
            .map(|k| (k * 8).min(self.rows.len() - 1))
            .collect();
        let keep = [0, 1, 3, 5, 7, 9];
        let side = |which: usize| -> Vec<Point> {
            keep.iter().map(|k| self.rows[turns[*k]][which]).collect()
        };
        let (top, bottom) = (self.rows[0], self.rows[self.rows.len() - 1]);
        vec![
            side(1),
            side(0),
            vec![top[0], top[1]],
            vec![bottom[0], bottom[1]],
        ]
    }

    /// The negative shape at the right of the neck: between the outline and the straight line
    /// down from the lip's edge to where the outline comes out to it again.
    fn pocket(&self) -> Vec<[Point; 2]> {
        let lip = (0..=LIP)
            .max_by(|a, b| self.rows[*a][1][0].total_cmp(&self.rows[*b][1][0]))
            .unwrap_or(0);
        let edge = self.rows[lip][1][0];
        self.rows[lip..]
            .iter()
            .skip_while(|[_, right]| right[0] >= edge - 1e-9)
            .take_while(|[_, right]| right[0] < edge)
            .map(|[_, right]| [*right, [edge, right[1]]])
            .collect()
    }
}

/// Catmull-Rom between `b` and `c` at `t`.
fn catmull(a: f64, b: f64, c: f64, d: f64, t: f64) -> f64 {
    let t2 = t * t;
    0.5 * (2.0 * b
        + (c - a) * t
        + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
        + (3.0 * b - a - 3.0 * c + d) * t2 * t)
}
