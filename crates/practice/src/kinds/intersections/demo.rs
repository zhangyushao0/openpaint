//! A form intersection's demonstrations (S156k, S158k), one a level on an example of its own forms,
//! Drawabox's and V大's ways, then Compare as the exercise shows it: the line where they meet red,
//! its hidden stretch dashed.
//!
//! - Two boxes (入门): each edge carried into the other's face towards its vanishing point
//!   (`Marker::Construction`); where an edge meets a face, the pierce points; the points joined
//!   along the faces in ink, the first piece a little off.
//! - A cylinder through a box's face (熟练): on the face, the square the cylinder sits in and its
//!   diagonals; through their crossing, the middles of its sides; the ellipse through them in ink.
//! - A thin cylinder into a thick one's side (稳): a few sections across both forms; where each
//!   section's lines cross, a point of the line; the points joined in ink.

use crate::demos::demo::{DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line};
use crate::kinds::intersections::meeting::{IntersectionTask, Meeting};
use crate::maths::path::{Point, length};

/// The examples' seed, and seeds of its sequence tried for the clearest.
const SEED: u64 = 0xDE30_5EED_1156;
const TRIES: u64 = 12;

/// The demonstration of `meeting`'s level.
pub(crate) fn intersections(meeting: Meeting) -> Demo {
    match meeting {
        Meeting::Edge | Meeting::Corner => boxes(),
        Meeting::CylinderIntoBox | Meeting::BoxIntoCylinder => cylinder(),
        Meeting::Cylinders | Meeting::Sphere => round(),
    }
}

/// The clearest example of `meeting` among the seeds tried: the longest line seen.
fn example(meeting: Meeting) -> IntersectionTask {
    clearest(
        (SEED, TRIES),
        |seed| IntersectionTask::generate(meeting, seed),
        |task| {
            let seen = task.answer.iter().filter(|s| !s.hidden);
            seen.map(|s| length(&s.points)).sum::<f64>()
        },
    )
}

/// `task`'s horizon and both forms as printed.
fn printed(task: &IntersectionTask) -> Script {
    let mut script = Script::new();
    let height = task.seen.horizon;
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for stretch in &task.printed {
        let pen = if stretch.hidden {
            Marker::PrintedHidden
        } else {
            Marker::Printed
        };
        script.given(On::Copy, pen, Drawn::Line(stretch.points.clone()));
    }
    script
}

/// The line where they meet joined in ink as the eye sees it, the first piece a little off; then
/// Compare: the line red, its hidden stretch dashed.
fn joined(mut script: Script, task: &IntersectionTask) -> Demo {
    script.step();
    let seen = task.answer.iter().filter(|s| !s.hidden);
    for (k, stretch) in seen.enumerate() {
        let by = if k == 0 { DRAWN_OFF } else { 0.0 };
        let inked = stretch
            .points
            .iter()
            .map(|p| [p[0] + by, p[1] + by])
            .collect();
        script.draw(On::Copy, Marker::Ink, Drawn::Line(inked));
    }
    let compared = (task.answer.iter())
        .map(|stretch| {
            let pen = if stretch.hidden {
                Marker::Hidden
            } else {
                Marker::Answer
            };
            (On::Copy, pen, Drawn::Line(stretch.points.clone()))
        })
        .collect();
    script.compare(DemoLayout::Single, compared)
}

/// A box across another's edge: the edges carried, the pierce points, the line joined.
fn boxes() -> Demo {
    let task = example(Meeting::Edge);
    let mut script = printed(&task);
    // Each edge carried into the other's face, towards its vanishing point.
    script.step();
    let carried = (task.found.lines.iter())
        .map(|[a, b]| (On::Copy, Marker::Construction, line(*a, *b)))
        .collect();
    script.together(carried);
    // Where an edge meets a face: the pierce points.
    script.step();
    let pierced = (task.pierced.iter())
        .map(|p| (On::Copy, Marker::Construction, Drawn::Dot(*p)))
        .collect();
    script.together(pierced);
    joined(script, &task)
}

/// A cylinder through a box's face: the square it sits in there and its diagonals, the middles of
/// its sides, the ellipse through them.
fn cylinder() -> Demo {
    let task = example(Meeting::CylinderIntoBox);
    let mut script = printed(&task);
    let built = |lines: &[[Point; 2]]| -> Vec<(On, Marker, Drawn)> {
        (lines.iter())
            .map(|[a, b]| (On::Copy, Marker::Construction, line(*a, *b)))
            .collect()
    };
    let square = task.found.squares.first();
    script.step();
    if let Some([c0, c1, c2, c3]) = square.map(|square| square.corners) {
        script.together(built(&[[c0, c1], [c1, c2], [c2, c3], [c3, c0]]));
        script.together(built(&[[c0, c2], [c1, c3]]));
    }
    script.step();
    if let Some(t) = square.map(|square| square.touches) {
        script.together(built(&[[t[0], t[2]], [t[1], t[3]]]));
        let middles = t.map(|p| (On::Copy, Marker::Construction, Drawn::Dot(p)));
        script.together(middles.to_vec());
    }
    joined(script, &task)
}

/// A thin cylinder into a thick one's side: sections across both, where their lines cross, the
/// points joined.
fn round() -> Demo {
    let task = example(Meeting::Cylinders);
    let mut script = printed(&task);
    script.step();
    let sections = (task.found.lines.iter())
        .map(|[a, b]| (On::Copy, Marker::Construction, line(*a, *b)))
        .collect();
    script.together(sections);
    script.step();
    let crossed = (task.found.points.iter())
        .map(|p| (On::Copy, Marker::Construction, Drawn::Dot(*p)))
        .collect();
    script.together(crossed);
    joined(script, &task)
}
