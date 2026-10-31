//! The perspective checks (S84, S97): a completed box's edges taken for their vanishing points (an
//! exact edge heads for its point and gets no mark, a turned one alone is drawn on past its point,
//! missing it by what it is off), free boxes read box by box
//! (their directions found, a direction drawn apart said, a front corner under 90 degrees marked),
//! and how reliable that is on boxes drawn by hand. Run with `--nocapture` to read the figures.

use openpaint_practice::{
    BoxCheck, BoxView, Direction, Edge, FreeCheck, Horizon, Off, PerspectiveBox, Point, distance,
};

use crate::edges::{found, matches};
use crate::hand::{Hand, Manner, all_edges, from_points, placed, to_complete};

/// `edges` drawn as clean straight strokes.
fn clean(edges: &[(Point, Point, Direction)]) -> Vec<Vec<Point>> {
    edges.iter().map(|(a, b, _)| vec![*a, *b]).collect()
}

/// `edge` turned `degrees` about its first end.
fn turned((a, b, d): (Point, Point, Direction), degrees: f64) -> (Point, Point, Direction) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let v = [b[0] - a[0], b[1] - a[1]];
    (
        a,
        [
            a[0] + cos * v[0] - sin * v[1],
            a[1] + sin * v[0] + cos * v[1],
        ],
        d,
    )
}

/// The drawn edge `edge` was found as, with its direction.
fn truth(edge: &Edge, drawn: &[(Point, Point, Direction)]) -> Option<Direction> {
    (drawn.iter())
        .find(|(a, b, _)| matches(edge, (*a, *b)))
        .map(|(_, _, d)| *d)
}

/// How far the line through `from` and `to` passes from `point`, and how far `to` lies beyond
/// where it comes nearest.
fn passing([from, to]: [Point; 2], point: Point) -> (f64, f64) {
    let d = crate::edges::unit(from, to);
    let rel = [point[0] - from[0], point[1] - from[1]];
    let along = rel[0] * d[0] + rel[1] * d[1];
    let off = (rel[0] * d[1] - rel[1] * d[0]).abs();
    (off, distance(from, to) - along)
}

#[test]
fn exact_edges_head_for_their_points_and_get_no_marks() {
    for view in BoxView::ALL {
        for seed in 0..100 {
            let exact = PerspectiveBox::generate(view, seed);
            let drawn = to_complete(&exact);
            let check = BoxCheck::of(&exact, &found(&clean(&drawn)));
            assert!(
                check.edges.len() >= 8,
                "{view:?} {seed}: {}",
                check.edges.len()
            );
            assert_eq!(check.missed(), 0, "{view:?} {seed}");
            for checked in &check.edges {
                assert!(checked.on, "{view:?} {seed}: {checked:?}");
                assert_eq!(truth(&checked.edge, &drawn), Some(checked.direction));
                assert!(checked.off.is_none());
            }
        }
    }
}

#[test]
fn only_an_edge_that_misses_its_point_is_drawn_on_past_it() {
    let exact = PerspectiveBox::generate(BoxView::TwoPoint, 0x5EED_0084);
    let mut drawn = to_complete(&exact);
    let at = (drawn.iter())
        .position(|(_, _, d)| *d == Direction::Right)
        .expect("an edge to the right");
    drawn[at] = turned(drawn[at], 5.0);
    let check = BoxCheck::of(&exact, &found(&clean(&drawn)));
    let point = exact.vanishing[Direction::Right.index()].expect("the right point");
    let off: Vec<_> = check.edges.iter().filter(|c| !c.on).collect();
    assert_eq!((off.len(), check.missed()), (1, 1), "{off:?}");
    assert_eq!(off[0].direction, Direction::Right);
    let Some(Off::RunOn {
        line: extension,
        past: None,
    }) = off[0].off
    else {
        panic!("extended: {:?}", off[0].off)
    };
    let (missed_by, past) = passing(extension, point);
    assert!(
        missed_by > 0.03,
        "{extension:?} misses {point:?} by {missed_by}"
    );
    assert!((past - openpaint_practice::PAST).abs() < 1e-9, "{past}");
    let others = check.edges.iter().filter(|c| c.on);
    assert!(others.clone().count() >= 7);
    assert!(others.into_iter().all(|c| c.off.is_none()));
}

