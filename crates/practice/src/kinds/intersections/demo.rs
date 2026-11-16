//! A form intersection's demonstration (S156k), Drawabox's way on an example of its own: a box into
//! a box, both drawn through; each edge carried into the other's face towards its vanishing point
//! (`Marker::Construction`); where an edge meets a face, the pierce points; the points joined
//! along the faces in ink, the first piece a little off; then Compare as the exercise shows it: the
//! line where they meet red, its hidden stretch dashed.

use crate::demos::demo::{DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line};
use crate::kinds::intersections::meeting::{IntersectionTask, Meeting};
use crate::maths::path::length;

/// The example's seed, and seeds of its sequence tried for the clearest.
const SEED: u64 = 0xDE30_5EED_1156;
const TRIES: u64 = 12;

/// A form intersection's demonstration.
pub(crate) fn intersections() -> Option<Demo> {
    let task = clearest(
        (SEED, TRIES),
        |seed| IntersectionTask::generate(Meeting::Edge, seed),
        |task| {
            let seen = task.answer.iter().filter(|s| !s.hidden);
            seen.map(|s| length(&s.points)).sum::<f64>()
        },
    );
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
    // Joined along the faces in ink, the first piece a little off.
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
    Some(script.compare(DemoLayout::Single, compared))
}
