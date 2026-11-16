//! An object's demonstration (`demo.rs`, S94), V大's way in his order beside the picture: the box
//! completed from its front corner, three edges and horizon; the biggest change's points by halves
//! and thirds, then the smaller part's, each change's construction dark while its step draws and
//! faint under the next (`Marker::Construction`); the visible edges in ink, the first a little off
//! its place; then Compare as the exercise shows it (S145): the visible edges red over the ink,
//! the construction faint under them, for the eye. The copy's givens are printed grey. The
//! picture, the lines and the points are `ObjectTask`'s.

use crate::demos::demo::{
    DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Pictured, Script, Shaded, bounds, line, wobbled,
};
use crate::kinds::objects::object::ObjectTask;
use crate::kinds::objects::object_design::Carve;
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example: a box with a slope cut down its far end and a notch out of its near corner, seen
/// from above; of the first 2,000 cuts of its seed's sequence with a second change seen so, the
/// one whose construction is fewest lines and points (6 and 11 lines, 3 and 9 points), so each
/// step reads on the card; a post's step is 29 lines at least, on a part a quarter to a third of
/// the box wide: too many to read there. Named, as laying 2,000 objects to find it would take
/// seconds.
const SEED: u64 = 0xDE30_5EED_0094;
const EXAMPLE: u64 = 1895;
/// How far a hand's line strays (frame units).
const STRAY: f64 = 0.004;
/// Visible edges inked in the time of one.
const INKED_TOGETHER: usize = 4;
/// Paper kept round the box where the example is cropped (frame units).
const AROUND: f64 = 0.04;

/// An object's demonstration: a box cut, then a smaller part taken out.
pub(crate) fn object() -> Option<Demo> {
    let task = ObjectTask::generate(Carve::Cut, derived(SEED, EXAMPLE));
    let [biggest, smaller] = [task.stages.first()?, task.stages.get(1)?];
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Shaded);
    let height = task.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for (a, b, _) in task.seen.given() {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    script.step();
    let given = task.seen.given();
    let completed = (task.seen.edges().into_iter())
        .filter(|(a, b, _)| !given.iter().any(|(p, q, _)| same(*a, *b, *p, *q)))
        .map(|(a, b, _)| (On::Copy, Marker::Pencil, line(a, b)))
        .collect();
    script.together(completed);
    // Each change's construction dark while its step draws, faint under the next.
    for stage in [biggest, smaller] {
        script.step();
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
    // Both frames cropped to the box, a little round it, so the example reads
    // large on the card.
    let seen: Vec<Point> = task.seen.corners.to_vec();
    demo.view = bounds(&seen).map(|[low, high]| {
        let side = (high[0] - low[0]).max(high[1] - low[1]) + 2.0 * AROUND;
        let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
        [
            [middle[0] - side / 2.0, middle[1] - side / 2.0],
            [middle[0] + side / 2.0, middle[1] + side / 2.0],
        ]
    });
    demo.shaded = Some(Shaded(std::sync::Arc::new(task)));
    Some(demo)
}

/// The example object's picture, as its exercise's original frame shows it.
impl Pictured for ObjectTask {
    fn picture(&self, side: u32) -> Vec<u8> {
        ObjectTask::picture(self, side)
    }

    fn same(&self, other: &dyn std::any::Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }
}

/// The visible edges and seen curves inked with Draw, a few a stroke's time (so an object's many
/// edges ink at a hand's pace overall), the first edge `DRAWN_OFF` beside its place, so Compare's
/// red shows it apart.
fn inked(script: &mut Script, task: &ObjectTask) {
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    let beside = |k: usize, [a, b]: [Point; 2]| {
        let long = (b[0] - a[0]).hypot(b[1] - a[1]).max(f64::MIN_POSITIVE);
        let by = if k == 0 { DRAWN_OFF / long } else { 0.0 };
        let across = [(a[1] - b[1]) * by, (b[0] - a[0]) * by];
        vec![
            [a[0] + across[0], a[1] + across[1]],
            [b[0] + across[0], b[1] + across[1]],
        ]
    };
    let strokes = (task.answer.sides.iter().enumerate())
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

/// Whether the segments `a b` and `p q` are one, either way.
fn same(a: Point, b: Point, p: Point, q: Point) -> bool {
    let near = |x: Point, y: Point| (x[0] - y[0]).hypot(x[1] - y[1]) <= 1e-9;
    (near(a, p) && near(b, q)) || (near(a, q) && near(b, p))
}

/// Compare: the visible edges red.
fn compared(script: Script, task: &ObjectTask) -> Demo {
    let mut marks: Vec<(On, Marker, Drawn)> = (task.answer.sides.iter())
        .map(|[a, b]| (On::Copy, Marker::Answer, line(*a, *b)))
        .collect();
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    marks.extend(seen.map(|stretch| {
        (
            On::Copy,
            Marker::Answer,
            Drawn::Line(stretch.points.clone()),
        )
    }));
    script.compare(DemoLayout::Pair, marks)
}
