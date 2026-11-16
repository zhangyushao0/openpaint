//! A structure rotation's demonstration (S113), V大's steps on an example of its own beside its
//! picture: the box tight round the object in the picture, in pencil; the same box at the new
//! angle completed from its front corner, three edges and horizon; the change's points by halves
//! and thirds (`Marker::Construction`); the visible edges in ink, the first a little off its place;
//! then Compare as the exercise shows it: the visible edges red and the hidden ones dashed over the
//! ink, for the eye. The copy's givens are printed grey; both frames are cropped to the two boxes.

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, Shaded, bounds, line, wobbled,
};
use crate::kinds::rotations::rotation::{Rotation, RotationTask};
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example: a box with a cut, turned a quarter (one change, so one step of construction),
/// the first of its seed's sequence.
const SEED: u64 = 0xDE30_5EED_0113;
const EXAMPLE: u64 = 0;
/// How far a hand's line strays (frame units), and the visible edges inked in the time of one.
const STRAY: f64 = 0.004;
const INKED_TOGETHER: usize = 4;
/// Paper kept round the boxes where the example is cropped (frame units).
const AROUND: f64 = 0.04;

/// A structure rotation's demonstration: a box with a cut, turned a quarter.
pub(crate) fn rotation() -> Option<Demo> {
    let task = RotationTask::generate(Rotation::ALL[1], derived(SEED, EXAMPLE));
    let (shown, asked) = (&task.shown, &task.asked);
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Shaded);
    let height = asked.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for (a, b, _) in asked.seen.given() {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    // The box tight round the object in the picture, drawn through.
    script.step();
    let around = (shown.seen.edges().into_iter())
        .map(|(a, b, _)| (On::Original, Marker::Pencil, line(a, b)))
        .collect();
    script.together(around);
    // The same box at the new angle.
    script.step();
    let given = asked.seen.given();
    let completed = (asked.seen.edges().into_iter())
        .filter(|(a, b, _)| !given.iter().any(|(p, q, _)| same(*a, *b, *p, *q)))
        .map(|(a, b, _)| (On::Copy, Marker::Pencil, line(a, b)))
        .collect();
    script.together(completed);
    // The change's points, by halves and thirds.
    script.step();
    for stage in &asked.stages {
        let lines = (stage.lines.iter())
            .map(|[a, b]| (On::Copy, Marker::Construction, line(*a, *b)))
            .collect();
        script.together(lines);
        let dots = (stage.found.iter().chain(&stage.middles))
            .map(|p| (On::Copy, Marker::Construction, Drawn::Dot(*p)))
            .collect();
        script.together(dots);
    }
    script.step();
    inked(&mut script, &task);
    let mut demo = compared(script, &task);
    let corners: Vec<Point> = (shown.seen.corners.iter())
        .chain(&asked.seen.corners)
        .copied()
        .collect();
    demo.view = bounds(&corners).map(|[low, high]| {
        let side = (high[0] - low[0]).max(high[1] - low[1]) + 2.0 * AROUND;
        let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
        [
            [middle[0] - side / 2.0, middle[1] - side / 2.0],
            [middle[0] + side / 2.0, middle[1] + side / 2.0],
        ]
    });
    demo.shaded = Some(Shaded(std::sync::Arc::new(task.shown.clone())));
    Some(demo)
}

/// Whether the segments `a b` and `p q` are one, either way.
fn same(a: Point, b: Point, p: Point, q: Point) -> bool {
    let near = |x: Point, y: Point| (x[0] - y[0]).hypot(x[1] - y[1]) <= 1e-9;
    (near(a, p) && near(b, q)) || (near(a, q) && near(b, p))
}

/// The asked view's visible edges and seen curves inked with Draw, a few a stroke's time, the
/// first edge `DRAWN_OFF` beside its place, so Compare's red shows it apart.
fn inked(script: &mut Script, task: &RotationTask) {
    let answer = &task.asked.answer;
    let seen = (answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    let beside = |k: usize, [a, b]: [Point; 2]| {
        let long = (b[0] - a[0]).hypot(b[1] - a[1]).max(f64::MIN_POSITIVE);
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
            .map(|line| (On::Copy, Marker::Ink, Drawn::Line(line.clone())))
            .collect();
        script.together(marks);
    }
}

/// Compare: the visible edges red, the hidden ones dashed.
fn compared(script: Script, task: &RotationTask) -> Demo {
    let asked = &task.asked;
    let mut marks: Vec<(On, Marker, Drawn)> = (asked.answer.sides.iter())
        .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
        .collect();
    marks.extend((asked.hidden.iter()).map(|[a, b]| (On::Copy, Marker::Hidden, line(*a, *b))));
    for stretch in asked.answer.outlines.iter().flatten() {
        let pen = if stretch.hidden {
            Marker::Hidden
        } else {
            Marker::Answer
        };
        marks.push((On::Copy, pen, Drawn::Line(stretch.points.clone())));
    }
    script.compare(DemoLayout::Pair, marks)
}
