//! The straight edges found in a hand's lines (S84): on known shapes (a square in one stroke, a
//! line drawn twice, overshooting, trembling; curves, which hold none), and how reliable the
//! finding is on boxes drawn as a hand draws them, steady and shaky: the share of the edges drawn
//! that are found, the share of edges found that were drawn, and how far a found edge's direction
//! lies from the one drawn. Run with `--nocapture` to read the figures.

use openpaint_practice::{
    BoxView, Edge, PerspectiveBox, Point, SHORTEST, corrected, distance, edges,
};

use crate::hand::{Hand, Manner, to_complete};

pub fn angle(a: Point, b: Point) -> f64 {
    let dot = a[0] * b[0] + a[1] * b[1];
    let cross = a[0] * b[1] - a[1] * b[0];
    let angle = cross.abs().atan2(dot).to_degrees();
    angle.min(180.0 - angle)
}

pub fn unit(a: Point, b: Point) -> Point {
    let long = distance(a, b);
    [(b[0] - a[0]) / long, (b[1] - a[1]) / long]
}

/// Whether `found` lies along the drawn edge `a`-`b`: within 3 degrees, its middle within 0.012
/// of the edge's line, overlapping it.
pub fn matches(found: &Edge, (a, b): (Point, Point)) -> bool {
    let d = unit(a, b);
    let m = found.middle();
    let off = ((m[0] - a[0]) * d[1] - (m[1] - a[1]) * d[0]).abs();
    let along = (m[0] - a[0]) * d[0] + (m[1] - a[1]) * d[1];
    angle(found.direction(), d) <= 3.0 && off <= 0.012 && (0.0..=distance(a, b)).contains(&along)
}

pub fn found(strokes: &[Vec<Point>]) -> Vec<Edge> {
    let lines: Vec<&[Point]> = strokes.iter().map(Vec::as_slice).collect();
    edges(&lines)
}

#[test]
fn a_square_in_one_stroke_is_four_edges_corner_to_corner() {
    let square = [[0.2, 0.2], [0.8, 0.2], [0.8, 0.7], [0.2, 0.7], [0.2, 0.2]];
    let found = found(&[square.to_vec()]);
    assert_eq!(found.len(), 4, "{found:?}");
    for (a, b) in square.windows(2).map(|p| (p[0], p[1])) {
        let side = found
            .iter()
            .find(|e| matches(e, (a, b)))
            .unwrap_or_else(|| panic!("{a:?}-{b:?} in {found:?}"));
        let ends_near = |p: Point| side.ends.iter().any(|e| distance(*e, p) < 0.006);
        assert!(ends_near(a) && ends_near(b), "{side:?}");
    }
}

#[test]
fn a_line_drawn_twice_overshooting_and_trembling_is_one_edge() {
    for seed in 0..50 {
        let mut hand = Hand::new(seed, Manner::SHAKY);
        let (a, b) = ([0.25, 0.3], [0.7, 0.55]);
        let strokes = vec![hand.line(a, b), hand.line([0.252, 0.302], [0.703, 0.551])];
        let found = found(&strokes);
        assert_eq!(found.len(), 1, "{seed}: {found:?}");
        assert!(
            angle(found[0].direction(), unit(a, b)) < 1.0,
            "{seed}: {found:?}"
        );
    }
}

#[test]
fn a_correction_takes_the_place_of_the_edge_it_redraws() {
    let earlier = found(&[vec![[0.2, 0.5], [0.6, 0.47]], vec![[0.2, 0.5], [0.2, 0.2]]]);
    let correction = found(&[vec![[0.21, 0.5], [0.6, 0.5]]]);
    let kept = corrected(earlier, correction.clone());
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert_eq!(kept[0], correction[0], "the correction");
    assert!(
        kept.iter()
            .any(|e| (e.ends[0][0] - e.ends[1][0]).abs() < 1e-9),
        "the vertical"
    );
}

#[test]
fn curves_hold_no_edge() {
    let circle: Vec<Point> = (0..=200)
        .map(|k| {
            let t = f64::from(k) / 200.0 * std::f64::consts::TAU;
            [0.5 + 0.2 * t.cos(), 0.5 + 0.2 * t.sin()]
        })
        .collect();
    assert!(found(&[circle]).is_empty());
    let wave: Vec<Point> = (0..=200)
        .map(|k| {
            let x = f64::from(k) / 200.0;
            [
                0.1 + 0.8 * x,
                0.5 + 0.08 * (x * std::f64::consts::TAU * 1.5).sin(),
            ]
        })
        .collect();
    assert!(found(&[wave]).is_empty());
    assert!(
        found(&[vec![[0.5, 0.5], [0.52, 0.51]]]).is_empty(),
        "too short"
    );
}

/// What the finding gave over many boxes drawn by `manner`'s hand: edges drawn (long enough to
/// keep) and found, edges found and matching one drawn, every matched edge's direction's error.
fn reliability(manner: Manner) -> (usize, usize, usize, usize, Vec<f64>) {
    let (mut drawn, mut hit, mut kept, mut true_found) = (0, 0, 0, 0);
    let mut errors = Vec::new();
    for view in BoxView::ALL {
        for seed in 0..300 {
            let exact = PerspectiveBox::generate(view, seed);
            let edges_drawn: Vec<(Point, Point)> = to_complete(&exact)
                .iter()
                .map(|(a, b, _)| (*a, *b))
                .collect();
            let mut hand = Hand::new(seed * 7 + 1, manner);
            let found = found(&hand.draw(&edges_drawn));
            for &(a, b) in &edges_drawn {
                if distance(a, b) < SHORTEST + 0.01 {
                    continue;
                }
                drawn += 1;
                if let Some(e) = found.iter().find(|e| matches(e, (a, b))) {
                    hit += 1;
                    errors.push(angle(e.direction(), unit(a, b)));
                }
            }
            kept += found.len();
            true_found += (found.iter())
                .filter(|e| edges_drawn.iter().any(|edge| matches(e, *edge)))
                .count();
        }
    }
    errors.sort_by(f64::total_cmp);
    (drawn, hit, kept, true_found, errors)
}

#[test]
fn edges_drawn_by_hand_are_found_reliably() {
    for (name, manner, least, p95) in [
        ("steady", Manner::STEADY, 0.97, 1.0),
        ("shaky", Manner::SHAKY, 0.95, 1.5),
    ] {
        let (drawn, hit, kept, true_found, errors) = reliability(manner);
        let found_share = hit as f64 / drawn as f64;
        let true_share = true_found as f64 / kept as f64;
        let at = |q: f64| errors[((errors.len() - 1) as f64 * q) as usize];
        println!(
            "{name}: {hit} of {drawn} edges found ({:.1} %), {true_found} of {kept} found were \
             drawn ({:.1} %); direction off by {:.2}° at the median, {:.2}° at 95 %, {:.2}° at most",
            found_share * 100.0,
            true_share * 100.0,
            at(0.5),
            at(0.95),
            errors[errors.len() - 1],
        );
        assert!(found_share >= least, "{name}: {found_share}");
        assert!(true_share >= 0.98, "{name}: {true_share}");
        assert!(at(0.95) <= p95, "{name}: {}", at(0.95));
    }
}
