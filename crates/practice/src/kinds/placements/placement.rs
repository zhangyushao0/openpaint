//! A placement's task (S153): a plan (`plan.rs`) seen by an eye (`sight.rs`), the ground's corners,
//! the horizon and the unit given in the copy's frame, the exact answer and V大's construction
//! (`answer.rs`). Laid again from its seed's sequence until the plan and the sight keep their rules
//! and every piece shows: its top's middle seen, and where its height is carried, the lines that
//! carry it crossing its upright at an angle the eye can read. Nothing is stored but kind and seed.

use crate::geometry::perspective::angle_between;
use crate::kinds::objects::object_answer::seen;
use crate::kinds::objects::object_body::Body;
use crate::kinds::placements::answer::{Construction, Given, body, construction};
use crate::kinds::placements::plan::Plan;
use crate::kinds::placements::sight::Sight;
use crate::kinds::solids::solid_answer::{SolidAnswer, View};
use crate::maths::path::Point;
use crate::maths::random::Random;

/// Plans laid before the generator keeps the last one whatever it is; sights tried for each.
const TRIES: usize = 300;
const SIGHTS: usize = 12;
/// How sharply a carried height's lines cross the piece's upright at least, degrees.
const CROSSING: f64 = 12.0;
/// A seen point lies nearer than anything else on its ray by this share of the way.
const SEEN: f64 = 1e-6;
/// The seed of the plain placement, the generator's last resort.
const PLAIN: u64 = 0x0001_53E0;

/// A placement's variant: how many pieces, what they stand on and how the eye looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Two boxes on the ground's halves and quarters, a unit or a half tall, looked at level (入门).
    TwoBoxes,
    /// Three boxes so (入门).
    ThreeBoxes,
    /// Four pieces, one turned in the plan and one a cylinder, heights in thirds (熟练).
    Turned,
    /// Five boxes on eighths seen from above in three points, no upright given but the unit along
    /// the ground's near edge (稳).
    Above,
}

impl Placement {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 4] = [Self::TwoBoxes, Self::ThreeBoxes, Self::Turned, Self::Above];

    /// Whether the upright at the near corner is given (else the unit along the near edge).
    #[must_use]
    pub fn upright_given(self) -> bool {
        self != Self::Above
    }
}

/// A placement, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementTask {
    pub placement: Placement,
    pub plan: Plan,
    /// The ground's corners in the copy's frame: near, right, far, left (the plan's from its near
    /// corner round).
    pub ground: [Point; 4],
    pub horizon: f64,
    /// Where the ground's edges vanish, to the left, to the right, and where its uprights meet
    /// (three points only).
    pub vanishing: [Option<Point>; 3],
    /// The unit upright at the near corner, foot and top: given but at 稳, where it is the eye's.
    pub upright: [Point; 2],
    /// At 稳 the unit's far end along the near edge from the near corner, given instead.
    pub along: Option<Point>,
    /// Every edge and outline the eye sees.
    pub answer: SolidAnswer,
    /// Where pieces hide their straight edges and outlines.
    pub hidden: Vec<[Point; 2]>,
    pub construction: Construction,
    pub(crate) sight: Sight,
}

impl PlacementTask {
    /// The placement `placement` and `seed` make.
    #[must_use]
    pub fn generate(placement: Placement, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let down = !placement.upright_given();
        let mut last = None;
        for _ in 0..TRIES {
            let Some(plan) = Plan::laid(placement, &mut random) else {
                continue;
            };
            for _ in 0..SIGHTS {
                let sight = Sight::looked(&plan, down, &mut random);
                let Some(sight) = sight.filter(|sight| sight.keeps(&plan)) else {
                    continue;
                };
                let Some((task, body)) = laid(placement, plan.clone(), sight) else {
                    continue;
                };
                if task.shows(&body) {
                    return task;
                }
                last = Some(task);
            }
        }
        last.unwrap_or_else(|| plain(placement))
    }

    /// Piece `index`'s footprint's corners in the copy's frame, in the plan's order: their exact
    /// places.
    #[must_use]
    pub fn feet(&self, index: usize) -> [Point; 4] {
        self.corners(index, 0.0)
    }

    /// Piece `index`'s top's corners in the copy's frame, in the plan's order.
    #[must_use]
    pub fn tops(&self, index: usize) -> [Point; 4] {
        let height = self
            .plan
            .pieces
            .get(index)
            .map_or(0.0, |piece| piece.height);
        self.corners(index, height)
    }

