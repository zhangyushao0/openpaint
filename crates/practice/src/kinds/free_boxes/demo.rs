//! Free boxes' demonstration (`demos/demo.rs`, week 2): two points on the horizon, a front
//! upright, its edges towards the points, the rest, then compared as the exercise compares it
//! (S97): an edge that misses its point drawn on past it, the points ringed red.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, line};
use crate::demos::demo_space::{horizon, point, red_rings, rule_edges, ruled, within_reach};
use crate::geometry::perspective::{Direction, EDGES, Horizon, PerspectiveBox};
use crate::sheets::exercise::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const FREE_SEED: u64 = 0xDE30_5EED_0008;

/// Free boxes on a horizon: one drawn as the method draws it.
pub(crate) fn free_boxes() -> Demo {
    let example = within_reach(FREE_SEED, |s| {
        PerspectiveBox::on_horizon(Horizon::Middle, s)
    });
    let c = example.corners;
    let [left, right] = [Direction::Left, Direction::Right].map(|d| point(&example, d));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(example.horizon));
    script.step();
    let rings = [left, right].into_iter().flatten();
    script.together(
        rings
            .map(|p| (On::Copy, Marker::Pencil, Drawn::Ring(p)))
            .collect(),
    );
    script.step();
    script.draw(On::Copy, Marker::Ink, ruled(c[0], c[2], 0));
    script.step();
    for corner in [0, 2] {
        let guides = [left, right].into_iter().flatten();
        script.together(
            guides
                .map(|p| (On::Copy, Marker::Pencil, line(c[corner], p)))
                .collect(),
        );
    }
    let edges = |pairs: &[(usize, usize)]| -> Vec<(usize, usize, Direction)> {
        (EDGES.into_iter())
            .filter(|(a, b, _)| pairs.contains(&(*a, *b)))
            .collect()
    };
    let near = edges(&[(0, 1), (2, 3), (0, 4), (2, 6)]);
    let far = edges(&[(1, 3), (4, 6), (5, 7), (1, 5), (3, 7), (4, 5), (6, 7)]);
    let mut compared = rule_edges(&mut script, &example, &near, (0, None));
    script.step();
    // The last of the far edges drawn a little off: it alone is drawn on at Compare.
    compared.extend(rule_edges(
        &mut script,
        &example,
        &far,
        (1, Some(far.len() - 1)),
    ));
    compared.extend(red_rings([left, right].into_iter().flatten()));
    script.compare(Family::FreeBoxes, DemoLayout::Wide, compared)
}
