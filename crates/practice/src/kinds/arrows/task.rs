//! An organic arrow's task (S156l): a ribbon (`ribbon.rs`) laid in the frame as large as it fits,
//! an arrowhead at its near end (a flat triangle in the ribbon's plane, pointing back at the eye),
//! the ribbon's answer (`answer.rs`), and what the copy's frame gives: the path dashed grey, the
//! arrowhead with where the edges leave it ticked (the near width), at 入门 the far width ticked.
//!
//! The scene's rules, seed after seed: everything inside the frame; the far width at most half
//! the near and still readable; every bend at least `BENDS_APART` from the next in the picture; at
//! 入门 no stretch crossing behind another (only a fold hides its own far side near it), at 熟练
//! and 稳 at least one; at 稳 one or two half turns, each folding once, no stretch passing in
//! front of it, its fold seen and at least `CLEAR` from every crossing in the picture; below 稳 no
//! half turn. Every bend folds the ribbon once too (the line it folds along, `answer.rs`).

use crate::kinds::arrows::answer::{ArrowAnswer, answered};
use crate::kinds::arrows::ribbon::{Arrow, Ribbon, SAMPLES, View, image};
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, times};

/// Ribbons laid before the generator keeps the next laid whatever it is (never met by a seed
/// tested).
const TRIES: usize = 300;
/// The picture's longer side in the frame, and how far inside it everything keeps.
const FILL: f64 = 0.84;
const INSIDE: f64 = 0.05;
/// The arrowhead's base's half over the ribbon's half width, and its length over it.
const BARB: f64 = 1.45;
const POINT: f64 = 1.9;
/// A tick's length (frame units).
const TICK: f64 = 0.016;
/// The far width at most this share of the near, and at least this (frame units).
const FAR_SHARE: f64 = 0.5;
const FAR_LEAST: f64 = 0.016;
/// Bends at least this far apart in the picture (frame units).
const BENDS_APART: f64 = 0.04;
/// A hidden edge sample whose hiding segment lies farther along than this share of the length is
/// a crossing: one stretch passing behind another, not a fold hiding its own far side.
const CROSSING: f64 = 0.1;
/// A half turn's fold at least this far from a crossing in the picture (frame units).
const CLEAR: f64 = 0.1;

/// An organic arrow, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowTask {
    pub arrow: Arrow,
    pub(crate) ribbon: Ribbon,
    pub(crate) view: View,
    pub answer: ArrowAnswer,
    /// The path the ribbon follows, its middle, in the frame.
    pub path: Vec<Point>,
    /// The arrowhead: its point, then its base's ends.
    pub head: [Point; 3],
    /// Where the edges leave the head (the near width) and where they end (the far width).
    pub near: [Point; 2],
    pub far: [Point; 2],
    /// The near width ticked where the edges leave the head, along the path; at 入门 the far
    /// width ticked outward.
    pub ticks: Vec<[Point; 2]>,
}

impl ArrowTask {
    /// The arrow `arrow` and `seed` make.
    #[must_use]
    pub fn generate(arrow: Arrow, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut tried = 0;
        loop {
            let Some(ribbon) = Ribbon::laid(arrow, &mut random) else {
                continue;
            };
            let task = Self::seen(arrow, ribbon);
            tried += 1;
            if tried >= TRIES || task.keeps() {
                return task;
            }
        }
    }

    /// `ribbon` laid in the frame with its head, its answer and its givens.
    pub(crate) fn seen(arrow: Arrow, ribbon: Ribbon) -> Self {
        let head = head(&ribbon);
        let half = ribbon.half;
        let images: Vec<Point> = (0..SAMPLES)
            .flat_map(|i| [half, -half].map(|u| image(ribbon.across(i, u))))
            .chain(head.map(image))
            .collect();
        let view = View::fitted(&images, FILL);
        let answer = answered(&ribbon, &view, head);
        let path = (ribbon.samples.iter()).map(|s| view.frame(s.at)).collect();
        let head = head.map(|p| view.frame(p));
        let last = SAMPLES - 1;
        let near = [answer.edges[0][0], answer.edges[1][0]];
        let far = [answer.edges[0][last], answer.edges[1][last]];
        let ticks = ticks(arrow, &answer, (near, far));
        Self {
            arrow,
            ribbon,
            view,
            answer,
            path,
            head,
            near,
            far,
            ticks,
        }
    }

    /// The hidden edge samples behind a stretch farther along than `CROSSING` (one stretch passing
    /// behind another, not a fold hiding its own far side near it), each with the segment hiding
    /// it.
    pub(crate) fn behind_far(&self) -> Vec<(usize, usize)> {
        let reach = CROSSING * (SAMPLES - 1) as f64;
        (self.answer.behind.iter().flatten())
            .filter(|(i, hiding)| i.abs_diff(*hiding) as f64 > reach)
            .copied()
            .collect()
    }

