//! Boxes in perspective (S84): the exact boxes. Each is the same for the same seed; its edges'
//! lines run through their vanishing points, the horizontal ones on the horizon, a two-point
//! box's verticals vertical, a three-point box's meeting above or below; every box keeps V大's
//! rules (a front angle over 90 degrees, vanishing points neither too near nor too far, in the
//! frame); free-box pages put their example on their horizon.

use openpaint_practice::{
    BoxView, Direction, EDGES, Exercise, FAR, Horizon, Kind, Original, PerspectiveBox, Point,
    distance,
};

/// How far the point `p` lies from the line through `a` and `b`.
fn off_line(p: Point, a: Point, b: Point) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs() / d[0].hypot(d[1])
}

/// Every rule a box keeps, `given` its front corner's edges long enough to read.
fn keeps_its_rules(b: &PerspectiveBox, given: bool, label: &str) {
    let inside = |v: f64| (0.06..=0.94).contains(&v);
    assert!(inside(b.horizon), "{label}: horizon {}", b.horizon);
    for c in b.corners {
        assert!(inside(c[0]) && inside(c[1]), "{label}: corner {c:?}");
    }
    for (a, z, direction) in b.edges() {
        match b.vanishing[direction.index()] {
            Some(p) => assert!(off_line(p, a, z) < 1e-9, "{label}: {direction:?} misses"),
            None => assert!((a[0] - z[0]).abs() < 1e-12, "{label}: a vertical leans"),
        }
    }
    let middle = [0.5, 0.5];
    for side in [Direction::Left, Direction::Right] {
        let p = b.vanishing[side.index()].expect("a horizontal direction's point");
        assert!((p[1] - b.horizon).abs() < 1e-12, "{label}: off the horizon");
        assert!(distance(p, middle) <= FAR, "{label}: {side:?} too far");
        assert!(
            distance(p, b.corners[b.front]) >= 0.55,
            "{label}: {side:?} too near"
        );
    }
    let angle = b.front_angle();
    assert!(
        (100.0..=150.0).contains(&angle),
        "{label}: front angle {angle}"
    );
    if given {
        let edges = b.given();
        assert_eq!(edges.len(), 3, "{label}");
        for (from, to, _) in edges {
            assert_eq!(from, b.corners[b.front]);
            assert!(
                distance(from, to) >= 0.12,
                "{label}: a given edge too short"
            );
        }
    }
}

#[test]
fn every_box_runs_to_its_vanishing_points_and_keeps_v_da_s_rules() {
    for view in BoxView::ALL {
        for seed in 0..500 {
            let label = format!("{view:?} {seed}");
            let b = PerspectiveBox::generate(view, seed);
            assert_eq!(b, PerspectiveBox::generate(view, seed), "{label}: per seed");
            keeps_its_rules(&b, true, &label);
            let third = b.vanishing[Direction::Vertical.index()];
            match view {
                BoxView::TwoPoint => assert_eq!(third, None, "{label}"),
                BoxView::ThreePoint => {
                    let p = third.expect("a third point");
                    let far = distance(p, [0.5, 0.5]);
                    assert!((1.3..=4.0).contains(&far), "{label}: third at {far}");
                    // Looking up the verticals meet above and the box's bottom shows (its front
                    // corner the lower one); looking down, below, the top.
                    let up = p[1] < b.horizon;
                    assert_eq!(b.front, if up { 0 } else { 2 }, "{label}");
                }
            }
        }
    }
}

#[test]
fn boxes_vary_in_angle_proportion_and_where_they_lie_against_the_horizon() {
    let boxes: Vec<PerspectiveBox> = (0..300)
        .map(|seed| PerspectiveBox::generate(BoxView::TwoPoint, seed))
        .collect();
    let angles: Vec<f64> = boxes.iter().map(PerspectiveBox::front_angle).collect();
    let low = angles.iter().copied().fold(f64::INFINITY, f64::min);
    let high = angles.iter().copied().fold(0.0, f64::max);
    assert!(low < 110.0 && high > 135.0, "{low} to {high}");
    let below = boxes
        .iter()
        .filter(|b| b.corners.iter().all(|c| c[1] > b.horizon));
    let above = boxes
        .iter()
        .filter(|b| b.corners.iter().all(|c| c[1] < b.horizon));
    let (below, above) = (below.count(), above.count());
    let across = boxes.len() - below - above;
    assert!(
        below > 60 && above > 30 && across > 30,
        "{below} {across} {above}"
    );
    // Proportions: the front edge against the box's width.
    let tall = |b: &PerspectiveBox| {
        distance(b.corners[0], b.corners[2]) / distance(b.corners[0], b.corners[1])
    };
    let shapes: Vec<f64> = boxes.iter().map(tall).collect();
    assert!(shapes.iter().any(|s| *s < 0.7) && shapes.iter().any(|s| *s > 1.6));
    // A three-point box looks up or down.
    let thirds: Vec<bool> = (0..100)
        .map(|seed| {
            let b = PerspectiveBox::generate(BoxView::ThreePoint, seed);
            b.vanishing[Direction::Vertical.index()].is_some_and(|p| p[1] < 0.5)
        })
        .collect();
    assert!(thirds.iter().filter(|up| **up).count() > 25);
    assert!(thirds.iter().filter(|up| !**up).count() > 25);
}

#[test]
fn a_free_page_s_example_box_lies_on_its_horizon() {
    for horizon in Horizon::ALL {
        for seed in 0..300 {
            let label = format!("{horizon:?} {seed}");
            let b = PerspectiveBox::on_horizon(horizon, seed);
            assert!((b.horizon - horizon.height()).abs() < 1e-12, "{label}");
            keeps_its_rules(&b, false, &label);
            // Seen from below under a low horizon, from above under a high one (or across).
            let below = b.corners.iter().all(|c| c[1] > b.horizon);
            let above = b.corners.iter().all(|c| c[1] < b.horizon);
            match horizon {
                Horizon::Low => assert!(!below, "{label}"),
                Horizon::High => assert!(!above, "{label}"),
                Horizon::Middle => {}
            }
        }
    }
}

#[test]
fn the_exercises_carry_their_boxes_and_their_codes() {
    let exercise = Exercise::generated(Kind::Box(BoxView::ThreePoint), 0x5EED_0084);
    let Original::Box(b) = exercise.original() else {
        panic!("a box")
    };
    assert_eq!(
        b,
        PerspectiveBox::generate(BoxView::ThreePoint, 0x5EED_0084)
    );
    let free = Exercise::generated(Kind::FreeBoxes(Horizon::High), 7).original();
    assert!(matches!(free, Original::FreeBoxes(b) if (b.horizon - 0.28).abs() < 1e-12));
    for kind in [
        Kind::Box(BoxView::TwoPoint),
        Kind::Box(BoxView::ThreePoint),
        Kind::FreeBoxes(Horizon::Low),
        Kind::FreeBoxes(Horizon::Middle),
        Kind::FreeBoxes(Horizon::High),
    ] {
        let (code, variant) = kind.code();
        assert!((8..=9).contains(&code));
        assert_eq!(Kind::of_code(code, variant), Some(kind));
    }
    assert_eq!(Kind::of_code(8, 2), None);
    assert_eq!(EDGES.len(), 12);
}