/// Boxes completed by `manner`'s hand, each edge drawn as meant or `error` degrees off: edges
/// checked, taken for their direction, found heading for their point.
fn completed(manner: Manner, error: f64) -> (usize, usize, usize) {
    let (mut checked, mut taken, mut on) = (0, 0, 0);
    for view in BoxView::ALL {
        for seed in 0..300 {
            let exact = PerspectiveBox::generate(view, seed);
            let drawn: Vec<_> = (to_complete(&exact).into_iter())
                .map(|edge| turned(edge, error))
                .collect();
            let pairs: Vec<(Point, Point)> = drawn.iter().map(|(a, b, _)| (*a, *b)).collect();
            let strokes = Hand::new(seed * 11 + 3, manner).draw(&pairs);
            for edge in BoxCheck::of(&exact, &found(&strokes)).edges {
                checked += 1;
                taken += usize::from(truth(&edge.edge, &drawn) == Some(edge.direction));
                on += usize::from(edge.on);
            }
        }
    }
    (checked, taken, on)
}

#[test]
fn completed_boxes_drawn_by_hand_are_checked_as_drawn() {
    for (name, manner) in [("steady", Manner::STEADY), ("shaky", Manner::SHAKY)] {
        let (checked, taken, on) = completed(manner, 0.0);
        let (wrong, wrong_taken, wrong_on) = completed(manner, 4.0);
        let share = |a: usize, b: usize| a as f64 / b as f64 * 100.0;
        println!(
            "{name}: drawn as meant, {on} of {checked} edges head for their point ({:.1} %), \
             {taken} taken for their own direction ({:.1} %); each 4° off, {} of {wrong} said \
             to miss ({:.1} %), {wrong_taken} taken for their own direction",
            share(on, checked),
            share(taken, checked),
            wrong - wrong_on,
            share(wrong - wrong_on, wrong),
        );
        assert!(share(taken, checked) >= 97.0, "{name}");
        assert!(share(on, checked) >= 95.0, "{name}");
        assert!(share(wrong - wrong_on, wrong) >= 98.0, "{name}");
    }
}

/// A page of boxes: up to three free-page examples on `horizon`, made smaller and laid left to
/// right a little apart; none when they do not fit.
fn page(horizon: Horizon, seed: u64) -> Option<Vec<PerspectiveBox>> {
    let mut boxes = Vec::new();
    let mut left = 0.08;
    for k in 0..3 {
        let example = PerspectiveBox::on_horizon(horizon, seed * 3 + k);
        let small = placed(&example, 0.5, 0.65);
        let low = small
            .corners
            .iter()
            .map(|c| c[0])
            .fold(f64::INFINITY, f64::min);
        let high = small.corners.iter().map(|c| c[0]).fold(0.0, f64::max);
        let laid = placed(&example, 0.5 + left - low, 0.65);
        left += high - low + 0.07;
        boxes.push(laid);
    }
    (left <= 1.0).then_some(boxes)
}

/// Pages of free boxes drawn by `manner`'s hand, as read: pages, those read as their boxes, edges
/// and those taken for their own direction, directions and those said apart, corners said narrow.
fn free_pages(manner: Manner) -> [usize; 7] {
    let mut counts = [0; 7];
    for horizon in Horizon::ALL {
        for seed in 0..100 {
            let Some(boxes) = page(horizon, seed) else {
                continue;
            };
            let drawn: Vec<_> = boxes.iter().flat_map(all_edges).collect();
            let pairs: Vec<(Point, Point)> = drawn.iter().map(|(a, b, _)| (*a, *b)).collect();
            let strokes = Hand::new(seed * 5 + 2, manner).draw(&pairs);
            let check = FreeCheck::of(horizon.height(), &found(&strokes));
            counts[0] += 1;
            counts[1] += usize::from(check.boxes.len() == boxes.len());
            for free in &check.boxes {
                counts[4] += free.meets.len();
                counts[5] += free.apart.len();
                counts[6] += free.narrow.len();
                for edge in &free.edges {
                    counts[2] += 1;
                    counts[3] += usize::from(truth(&edge.edge, &drawn) == Some(edge.direction));
                }
            }
        }
    }
    counts
}

