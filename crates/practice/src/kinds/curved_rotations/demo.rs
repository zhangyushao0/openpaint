//! A curved structure rotation's demonstration (S154f), V大's steps on an example of its own beside
//! its picture: the box tight round the body in the picture and its axis, in pencil; the same box
//! at the new angle completed from its front corner, three edges and horizon; each section's
//! square found at its joint along the axis (`Marker::Construction`); the ellipses in the squares
//! and the outlines touching them; the visible edges in ink, the first a little off its place;
//! then Compare as the exercise shows it: what the eye sees red, what the object hides dashed, for
//! the eye. The copy's givens are printed grey; both frames are cropped to the two boxes.

use std::sync::Arc;

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, Shaded, bounds, line, tick, wobbled,
};
use crate::kinds::curved_rotations::curved::{CurvedRotation, CurvedTask};
use crate::kinds::curved_rotations::sections::Section;
use crate::kinds::curved_rotations::view::CurvedView;
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example: two segments, lying in the picture and standing asked, the joint ticked.
const SEED: u64 = 0xDE30_5EED_0154;
const EXAMPLE: u64 = 0;
/// How far a hand's line strays (frame units); the visible edges inked in the time of one; an
/// ellipse's pieces; a tick's half.
const STRAY: f64 = 0.004;
const INKED_TOGETHER: usize = 6;
const PIECES: usize = 72;
const TICK: f64 = 0.018;
/// Paper kept round the boxes where the example is cropped (frame units).
const AROUND: f64 = 0.04;

/// A curved structure rotation's demonstration.
pub(crate) fn curved() -> Option<Demo> {
    let task = CurvedTask::generate(CurvedRotation::ALL[0], derived(SEED, EXAMPLE));
    let (shown, asked) = (&task.shown, &task.asked);
    let mut script = Script::new();
    givens(&mut script, asked);
    // The box tight round the body in the picture, drawn through, and its axis.
    script.step();
    let mut around: Vec<(On, Marker, Drawn)> = (shown.seen.edges().into_iter())
        .map(|(a, b, _)| (On::Original, Marker::Pencil, line(a, b)))
        .collect();
    let axis = line(shown.axis[0], shown.axis[1]);
    around.push((On::Original, Marker::Pencil, axis));
    script.together(around);
    // The same box at the new angle.
    script.step();
    let given = asked.seen.given();
    let completed = (asked.seen.edges().into_iter())
        .filter(|(a, b, _)| !given.iter().any(|(p, q, _)| same(*a, *b, *p, *q)))
        .map(|(a, b, _)| (On::Copy, Marker::Pencil, line(a, b)))
        .collect();
    script.together(completed);
    sectioned(&mut script, asked);
    script.step();
    inked(&mut script, &task);
    let mut demo = compared(script, &task);
    let corners: Vec<Point> = (shown.seen.corners.iter())
        .chain(&asked.seen.corners)
        .chain(&shown.extent)
        .chain(&asked.extent)
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
    demo.shaded = Some(Shaded(Arc::new(task.shown.clone())));
    Some(demo)
}

/// What the example gives: its picture, and in the copy's frame the horizon, the box's front
/// corner and three edges, the axis and the joints ticked on it.
fn givens(script: &mut Script, asked: &CurvedView) {
    script.given(On::Original, Marker::Given, Drawn::Shaded);
    let height = asked.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for (a, b, _) in asked.seen.given() {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    let [foot, top] = asked.axis;
    script.given(On::Copy, Marker::PrintedHidden, line(foot, top));
    for at in &asked.joints {
        let towards = if *at == top { foot } else { top };
        let ticked = Drawn::Line(tick(*at, towards, TICK));
        script.given(On::Copy, Marker::Printed, ticked);
    }
}

/// Each section's square at its joint, the box cut across there; then the ellipses in them and
/// the outlines touching them.
fn sectioned(script: &mut Script, asked: &CurvedView) {
    script.step();
    let stages = asked.stages.iter();
    let lines = (stages.clone().flat_map(|stage| stage.lines.iter()))
        .map(|[a, b]| (On::Copy, Marker::Construction, line(*a, *b)))
        .collect();
    script.together(lines);
    let dots = (stages.flat_map(|stage| stage.found.iter().chain(&stage.middles)))
        .map(|p| (On::Copy, Marker::Construction, Drawn::Dot(*p)))
        .collect();
    script.together(dots);
    script.step();
    let ellipse = |s: &Section| Drawn::Line(s.guide.exact.outline(PIECES));
    let ellipses = (asked.sections.iter())
        .map(|s| (On::Copy, Marker::Pencil, ellipse(s)))
        .collect();
    script.together(ellipses);
    let outlines = (asked.answer.sides.iter())
        .map(|[a, b]| (On::Copy, Marker::Pencil, line(*a, *b)))
        .collect();
    script.together(outlines);
}

/// Whether the segments `a b` and `p q` are one, either way.
fn same(a: Point, b: Point, p: Point, q: Point) -> bool {
    let near = |x: Point, y: Point| (x[0] - y[0]).hypot(x[1] - y[1]) <= 1e-9;
    (near(a, p) && near(b, q)) || (near(a, q) && near(b, p))
}

/// The asked view's visible edges and outlines inked with Draw, a few a stroke's time, the first
/// edge `DRAWN_OFF` beside its place, so Compare's red shows it apart.
fn inked(script: &mut Script, task: &CurvedTask) {
    let answer = &task.asked.answer;
    let seen = (answer.curves.iter().flat_map(|curve| &curve.stretches)).filter(|s| !s.hidden);
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

/// Compare: what the eye sees red, what the object hides dashed.
fn compared(script: Script, task: &CurvedTask) -> Demo {
    let answer = &task.asked.answer;
    let mut marks: Vec<(On, Marker, Drawn)> = (answer.sides.iter())
        .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
        .collect();
    marks.extend((answer.hidden.iter()).map(|[a, b]| (On::Copy, Marker::Hidden, line(*a, *b))));
    for stretch in answer.curves.iter().flat_map(|curve| &curve.stretches) {
        let pen = if stretch.hidden {
            Marker::Hidden
        } else {
            Marker::Answer
        };
        marks.push((On::Copy, pen, Drawn::Line(stretch.points.clone())));
    }
    script.compare(DemoLayout::Pair, marks)
}
