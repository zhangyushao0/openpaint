//! A contour exercise's demonstration (S156k), Drawabox's contour ellipses on an example of its own
//! beside its picture, both frames cropped to the tube: the spine followed over the picture in
//! pencil, where the tube turns towards the eye and away; at each tick the minor axis along the
//! spine; the ellipses round them in ink, touching the outline, rounder where the tube turns towards
//! the eye, the first a little off; then Compare as the exercise shows it: the exact ellipses red,
//! their hidden halves dashed.

use std::sync::Arc;

use crate::demos::demo::{DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, Shaded, wobbled};
use crate::kinds::contours::sections::Contour;
use crate::kinds::contours::task::{Bend, ContourTask};
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example: a gentle bend, its near end's contour given.
const SEED: u64 = 0xDE30_5EED_0156;
const EXAMPLE: u64 = 0;
/// How far a hand's line strays (frame units); an ellipse's pieces; how far past its ellipse a
/// minor axis is drawn, a share of it.
const STRAY: f64 = 0.003;
const PIECES: usize = 72;
const PAST: f64 = 1.5;
/// Paper kept round the tube where the example is cropped (frame units).
const AROUND: f64 = 0.04;

/// A contour exercise's demonstration.
pub(crate) fn contours() -> Option<Demo> {
    let task = ContourTask::generate(Bend::Gentle, derived(SEED, EXAMPLE));
    let mut script = Script::new();
    givens(&mut script, &task);
    // The spine over the picture: where the tube turns towards the eye, where away.
    script.step();
    script.draw(
        On::Original,
        Marker::Pencil,
        Drawn::Line(task.spine.clone()),
    );
    // At each tick, the minor axis along the spine.
    script.step();
    let axes = (task.contours.iter())
        .map(|c| (On::Copy, Marker::Pencil, Drawn::Line(minor_axis(c))))
        .collect();
    script.together(axes);
    // The ellipses round them in ink, the first a little off its place.
    script.step();
    for (k, contour) in task.contours.iter().enumerate() {
        let by = if k == 0 { DRAWN_OFF } else { 0.0 };
        let ellipse: Vec<Point> = (contour.whole(PIECES).into_iter())
            .map(|p| [p[0] + by, p[1] - by])
            .collect();
        let inked = wobbled(&ellipse, STRAY, 0.3 + k as f64);
        script.draw(On::Copy, Marker::Ink, Drawn::Line(inked));
    }
    let mut demo = compared(script, &task);
    // Both frames cropped to the tube, square, a little paper round it.
    let [low, high] = task.extent;
    let side = (high[0] - low[0]).max(high[1] - low[1]) + 2.0 * AROUND;
    let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
    demo.view = Some([
        [middle[0] - side / 2.0, middle[1] - side / 2.0],
        [middle[0] + side / 2.0, middle[1] + side / 2.0],
    ]);
    demo.shaded = Some(Shaded(Arc::new(task)));
    Some(demo)
}

/// What the example gives: its picture, and in the copy's frame its outline, spine, ticks and the
/// near end's contour.
fn givens(script: &mut Script, task: &ContourTask) {
    script.given(On::Original, Marker::Given, Drawn::Shaded);
    for line in &task.outline {
        script.given(On::Copy, Marker::Printed, Drawn::Line(line.clone()));
    }
    script.given(
        On::Copy,
        Marker::PrintedHidden,
        Drawn::Line(task.spine.clone()),
    );
    for tick in task.contours.iter().flat_map(|c| &c.ticks) {
        script.given(On::Copy, Marker::Printed, Drawn::Line(tick.to_vec()));
    }
    for stretch in task.example.iter().flat_map(|c| &c.stretches) {
        let pen = if stretch.hidden {
            Marker::PrintedHidden
        } else {
            Marker::Printed
        };
        script.given(On::Copy, pen, Drawn::Line(stretch.points.clone()));
    }
}

/// `contour`'s exact ellipse's minor axis, run a little past it either way.
fn minor_axis(contour: &Contour) -> Vec<Point> {
    let exact = contour.guide.exact;
    let (sin, cos) = exact.angle.sin_cos();
    let reach = exact.axes[1] * PAST;
    let [x, y] = [-sin * reach, cos * reach];
    let c = exact.centre;
    vec![[c[0] - x, c[1] - y], [c[0] + x, c[1] + y]]
}

/// Compare: the exact ellipses red, their hidden halves dashed.
fn compared(script: Script, task: &ContourTask) -> Demo {
    let marks = (task.contours.iter().flat_map(|c| &c.stretches))
        .map(|stretch| {
            let pen = if stretch.hidden {
                Marker::Hidden
            } else {
                Marker::Answer
            };
            (On::Copy, pen, Drawn::Line(stretch.points.clone()))
        })
        .collect();
    script.compare(DemoLayout::Pair, marks)
}
