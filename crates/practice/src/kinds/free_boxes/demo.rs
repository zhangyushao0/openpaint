//! Free boxes' demonstration (`demos/demo.rs`, week 2): two points on the horizon, a front
//! upright, its edges towards the points, the rest (one a little off), then compared as the
//! exercise compares it (S145): the horizon run on, faint, for the eye. A page that looks down or
//! up (S142) is shown on a three-point example, its guides given faint: its uprights lead to the
//! third point they show. A path page (S156l): three boxes sent back along its path, the nearest
//! big, each farther one smaller and turned its own way, each a box of the world standing on the
//! path seen by the page's eye; then the horizon run on.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, bounds, clearest, line};
use crate::demos::demo_space::{horizon, point, rule_edges, ruled, within_reach};
use crate::geometry::perspective::{Direction, EDGES, Horizon, Look, PerspectiveBox};
use crate::geometry::perspective_eye::{box_corners, directions};
use crate::kinds::free_boxes::marks::guides;
use crate::kinds::free_boxes::path::{EYE, PathPage};
use crate::maths::random::Random;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const FREE_SEED: u64 = 0xDE30_5EED_0008;
const PATH_SEED: u64 = 0xDE30_5EED_0156;
/// Where along the path the example's boxes stand (shares of its samples), their width and depth
/// and their height (world).
const ALONG: [f64; 3] = [0.04, 0.14, 0.26];
const SIDES: [f64; 2] = [0.22, 0.32];
const HEIGHT: [f64; 2] = [0.18, 0.3];

/// Free boxes on a horizon, the page's eye looking `look`: one drawn as the method draws it.
pub(crate) fn free_boxes(look: Look) -> Demo {
    let page = match look {
        Look::Level => Horizon::Middle,
        Look::Down => Horizon::HighDown,
        Look::Up => Horizon::LowUp,
    };
    let example = within_reach(FREE_SEED, |s| PerspectiveBox::on_horizon(page, s));
    let c = example.corners;
    let [left, right] = [Direction::Left, Direction::Right].map(|d| point(&example, d));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(example.horizon));
    if let Some(third) = point(&example, Direction::Vertical) {
        for [a, b] in guides(example.horizon, third) {
            script.given(On::Copy, Marker::Guide, line(a, b));
        }
    }
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
    rule_edges(&mut script, &example, &near, (0, None));
    script.step();
    // The last of the far edges drawn a little off, as a hand's may be.
    rule_edges(&mut script, &example, &far, (1, Some(far.len() - 1)));
    let run_on = (On::Copy, Marker::Faint, horizon(example.horizon));
    script.compare(DemoLayout::Wide, vec![run_on])
}

/// A path page's example: boxes sent back along its path, the nearest first.
pub(crate) fn path_page() -> Demo {
    let (page, boxes) = clearest((PATH_SEED, 16), path_boxes, |(_, boxes)| apart(boxes));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(page.horizon));
    for edge in &page.edges {
        script.given(On::Copy, Marker::Printed, Drawn::Line(edge.clone()));
    }
    for (k, example) in boxes.iter().enumerate() {
        script.step();
        let seen: Vec<(usize, usize)> = (EDGES.iter().zip(example.hidden()))
            .filter(|(_, hidden)| !hidden)
            .map(|((a, b, _), _)| (*a, *b))
            .collect();
        let (near, far): (Vec<_>, Vec<_>) = seen
            .iter()
            .partition(|(a, b)| [0, 2].contains(a) || [0, 2].contains(b));
        for edges in [near, far] {
            let ruled_edges = (edges.iter().enumerate())
                .map(|(turn, (a, b))| {
                    let (a, b) = (example.corners[*a], example.corners[*b]);
                    (On::Copy, Marker::Ink, ruled(a, b, turn + k))
                })
                .collect();
            script.together(ruled_edges);
        }
    }
    let run_on = (On::Copy, Marker::Faint, horizon(page.horizon));
    script.compare(DemoLayout::Single, vec![run_on])
}

/// The path page `seed` makes, and three boxes standing on its path at `ALONG`, each turned its
/// own way, as its eye sees them.
fn path_boxes(seed: u64) -> (PathPage, [PerspectiveBox; 3]) {
    let page = PathPage::generate(seed);
    let mut random = Random::new(seed);
    let ground = page.ground;
    let last = page.middle.len().saturating_sub(1);
    let boxes = ALONG.map(|along| {
        let index = (along * last as f64) as usize;
        let at = page.middle.get(index).copied().unwrap_or([0.0, -EYE, 3.0]);
        let side = |random: &mut Random| random.range(SIDES[0], SIDES[1]);
        let size = [
            side(&mut random),
            random.range(HEIGHT[0], HEIGHT[1]),
            side(&mut random),
        ];
        let yaw = random.range(25.0_f64, 65.0).to_radians();
        let middle = [at[0], -EYE + size[1] / 2.0, at[2]];
        let corners = box_corners(middle, yaw, size).map(|c| ground.frame(c));
        let [left, right, _] = directions(yaw);
        PerspectiveBox {
            corners,
            horizon: page.horizon,
            vanishing: [Some(ground.frame(left)), Some(ground.frame(right)), None],
            front: 2,
        }
    });
    (page, boxes)
}

/// How far apart the closest two of `boxes` lie on the page (their bounds; below 0 where they
/// overlap), so the example's boxes stand clear of one another.
fn apart(boxes: &[PerspectiveBox; 3]) -> f64 {
    let bound = |b: &PerspectiveBox| bounds(&b.corners).unwrap_or_default();
    let gap = |a: [[f64; 2]; 2], b: [[f64; 2]; 2]| {
        let x = (b[0][0] - a[1][0]).max(a[0][0] - b[1][0]);
        let y = (b[0][1] - a[1][1]).max(a[0][1] - b[1][1]);
        x.max(y)
    };
    let [a, b, c] = boxes.each_ref().map(bound);
    gap(a, b).min(gap(b, c)).min(gap(a, c))
}
