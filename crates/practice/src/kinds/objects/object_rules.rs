//! The rules every object keeps (S94, `object.rs`): its corners at halves, thirds and quarters a
//! sixth apart, no surface turning by under `CREASE` (a seam the eye could not tell), a round part
//! standing on a top and clear of the blocks, no visible edge under `LEAST_EDGE` (40 pixels of a
//! frame), no two running together, a round part's ends no slivers, each change showing at least
//! two corners; and every visible edge a step of shade (`object_picture.rs`).

use crate::kinds::objects::object::ObjectTask;
use crate::kinds::objects::object_body::Body;
use crate::kinds::objects::object_construction::Wanted;
use crate::kinds::objects::object_design::{Design, FRACTIONS, Local, SAME, THINNEST};
use crate::maths::path::{Point, apart, distance, length};
use crate::maths::vector::{Vector, dot};

/// A surface turning by less than this (degrees) is a seam the eye could not tell.
const CREASE: f64 = 15.0;
/// No visible edge shorter (frame units, 40 pixels of a 400-pixel frame); two closer than `GAP`
/// meet, and at `MEET` degrees at least.
pub const LEAST_EDGE: f64 = 0.1;
const GAP: f64 = 0.025;
const MEET: f64 = 12.0;
/// A round part's ends at least this round (minor over major, as S93's): no sliver.
const ROUND: f64 = 0.22;

/// Whether `v` is one of `FRACTIONS`.
pub(crate) fn fraction(v: f64) -> bool {
    FRACTIONS.iter().any(|f| (f - v).abs() <= 1e-6)
}

/// Whether every coordinate `design` uses lies a sixth or more from the others along its axis.
pub(crate) fn spaced(design: &Design) -> bool {
    let mut values: [Vec<f64>; 3] = Default::default();
    let mut take = |p: Local| (0..3).for_each(|k| values[k].push(p[k]));
    for change in &design.changes {
        change
            .out
            .iter()
            .for_each(|b| [b.low, b.high].into_iter().for_each(&mut take));
        change
            .cuts
            .iter()
            .for_each(|c| c.through.into_iter().for_each(&mut take));
        if let Some(turned) = change.turned {
            [turned.block.low, turned.block.high]
                .into_iter()
                .for_each(&mut take);
        }
    }
    values.iter_mut().all(|axis| {
        axis.extend([0.0, 1.0]);
        axis.sort_by(f64::total_cmp);
        axis.dedup_by(|a, b| (*a - *b).abs() <= SAME);
        axis.windows(2)
            .all(|pair| pair[1] - pair[0] >= THINNEST - SAME)
    })
}

/// Whether every two of `body`'s planes that are not parallel turn by `CREASE` or more.
pub(crate) fn creased(body: &Body) -> bool {
    let normals: Vec<Vector> = (body.hulls.iter())
        .flat_map(|hull| hull.planes.iter().map(|plane| plane.normal))
        .collect();
    let least = CREASE.to_radians().cos();
    normals.iter().enumerate().all(|(i, a)| {
        normals[i + 1..].iter().all(|b| {
            let cos = dot(*a, *b).abs();
            cos >= 1.0 - 1e-9 || cos <= least
        })
    })
}

/// Whether each round part of `design` stands on a top of `body`'s blocks, clear of them: solid
/// just under its lowest point, nothing of the blocks in it.
pub(crate) fn stands(design: &Design, body: &Body) -> bool {
    let placed = body.placed;
    (design.changes.iter())
        .filter_map(|c| c.turned)
        .all(|turned| {
            let (low, high) = (turned.block.low, turned.block.high);
            let middle: Local = std::array::from_fn(|k| (low[k] + high[k]) / 2.0);
            let under = placed.world([middle[0], low[1] - 1e-3, middle[2]]);
            let inside = [0.5, 0.1, 0.9].map(|share| {
                let mut p = middle;
                p[turned.axis] = low[turned.axis] + (high[turned.axis] - low[turned.axis]) * share;
                placed.world(p)
            });
            let blocks = Body {
                rounds: Vec::new(),
                ..body.clone()
            };
            blocks.holds(under, 0.0, None) && inside.iter().all(|p| !blocks.holds(*p, -1e-9, None))
        })
}

/// Whether each change of `task` shows at least two of the corners it asks found.
pub(crate) fn shown(task: &ObjectTask, wanted: &[Wanted]) -> bool {
    wanted.iter().all(|want| {
        let ends = want.ends.iter().flatten();
        let all: Vec<&Local> = want.corners.iter().chain(ends).collect();
        all.iter().filter(|p| task.shows(***p)).count() >= 2
    })
}

impl ObjectTask {
    /// Whether its visible edges can be drawn and told apart: none shorter than `LEAST_EDGE`, no
    /// two closer than `GAP` but where they meet, and those at `MEET` degrees or more.
    pub(crate) fn drawable(&self) -> bool {
        let sides = &self.answer.sides;
        let long = sides.iter().all(|[a, b]| distance(*a, *b) >= LEAST_EDGE)
            && (self.answer.outlines.iter().flatten())
                .filter(|stretch| !stretch.hidden)
                .all(|stretch| length(&stretch.points) >= LEAST_EDGE);
        let direction = |[a, b]: [Point; 2]| {
            let long = distance(a, b).max(f64::MIN_POSITIVE);
            [(b[0] - a[0]) / long, (b[1] - a[1]) / long]
        };
        let told = (0..sides.len()).all(|i| {
            (i + 1..sides.len()).all(|j| {
                if apart(&sides[i], &sides[j]) >= GAP {
                    return true;
                }
                let [u, v] = [direction(sides[i]), direction(sides[j])];
                let sin = (u[0] * v[1] - u[1] * v[0]).abs();
                apart(&sides[i], &sides[j]) <= 1e-6 && sin >= MEET.to_radians().sin()
            })
        });
        let round = (self.squares.iter()).all(|square| square.exact.roundness() >= ROUND);
        long && told && round
    }
}