#[test]
fn free_boxes_drawn_by_hand_are_read_box_by_box() {
    let share = |a: usize, b: usize| a as f64 / b as f64 * 100.0;
    // A shaky hand's short edges (these boxes are a page's, some 100 pixels) truly point some
    // degrees apart: their extensions show it, and so do the words.
    for (name, manner, least_taken, most_apart, most_narrow) in [
        ("steady", Manner::STEADY, 98.0, 4.0, 0),
        ("shaky", Manner::SHAKY, 88.0, 40.0, 3),
    ] {
        let [pages, read, edges, taken, directions, apart, narrow] = free_pages(manner);
        println!(
            "free boxes, {name}: {read} of {pages} pages read as their boxes ({:.1} %), {taken} \
             of {edges} edges taken for their direction ({:.1} %); {apart} of {directions} \
             directions said apart ({:.1} %: the hand's bow and tremor turn its lines) and \
             {narrow} corners said narrow, where the hand meant none",
            share(read, pages),
            share(taken, edges),
            share(apart, directions),
        );
        assert!(pages >= 200, "{pages}");
        assert!(share(read, pages) >= 97.0, "{name}");
        assert!(share(taken, edges) >= least_taken, "{name}");
        assert!(share(apart, directions) <= most_apart, "{name}");
        assert!(narrow <= most_narrow, "{name}");
    }
}

#[test]
fn a_front_corner_under_ninety_degrees_is_marked_and_a_direction_drawn_apart_said() {
    let distorted = from_points(
        [0.1, 0.3],
        [0.75, 0.3],
        [[0.5, 0.55], [0.5, 0.8]],
        [0.4, 0.4],
    );
    assert!(
        distorted.front_angle() < 70.0,
        "{}",
        distorted.front_angle()
    );
    let check = FreeCheck::of(0.3, &found(&clean(&all_edges(&distorted))));
    assert_eq!(check.boxes.len(), 1);
    let narrow = &check.boxes[0].narrow;
    assert_eq!(narrow.len(), 1, "{narrow:?}");
    assert!(distance(narrow[0].at, [0.5, 0.8]) < 0.005 && narrow[0].angle < 70.0);
    assert!(check.boxes[0].apart.is_empty());
    // A wide one: nothing marked. One left edge turned 6 degrees: its direction apart.
    let wide = from_points(
        [-0.6, 0.3],
        [1.5, 0.3],
        [[0.45, 0.5], [0.45, 0.75]],
        [0.25, 0.2],
    );
    let mut drawn = all_edges(&wide);
    let check = FreeCheck::of(0.3, &found(&clean(&drawn)));
    assert!(check.boxes[0].narrow.is_empty() && check.boxes[0].apart.is_empty());
    let at = (drawn.iter())
        .position(|(_, _, d)| *d == Direction::Left)
        .expect("a left edge");
    drawn[at] = turned(drawn[at], 6.0);
    let check = FreeCheck::of(0.3, &found(&clean(&drawn)));
    assert_eq!(check.boxes[0].apart, vec![Direction::Left]);
    // The other three still meet where they did: the turned one alone misses.
    let missing: Vec<_> = check.boxes[0].edges.iter().filter(|e| !e.on).collect();
    assert_eq!(missing.len(), 1, "{missing:?}");
    let (_, meet) = check.boxes[0].meets[0];
    assert!(distance(meet, [-0.6, 0.3]) < 1e-3, "{meet:?}");
}
