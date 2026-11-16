//! A placement's demonstration (S153), V大's steps on an example of its own beside its plan (three
//! boxes on the ground's quarters): the ground's diagonals, then the lines of its halves and
//! quarters; each footprint's corners there (`Marker::Construction`); each height carried from the
//! upright along lines to the horizon, the tops found; the uprights and the tops in ink, the first
//! edge a little off its place; then Compare as the exercise shows it: the answer red, what the
//! boxes hide dashed, for the eye. What the exercise gives is printed grey.

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line, wobbled,
};
use crate::kinds::placements::placement::{Placement, PlacementTask};
use crate::kinds::placements::printed::{Printed, printed};
use crate::maths::path::{Point, distance};

/// The examples looked at, the first of a fixed seed's sequence; the clearest kept.
const SEED: u64 = 0xDE30_5EED_0153;
const EXAMPLES: u64 = 12;
/// How far apart an example's boxes stand on the frame before more counts for nothing.
const APART: f64 = 0.04;
/// How far a hand's line strays (frame units), and the edges inked in the time of one.
const STRAY: f64 = 0.004;
const INKED_TOGETHER: usize = 8;

/// A placement's demonstration: three boxes on the ground's quarters.
pub(crate) fn placement() -> Demo {
    let task = clearest(
        (SEED, EXAMPLES),
        |seed| PlacementTask::generate(Placement::ThreeBoxes, seed),
        clear,
    );
    let mut script = Script::new();
    given(&mut script, &task);
    let built = &task.construction;
    // The ground's halves and quarters by its diagonals.
    script.step();
    let pencil = |[a, b]: &[Point; 2]| (On::Copy, Marker::Pencil, line(*a, *b));
    script.together(built.grid[..2].iter().map(pencil).collect());
    script.together(built.grid[2..].iter().map(pencil).collect());
    // Each footprint's corners where the plan has them.
    script.step();
    let found = |p: &Point| (On::Copy, Marker::Construction, Drawn::Dot(*p));
    let outline = |feet: &[Point; 4]| {
        let round = feet.iter().chain(feet.first()).copied().collect();
        (On::Copy, Marker::Construction, Drawn::Line(round))
    };
    script.together(built.footprints.iter().map(outline).collect());
    script.together(built.footprints.iter().flatten().map(found).collect());
    // Each height from the upright along lines to the horizon.
    script.step();
    let carried = (built.carried.iter()).flat_map(|carried| carried.lines.iter());
    let construction = |[a, b]: &[Point; 2]| (On::Copy, Marker::Construction, line(*a, *b));
    script.together(carried.map(construction).collect());
    let tops = built.carried.iter().map(|carried| carried.top);
    script.together(tops.map(|top| found(&top)).collect());
    // The uprights, then the tops closed, in ink.
    script.step();
    inked(&mut script, &task);
    compared(script, &task)
}

/// How clear an example reads on a small card: its boxes standing apart on the frame (the least
/// gap between two boxes' bounds, up to `APART`), then its shortest footprint edge.
fn clear(task: &PlacementTask) -> f64 {
    let bounds: Vec<[Point; 2]> = (0..task.plan.pieces.len())
        .map(|k| {
            let corners: Vec<Point> = task.feet(k).into_iter().chain(task.tops(k)).collect();
            let low = [0, 1].map(|i| corners.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
            let high = [0, 1].map(|i| {
                (corners.iter())
                    .map(|p| p[i])
                    .fold(f64::NEG_INFINITY, f64::max)
            });
            [low, high]
        })
        .collect();
    let mut gap = APART;
    for (k, [low, high]) in bounds.iter().enumerate() {
        for [other_low, other_high] in &bounds[k + 1..] {
            let across = (other_low[0] - high[0]).max(low[0] - other_high[0]);
            let down = (other_low[1] - high[1]).max(low[1] - other_high[1]);
            gap = gap.min(across.max(down));
        }
    }
    let shortest = (task.construction.footprints.iter())
        .flat_map(|feet| (0..4).map(|k| distance(feet[k], feet[(k + 1) % 4])))
        .fold(f64::INFINITY, f64::min);
    gap + shortest
}

/// What the exercise gives: the plan beside, the horizon, the ground and the upright printed.
fn given(script: &mut Script, task: &PlacementTask) {
    for mark in printed(&task.plan, false) {
        let drawn = match mark {
            Printed::Line(points) => Drawn::Line(points),
            Printed::Area(points) => Drawn::Band {
                rows: rows(&points),
                colour: None,
            },
            Printed::Dot(at) => Drawn::Dot(at),
        };
        script.given(On::Original, Marker::Printed, drawn);
    }
    let height = task.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    let g = task.ground;
    for k in 0..4 {
        script.given(On::Copy, Marker::Printed, line(g[k], g[(k + 1) % 4]));
    }
    script.given(
        On::Copy,
        Marker::Printed,
        line(task.upright[0], task.upright[1]),
    );
}

/// A convex polygon's area as rows of two points, each row joined to the next.
fn rows(polygon: &[Point]) -> Vec<[Point; 2]> {
    let n = polygon.len();
    (0..n.div_ceil(2))
        .map(|k| [polygon[k], polygon[(n - 1 - k).max(k)]])
        .collect()
}

/// The edges and outlines the eye sees inked with Draw, a few a stroke's time, the first edge
/// `DRAWN_OFF` beside its place, so Compare's red shows it apart.
fn inked(script: &mut Script, task: &PlacementTask) {
    let answer = &task.answer;
    let seen = (answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    let beside = |k: usize, [a, b]: [Point; 2]| {
        let long = distance(a, b).max(f64::MIN_POSITIVE);
        let by = if k == 0 { DRAWN_OFF / long } else { 0.0 };
        let across = [(a[1] - b[1]) * by, (b[0] - a[0]) * by];
        vec![
            [a[0] + across[0], a[1] + across[1]],
            [b[0] + across[0], b[1] + across[1]],
        ]
    };
    let strokes = (answer.sides.iter().enumerate())
        .map(|(k, side)| beside(k, *side))
        .chain(seen.map(|stretch| stretch.points.clone()));
    let inked: Vec<Vec<Point>> = (strokes.enumerate())
        .map(|(turn, stroke)| wobbled(&stroke, STRAY, 0.3 + turn as f64))
        .collect();
    for group in inked.chunks(INKED_TOGETHER) {
        let marks = (group.iter())
            .map(|stroke| (On::Copy, Marker::Ink, Drawn::Line(stroke.clone())))
            .collect();
        script.together(marks);
    }
}

/// Compare: the edges and outlines seen red, the hidden ones dashed.
fn compared(script: Script, task: &PlacementTask) -> Demo {
    let mut marks: Vec<(On, Marker, Drawn)> = (task.answer.sides.iter())
        .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
        .collect();
    marks.extend((task.hidden.iter()).map(|[a, b]| (On::Copy, Marker::Hidden, line(*a, *b))));
    for stretch in task.answer.outlines.iter().flatten() {
        let pen = if stretch.hidden {
            Marker::Hidden
        } else {
            Marker::Answer
        };
        marks.push((On::Copy, pen, Drawn::Line(stretch.points.clone())));
    }
    script.compare(DemoLayout::Pair, marks)
}
