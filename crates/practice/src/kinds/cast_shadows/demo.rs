//! Cast shadows' demonstration (S153), on an example of its own in its one frame (a sun parallel to
//! the picture, its fixed seed's first): the horizon, the boxes and the pin given; with the pencil a
//! ray through each top corner at the pin's slant, then a ground line through the corner under it
//! along the pin's shadow; where they meet, the shadow's corners, its outline inked from the boxes'
//! feet; then Compare as the exercise lays it: the outline red, what the boxes hide dashed.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, line, wobbled};
use crate::kinds::cast_shadows::scene::{ShadowTask, Throw};
use crate::maths::random::derived;

/// The example's seed, and how far a hand's line strays (frame units).
const SEED: u64 = 0xDE30_5EED_0154;
const STRAY: f64 = 0.004;

/// Cast shadows' demonstration.
pub(crate) fn cast_shadows() -> Demo {
    let task = ShadowTask::generate(Throw::Side, derived(SEED, 0));
    let mut script = Script::new();
    let height = task.horizon();
    script.given(On::Copy, Marker::Guide, line([0.0, height], [1.0, height]));
    for seen in &task.boxes {
        for ((a, b, _), hidden) in seen.edges().into_iter().zip(seen.hidden()) {
            let pen = if hidden {
                Marker::PrintedHidden
            } else {
                Marker::Printed
            };
            script.given(On::Copy, pen, line(a, b));
        }
    }
    if let Some([foot, top, tip]) = task.pinned {
        script.given(On::Copy, Marker::Printed, line(foot, top));
        script.given(On::Copy, Marker::Printed, line(foot, tip));
        script.given(On::Copy, Marker::Printed, Drawn::Dot(top));
    }
    let corners = &task.shadows.corners;
    script.step();
    let rays = (corners.iter())
        .map(|c| (On::Copy, Marker::Pencil, line(c.top, c.shadow)))
        .collect();
    script.together(rays);
    script.step();
    let grounds = (corners.iter())
        .map(|c| (On::Copy, Marker::Pencil, line(c.foot, c.shadow)))
        .collect();
    script.together(grounds);
    script.step();
    let seen = (task.shadows.outline.iter()).filter(|(_, hidden)| !hidden);
    let inked = (seen.enumerate())
        .map(|(turn, (ends, _))| {
            let stroke = wobbled(ends, STRAY, 0.3 + turn as f64);
            (On::Copy, Marker::Ink, Drawn::Line(stroke))
        })
        .collect();
    script.together(inked);
    let compared = (task.shadows.outline.iter())
        .map(|(ends, hidden)| {
            let pen = if *hidden {
                Marker::Hidden
            } else {
                Marker::Answer
            };
            (On::Copy, pen, Drawn::Line(ends.to_vec()))
        })
        .collect();
    script.compare(DemoLayout::Single, compared)
}
