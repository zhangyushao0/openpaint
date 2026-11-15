//! Three-point free pages (S142): the eye looking down from a high horizon or up from a low one,
//! every box's uprights read against the page's third point, given (two faint guides lead to it)
//! or found where the uprights drawn pass nearest; the page's own where they meet on no side its
//! eye looks or nowhere; an upright that misses gets its true upright through it. Run with
//! `--nocapture` to read the figures.

use openpaint_practice::{
    Direction, Exercise, FreeCheck, FreePage, Horizon, Kind, Off, PerspectiveBox, Point, Uprights,
    distance, guides,
};

use crate::checks::{clean, truth};
use crate::edges::found;
use crate::hand::{Hand, Manner, all_edges, from_points, from_three_points};

const THREE_POINT: [Horizon; 4] = [
    Horizon::HighDown,
    Horizon::LowUp,
    Horizon::HighDownBare,
    Horizon::LowUpBare,
];

/// The page `horizon` and `seed` make, and its third point.
fn page(horizon: Horizon, seed: u64) -> (FreePage, Point) {
    let example = PerspectiveBox::on_horizon(horizon, seed);
    let third = example.vanishing[Direction::Vertical.index()].expect("three-point");
    (FreePage(example, horizon), third)
}

/// How far `p` lies from the line through `a` and `b`.
fn off_line(p: Point, [a, b]: [Point; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs() / d[0].hypot(d[1])
}

#[test]
fn a_guided_page_s_guides_lead_to_its_third_point_and_a_bare_page_has_none() {
    for horizon in THREE_POINT {
        for seed in 0..100 {
            let label = format!("{horizon:?} {seed}");
            let (page, third) = page(horizon, seed);
            let given = Exercise::generated(Kind::FreeBoxes(horizon), seed).original();
            assert_eq!(given.givens().len(), if horizon.guided() { 3 } else { 1 });
            let [left, right] = guides(page.0.horizon, third);
            for guide in [left, right] {
                assert!(off_line(third, guide) < 1e-9, "{label}: {guide:?}");
                let inside = |p: &Point| p.iter().all(|v| (0.06..=0.94).contains(v));
                assert!(guide.iter().all(inside), "{label}: {guide:?}");
                assert!(distance(guide[0], guide[1]) > 0.4, "{label}: {guide:?}");
            }
            // They draw together towards the point: plainly, at the sheet's view.
            let apart = |k: usize| distance(left[k], right[k]);
            assert!(
                apart(0) - apart(1) > 0.04,
                "{label}: {} {}",
                apart(0),
                apart(1)
            );
        }
    }
}

#[test]
fn a_page_s_example_drawn_exactly_reads_every_upright_on_its_third_point() {
    for horizon in THREE_POINT {
        for seed in 0..100 {
            let label = format!("{horizon:?} {seed}");
            let (page, third) = page(horizon, seed);
            let uprights = page.uprights();
            assert_eq!(matches!(uprights, Uprights::Given(_)), horizon.guided());
            // Its visible edges (a hidden upright behind the front one, seen corner on, would read
            // as one edge with it): given, the page's point; found, where they meet, the same.
            let visible = (page.0.edges().into_iter().zip(page.0.hidden()))
                .filter(|(_, hidden)| !hidden)
                .map(|(edge, _)| edge);
            let drawn: Vec<(Point, Point, Direction)> = visible.collect();
            let check = FreeCheck::on_page(page.0.horizon, uprights, &found(&clean(&drawn)));
            let met = check.third.expect("a third point");
            let off = distance(met, third) / distance(third, [0.5, 0.5]);
            assert!(off < 1e-6, "{label}: {met:?} {third:?}");
            assert_eq!(check.boxes.len(), 1, "{label}");
            let edges = &check.boxes[0].edges;
            let upright = edges.iter().filter(|e| e.direction == Direction::Vertical);
            assert_eq!(upright.clone().count(), 3, "{label}: its three visible");
            assert!(upright.clone().all(|e| e.on && e.off.is_none()), "{label}");
            for edge in edges {
                assert_eq!(truth(&edge.edge, &drawn), Some(edge.direction), "{label}");
            }
        }
    }
}

/// Pages of one box each drawn by `manner`'s hand: pages, read as one box, uprights, uprights
/// taken as uprights, those on their third point, bare pages whose point was found (not the page's
/// own) on the side the eye looks.
fn by_hand(manner: Manner) -> [usize; 6] {
    let mut counts = [0; 6];
    for horizon in THREE_POINT {
        for seed in 0..100 {
            let (page, third) = page(horizon, seed);
            let drawn = all_edges(&page.0);
            let pairs: Vec<(Point, Point)> = drawn.iter().map(|(a, b, _)| (*a, *b)).collect();
            let strokes = Hand::new(seed * 13 + 5, manner).draw(&pairs);
            let check = FreeCheck::on_page(page.0.horizon, page.uprights(), &found(&strokes));
            counts[0] += 1;
            counts[1] += usize::from(check.boxes.len() == 1);
            for edge in check.boxes.iter().flat_map(|free| &free.edges) {
                let meant = truth(&edge.edge, &drawn);
                if meant == Some(Direction::Vertical) {
                    counts[2] += 1;
                    counts[3] += usize::from(edge.direction == Direction::Vertical);
                    counts[4] += usize::from(edge.on);
                }
            }
            let own = check.third.is_some_and(|p| distance(p, third) > 1e-9);
            counts[5] += usize::from(!horizon.guided() && own);
        }
    }
    counts
}

#[test]
fn a_hand_s_uprights_on_a_three_point_page_are_read_as_uprights() {
    let share = |a: usize, b: usize| a as f64 / b as f64 * 100.0;
    for (name, manner, least_on) in [
        ("steady", Manner::STEADY, 85.0),
        ("shaky", Manner::SHAKY, 60.0),
    ] {
        let [pages, read, uprights, taken, on, own] = by_hand(manner);
        println!(
            "three-point pages, {name}: {read} of {pages} read as their box, {taken} of \
             {uprights} uprights taken as uprights ({:.1} %), {on} on their point ({:.1} %); \
             {own} of {} bare pages' points found from the uprights",
            share(taken, uprights),
            share(on, uprights),
            pages / 2,
        );
        assert!(share(read, pages) >= 97.0, "{name}");
        assert!(share(taken, uprights) >= 98.0, "{name}");
        assert!(share(on, uprights) >= least_on, "{name}");
    }
}

#[test]
fn uprights_kept_vertical_or_meeting_on_the_wrong_side_lean_against_the_page_s_own_point() {
    let (page, third) = page(Horizon::HighDownBare, 3);
    let h = page.0.horizon;
    let uprights = page.uprights();
    // Drawn as a two-point box at the page's left: its uprights meet nowhere; the page's own
    // point is taken, and every upright that misses it gets its true upright, heading for it.
    let level = from_points([-0.4, h], [1.3, h], [[0.2, 0.5], [0.2, 0.78]], [0.25, 0.2]);
    let check = FreeCheck::on_page(h, uprights, &found(&clean(&all_edges(&level))));
    assert_eq!(check.third, Some(third));
    let leaning: Vec<_> = (check.boxes.iter().flat_map(|free| &free.edges))
        .filter(|e| e.direction == Direction::Vertical && !e.on)
        .collect();
    assert!(leaning.len() >= 3, "{leaning:?}");
    for edge in leaning {
        let Some(Off::Plumb(line)) = edge.off else {
            panic!("a true upright: {edge:?}")
        };
        assert!(off_line(third, line) < 1e-9, "it heads for the point");
    }
    // Uprights meeting far below, at a point of the learner's own: theirs, every upright on it.
    let own = [0.42, 2.7];
    let toward = |p: Point, by: f64| {
        let d = [own[0] - p[0], own[1] - p[1]];
        let long = d[0].hypot(d[1]);
        [p[0] + d[0] / long * by, p[1] + d[1] / long * by]
    };
    let top = [0.3, 0.5];
    let three = from_three_points(
        [[-0.4, h], [1.3, h], own],
        [top, toward(top, 0.26)],
        [0.3, 0.2],
    );
    let check = FreeCheck::on_page(h, uprights, &found(&clean(&all_edges(&three))));
    let met = check.third.expect("found");
    assert!(distance(met, own) < 0.01, "{met:?}");
    let edges = check.boxes.iter().flat_map(|free| &free.edges);
    assert!(
        edges
            .filter(|e| e.direction == Direction::Vertical)
            .all(|e| e.on)
    );
    // The same meeting above, on a page that looks down: the page's own.
    let above = [0.42, -2.0];
    let up = |p: Point, by: f64| {
        let d = [above[0] - p[0], above[1] - p[1]];
        let long = d[0].hypot(d[1]);
        [p[0] - d[0] / long * by, p[1] - d[1] / long * by]
    };
    let three = from_three_points(
        [[-0.4, h], [1.3, h], above],
        [top, up(top, 0.26)],
        [0.3, 0.2],
    );
    let check = FreeCheck::on_page(h, uprights, &found(&clean(&all_edges(&three))));
    assert_eq!(check.third, Some(third));
}