    /// Where one stretch crosses another in the picture: each place an edge goes behind a stretch
    /// farther along than `CROSSING` or comes out from behind it (its sample's point in the frame).
    pub(crate) fn crossings(&self) -> Vec<Point> {
        let reach = CROSSING * (SAMPLES - 1) as f64;
        let mut places = Vec::new();
        for (side, behind) in self.answer.behind.iter().enumerate() {
            let seen = &self.answer.seen[side];
            let open = |i: Option<usize>| i.and_then(|i| seen.get(i)).is_some_and(|s| *s);
            for &(i, hiding) in behind {
                let edge = open(i.checked_sub(1)) || open(Some(i + 1));
                if edge && i.abs_diff(hiding) as f64 > reach {
                    places.push(self.answer.edges[side][i]);
                }
            }
        }
        places
    }

    /// The folds a half turn makes: those whose places lie within a half turn's span.
    pub(crate) fn turned_folds(&self) -> Vec<usize> {
        (self.answer.placed.iter().enumerate())
            .filter(|(_, at)| (at.iter()).any(|a| self.ribbon.turning(a.round().max(0.0) as usize)))
            .map(|(k, _)| k)
            .collect()
    }

    /// Whether it keeps the scene's rules.
    pub(crate) fn keeps(&self) -> bool {
        let inside = |p: &Point| p.iter().all(|v| (INSIDE..=1.0 - INSIDE).contains(v));
        let answer = &self.answer;
        let points = (answer.edges.iter().flatten()).chain(&self.head);
        let ticked = self.ticks.iter().flatten();
        let within = points.chain(ticked).all(inside);
        let (near, far) = (
            distance(self.near[0], self.near[1]),
            distance(self.far[0], self.far[1]),
        );
        let widths = far <= FAR_SHARE * near && far >= FAR_LEAST;
        let bent: Vec<Point> = self.ribbon.bends.iter().map(|&i| self.path[i]).collect();
        let apart = bent.windows(2).all(|w| distance(w[0], w[1]) >= BENDS_APART);
        let crossings = self.crossings();
        let crossed = match self.arrow {
            Arrow::Gentle => self.behind_far().is_empty(),
            Arrow::Crossing | Arrow::Turning => !crossings.is_empty(),
        };
        within && widths && apart && crossed && self.turned_well(&crossings)
    }

    /// Whether its half turns are as its variant asks: at 稳 one or two, each folding once, no
    /// stretch passing in front of its span, its fold seen and clear of every crossing; else none.
    fn turned_well(&self, crossings: &[Point]) -> bool {
        let turns = self.ribbon.turns.len();
        let folds = self.turned_folds();
        if self.arrow != Arrow::Turning {
            return turns == 0 && folds.is_empty();
        }
        let answer = &self.answer;
        let open = !(self.behind_far().iter()).any(|(i, _)| self.ribbon.turning(*i));
        let clear = |k: &usize| {
            let fold = &answer.folds[*k];
            let seen = fold.iter().all(|s| !s.hidden);
            let points = fold.iter().flat_map(|s| &s.points);
            seen && (points.flat_map(|p| crossings.iter().map(|c| distance(*p, *c))))
                .all(|apart| apart >= CLEAR)
        };
        (1..=2).contains(&turns) && folds.len() == turns && open && folds.iter().all(clear)
    }
}

/// The arrowhead at `ribbon`'s near end, in its plane: its point back along the path towards the
/// eye, then its base's ends either side across it (world).
fn head(ribbon: &Ribbon) -> [Vector; 3] {
    let start = &ribbon.samples[0];
    let half = ribbon.half;
    let across = |by: f64| add(start.at, times(start.normal, by * half));
    [
        add(start.at, times(start.tangent, -POINT * half)),
        across(BARB),
        across(-BARB),
    ]
}

/// The near width ticked where the edges leave the head, along the path; at 入门 the far width
/// ticked outward from each edge's end.
fn ticks(
    arrow: Arrow,
    answer: &ArrowAnswer,
    (near, far): ([Point; 2], [Point; 2]),
) -> Vec<[Point; 2]> {
    let way = |from: Point, to: Point| {
        let long = distance(from, to).max(f64::MIN_POSITIVE);
        [(to[0] - from[0]) / long, (to[1] - from[1]) / long]
    };
    let step = |p: Point, d: Point, by: f64| [p[0] + d[0] * by, p[1] + d[1] * by];
    let mut ticks: Vec<[Point; 2]> = (0..2)
        .map(|side| {
            let along = way(answer.edges[side][0], answer.edges[side][1]);
            [near[side], step(near[side], along, TICK)]
        })
        .collect();
    if arrow == Arrow::Gentle {
        let outward = way(far[1], far[0]);
        ticks.push([far[0], step(far[0], outward, TICK)]);
        ticks.push([far[1], step(far[1], outward, -TICK)]);
    }
    ticks
}
