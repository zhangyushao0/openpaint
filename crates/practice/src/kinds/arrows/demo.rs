//! An organic arrow's demonstration (S156l), Drawabox's way on an example of the level's own: the
//! path dashed grey and the arrowhead given; the width marked lightly in pencil at a few places
//! along the path, near wide and far narrow; both edges in long ink strokes, broken where a nearer
//! stretch passes in front, the first drifting too wide far away; the folds in ink where the ribbon
//! turns away; then Compare as the exercise shows it: the ribbon red, what it hides dashed.

use crate::demos::demo::{DRAWN_OFF, Demo, DemoLayout, Drawn, Marker, On, Script, line, wobbled};
use crate::kinds::arrows::ribbon::{Arrow, SAMPLES};
use crate::kinds::arrows::task::ArrowTask;
use crate::kinds::solids::solid_answer::Stretch;
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const SEED: u64 = 0xDE30_5EED_0156;
/// The places along the path the width is marked at, and how far a hand's line strays.
const MARKED: usize = 6;
const STRAY: f64 = 0.003;

/// An organic arrow's demonstration on an example of `arrow`.
pub(crate) fn arrow(arrow: Arrow) -> Demo {
    let task = ArrowTask::generate(arrow, derived(SEED, 0));
    let answer = &task.answer;
    let mut script = Script::new();
    script.given(
        On::Copy,
        Marker::PrintedHidden,
        Drawn::Line(task.path.clone()),
    );
    let [tip, a, b] = task.head;
    script.given(On::Copy, Marker::Printed, Drawn::Line(vec![tip, a, b, tip]));
    for [from, to] in &task.ticks {
        script.given(On::Copy, Marker::Printed, line(*from, *to));
    }
    // The width marked lightly along the path: near wide, far narrow.
    script.step();
    let marks = (1..MARKED)
        .map(|k| {
            let i = k * (SAMPLES - 1) / MARKED;
            let across = line(answer.edges[0][i], answer.edges[1][i]);
            (On::Copy, Marker::Pencil, across)
        })
        .collect();
    script.together(marks);
    // Each edge in long strokes, broken where a nearer stretch passes in front.
    script.step();
    for (side, stretches) in answer.stretches.iter().enumerate() {
        script.together(inked(stretches, side == 0));
    }
    // Where it turns away, the line it folds along.
    script.step();
    script.together(inked(answer.folds.iter().flatten(), false));
    let red = |stretch: &Stretch| {
        let pen = if stretch.hidden {
            Marker::Hidden
        } else {
            Marker::Answer
        };
        (On::Copy, pen, Drawn::Line(stretch.points.clone()))
    };
    let compared = (answer.stretches.iter().flatten())
        .chain(answer.folds.iter().flatten())
        .map(red)
        .collect();
    script.compare(DemoLayout::Single, compared)
}

/// The seen ones of `stretches` inked as a hand draws them; when `off`, the first drifting up off
/// its place towards its far end, up to `DRAWN_OFF` (the ribbon drawn too wide far away), so
/// Compare's red shows it apart.
fn inked<'a>(
    stretches: impl IntoIterator<Item = &'a Stretch>,
    off: bool,
) -> Vec<(On, Marker, Drawn)> {
    let seen = stretches.into_iter().filter(|s| !s.hidden);
    (seen.enumerate())
        .map(|(k, stretch)| {
            let last = stretch.points.len().saturating_sub(1).max(1) as f64;
            let by = |j: usize| {
                if off && k == 0 {
                    -DRAWN_OFF * j as f64 / last
                } else {
                    0.0
                }
            };
            let moved: Vec<Point> = (stretch.points.iter().enumerate())
                .map(|(j, p)| [p[0], p[1] + by(j)])
                .collect();
            let drawn = wobbled(&moved, STRAY, 0.3 + k as f64);
            (On::Copy, Marker::Ink, Drawn::Line(drawn))
        })
        .collect()
}
