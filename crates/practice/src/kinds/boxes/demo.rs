//! A box's demonstration (`demos/demo.rs`, week 2): the given edges run on to their vanishing
//! points on the horizon, lines back from the points to the far corners, the nine edges inked
//! once, then compared as the exercise compares it (S97): the answer red, an edge that misses its
//! point drawn on past it; nothing on what is right.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, line};
use crate::demos::demo_space::{horizon, point, red_rings, rule_edges, within_reach};
use crate::geometry::perspective::{BoxView, Direction, EDGES, PerspectiveBox};
use crate::maths::path::distance;
use crate::sheets::exercise::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const BOX_SEED: u64 = 0xDE30_5EED_0007;

/// Lines in pencil back from `exact`'s vanishing point of `direction` to the far end of each of
/// its `missing` edges running that way.
fn back_lines(
    exact: &PerspectiveBox,
    missing: &[(usize, usize, Direction)],
    direction: Direction,
) -> Vec<(On, Marker, Drawn)> {
    let Some(p) = point(exact, direction) else {
        return Vec::new();
    };
    (missing.iter())
        .filter(|(_, _, d)| *d == direction)
        .map(|(a, b, _)| {
            let [a, b] = [exact.corners[*a], exact.corners[*b]];
            let far = if distance(a, p) > distance(b, p) {
                a
            } else {
                b
            };
            (On::Copy, Marker::Pencil, line(p, far))
        })
        .collect()
}

/// A box to complete.
pub(crate) fn perspective_box() -> Demo {
    let exact = within_reach(BOX_SEED, |s| PerspectiveBox::generate(BoxView::TwoPoint, s));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(exact.horizon));
    for (a, b, _) in exact.given() {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    script.step();
    let run: Vec<_> = (exact.given().into_iter())
        .filter_map(|(_, far, direction)| point(&exact, direction).map(|p| (far, p)))
        .collect();
    script.together(
        run.iter()
            .map(|(far, p)| (On::Copy, Marker::Pencil, line(*far, *p)))
            .collect(),
    );
    script.together(
        run.iter()
            .map(|(_, p)| (On::Copy, Marker::Pencil, Drawn::Ring(*p)))
            .collect(),
    );
    let front = exact.front;
    let missing: Vec<(usize, usize, Direction)> = EDGES
        .into_iter()
        .filter(|(a, b, _)| *a != front && *b != front)
        .collect();
    script.step();
    let back = |direction| back_lines(&exact, &missing, direction);
    let (lefts, rights) = (back(Direction::Left), back(Direction::Right));
    for (left, right) in lefts.into_iter().zip(rights) {
        script.together(vec![left, right]);
    }
    script.step();
    // One of the far edges drawn a little off: it alone is drawn on at Compare.
    let off = missing
        .iter()
        .rposition(|(_, _, d)| *d != Direction::Vertical);
    let missed = rule_edges(&mut script, &exact, &missing, (0, off));
    let edges = exact.edges().into_iter().zip(exact.hidden());
    let mut compared: Vec<(On, Marker, Drawn)> = edges
        .map(|((a, b, _), hidden)| {
            let pen = if hidden {
                Marker::Hidden
            } else {
                Marker::Answer
            };
            (On::Copy, pen, line(a, b))
        })
        .collect();
    compared.extend(red_rings(exact.vanishing.iter().flatten().copied()));
    compared.extend(missed);
    script.compare(Family::Box, DemoLayout::Wide, compared)
}