    /// Where the plan's point `p` lies `height` above the ground in the copy's frame, exactly.
    #[must_use]
    pub fn at(&self, p: Point, height: f64) -> Point {
        self.sight.seen(p, height)
    }

    fn corners(&self, index: usize, height: f64) -> [Point; 4] {
        let Some(piece) = self.plan.pieces.get(index) else {
            return [[0.0; 2]; 4];
        };
        (self.plan.corners(piece)).map(|p| self.sight.seen(p, height))
    }

    /// Whether every piece shows: its top's middle seen, and its height's lines crossing its
    /// upright at `CROSSING` at least.
    fn shows(&self, body: &Body) -> bool {
        let tops_seen = (self.plan.pieces.iter()).all(|piece| {
            let corners = self.plan.corners(piece);
            let middle = [0, 1].map(|k| corners.iter().map(|p| p[k]).sum::<f64>() / 4.0);
            let p = self.sight.viewed(self.sight.world(middle, piece.height));
            body.hit(p).is_none_or(|hit| hit.t >= 1.0 - SEEN)
        });
        let readable = (self.construction.carried.iter()).all(|carried| {
            let Some([from, to]) = carried.lines.last().copied() else {
                return true;
            };
            let foot = carried.foot;
            let up = self.vanishing[2].unwrap_or([foot[0], foot[1] - 1.0]);
            let crossing = angle_between(
                [to[0] - from[0], to[1] - from[1]],
                [up[0] - foot[0], up[1] - foot[1]],
            );
            crossing.min(180.0 - crossing) >= CROSSING
        });
        tops_seen && readable
    }
}

/// The task of `plan` seen by `sight`, with its body; none where its construction finds no
/// crossing.
fn laid(placement: Placement, plan: Plan, sight: Sight) -> Option<(PlacementTask, Body)> {
    let given = given(placement, &plan, &sight);
    let construction = construction(&plan, &given)?;
    Some(made(placement, (plan, sight), given, construction))
}

/// What the copy's frame of `plan` seen by `sight` gives.
fn given(placement: Placement, plan: &Plan, sight: &Sight) -> Given {
    let [width, depth] = plan.ground;
    let corners = [[0.0, 0.0], [width, 0.0], [width, depth], [0.0, depth]];
    Given {
        ground: corners.map(|p| sight.seen(p, 0.0)),
        horizon: sight.horizon(),
        upright: [0.0, 1.0].map(|height| sight.seen([0.0, 0.0], height)),
        third: sight.vanishing_points()[2].filter(|_| !placement.upright_given()),
    }
}

/// The task made of its plan, sight, givens and construction, with its body.
fn made(
    placement: Placement,
    (plan, sight): (Plan, Sight),
    given: Given,
    construction: Construction,
) -> (PlacementTask, Body) {
    let [left, right, _] = sight.vanishing_points();
    let along = (!placement.upright_given()).then(|| sight.seen([1.0, 0.0], 0.0));
    let body = body(&plan, &sight);
    let shown = seen(&body, sight.view);
    let task = PlacementTask {
        placement,
        plan,
        ground: given.ground,
        horizon: given.horizon,
        vanishing: [left, right, given.third],
        upright: given.upright,
        along,
        answer: shown.answer,
        hidden: shown.hidden,
        construction,
        sight,
    };
    (task, body)
}

/// A plain placement, the generator's last resort (never met by a seed tested): the first plan and
/// sight laid from a fixed sequence whatever their rules; else a bare ground seen from a fixed
/// sight.
fn plain(placement: Placement) -> PlacementTask {
    let mut random = Random::new(PLAIN);
    let down = !placement.upright_given();
    let found = (0..TRIES).find_map(|_| {
        let plan = Plan::laid(placement, &mut random)?;
        let sight = Sight::looked(&plan, down, &mut random)?;
        laid(placement, plan, sight)
    });
    found.map_or_else(
        || {
            let plan = Plan {
                ground: [4.0, 3.0],
                cells: 4,
                pieces: Vec::new(),
            };
            let sight = Sight {
                pitch: 0.0,
                yaw: 0.8,
                near: [-1.0, -3.0, 8.0],
                view: View {
                    scale: 0.6,
                    offset: [0.5, 0.2],
                },
            };
            let given = given(placement, &plan, &sight);
            let foot = given.upright[0];
            let empty = Construction {
                grid: Vec::new(),
                upright: [foot, foot],
                footprints: Vec::new(),
                carried: Vec::new(),
                tops: Vec::new(),
                runs: Vec::new(),
            };
            made(placement, (plan, sight), given, empty).0
        },
        |(task, _)| task,
    )
}
