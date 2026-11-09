//! A box's result (S119): completed on its answer it is clean, one edge turned past `ON_ANGLE`
//! near, more redo; with fewer than `EDGES_TO_ADD` edges beside the three given, incomplete, and
//! the given edges drawn over count for nothing. A steady hand's bow and tremor turn an edge past
//! `ON_ANGLE` now and then (`checks.rs`: some 5 % of edges): its boxes come out clean or near.

use openpaint_practice::{
    BoxCompared, BoxView, Direction, DrawnCopy, EDGES_TO_ADD, Outcome, PerspectiveBox, Point,
    Verdict,
};

use crate::hand::{Hand, Manner, to_complete};

/// `edge` turned `degrees` about its middle.
fn turned((a, b, d): (Point, Point, Direction), degrees: f64) -> (Point, Point, Direction) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let turn = |p: Point| {
        let v = [p[0] - m[0], p[1] - m[1]];
        [
            m[0] + cos * v[0] - sin * v[1],
            m[1] + sin * v[0] + cos * v[1],
        ]
    };
    (turn(a), turn(b), d)
}

/// The receding edges of `edges`, those lying farthest in angle from every other edge first: one
/// turned never comes into another's line and is read with it as one (`edges.rs`).
fn loneliest(edges: &[(Point, Point, Direction)]) -> Vec<usize> {
    let angle = |(a, b, _): &(Point, Point, Direction)| (b[1] - a[1]).atan2(b[0] - a[0]);
    let apart = |k: usize| {
        (0..edges.len())
            .filter(|j| *j != k)
            .map(|j| {
                let turn = (angle(&edges[k]) - angle(&edges[j]))
                    .to_degrees()
                    .rem_euclid(180.0);
                turn.min(180.0 - turn)
            })
            .fold(f64::INFINITY, f64::min)
    };
    let mut receding: Vec<usize> = (0..edges.len())
        .filter(|k| edges[*k].2 != Direction::Vertical)
        .collect();
    receding.sort_by(|i, j| apart(*j).total_cmp(&apart(*i)));
    receding
}

/// `exact` completed with `strokes`: its verdict.
fn verdict(exact: &PerspectiveBox, strokes: Vec<Vec<Point>>) -> Verdict {
    BoxCompared::of(
        exact,
        &DrawnCopy {
            all: strokes,
            corrected: None,
        },
    )
    .verdict()
}

/// `edges` drawn as clean straight strokes.
fn clean(edges: &[(Point, Point, Direction)]) -> Vec<Vec<Point>> {
    edges.iter().map(|(a, b, _)| vec![*a, *b]).collect()
}

#[test]
fn a_box_completed_is_clean_one_edge_off_near_more_redo_and_half_done_incomplete() {
    for view in BoxView::ALL {
        for seed in 0..50 {
            let label = format!("{view:?} {seed}");
            let exact = PerspectiveBox::generate(view, seed);
            let edges = to_complete(&exact);
            let clean_box = verdict(&exact, clean(&edges));
            assert_eq!(clean_box, Verdict::App(Outcome::Clean), "{label}");
            // Receding edges turned 6 degrees: a miss a hand's ghosting never makes.
            let receding = loneliest(&edges);
            let mut one = edges.clone();
            one[receding[0]] = turned(one[receding[0]], 6.0);
            assert_eq!(
                verdict(&exact, clean(&one)),
                Verdict::App(Outcome::Near),
                "{label}"
            );
            // Three (two may come into one line on a box seen level and read as one).
            let mut more = one.clone();
            for k in &receding[1..3] {
                more[*k] = turned(more[*k], -6.0);
            }
            assert_eq!(
                verdict(&exact, clean(&more)),
                Verdict::App(Outcome::Redo),
                "{label}"
            );
            // Stopped half-way, the given edges traced over again besides.
            let mut half = edges[..EDGES_TO_ADD - 2].to_vec();
            half.extend(exact.given());
            assert_eq!(
                verdict(&exact, clean(&half)),
                Verdict::Incomplete,
                "{label}"
            );
        }
    }
}

#[test]
fn a_steady_hand_s_boxes_come_out_clean_or_near() {
    let [mut clean_boxes, mut near, mut all] = [0; 3];
    for view in BoxView::ALL {
        for seed in 0..100 {
            let exact = PerspectiveBox::generate(view, seed);
            let pairs: Vec<(Point, Point)> = to_complete(&exact)
                .iter()
                .map(|(a, b, _)| (*a, *b))
                .collect();
            let strokes = Hand::new(seed, Manner::STEADY).draw(&pairs);
            let got = verdict(&exact, strokes);
            all += 1;
            clean_boxes += usize::from(got == Verdict::App(Outcome::Clean));
            near += usize::from(got == Verdict::App(Outcome::Near));
        }
    }
    println!("steady boxes: {clean_boxes} clean, {near} near of {all}");
    assert!(clean_boxes * 100 >= all * 95, "{clean_boxes} of {all}");
    assert!((clean_boxes + near) * 100 >= all * 99, "{near} near");
}
