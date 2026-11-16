//! Boxes in perspective (S84): the exact boxes. Each is the same for the same seed; its edges'
//! lines run through their vanishing points, the horizontal ones on the horizon, a two-point
//! box's verticals vertical, a three-point box's meeting above or below; every box keeps V大's
//! rules (a front angle over 90 degrees, vanishing points neither too near nor too far, in the
//! frame); free-box pages put their example on their horizon, a three-point page's third point
//! far below or above (S142), a guided page's guides leading to it; a long box is long and a box
//! drawn through hides its far corner clear inside it (S142).

use openpaint_practice::{
    BoxView, Direction, EDGES, Exercise, FAR, FreePage, Horizon, Kind, Look, PerspectiveBox, Point,
    distance, guides,
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
                BoxView::TwoPoint | BoxView::Long => assert_eq!(third, None, "{label}"),
                BoxView::ThreePoint | BoxView::Through => {
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
            let third = b.vanishing[Direction::Vertical.index()];
            match horizon.look() {
                Look::Level => assert_eq!(third, None, "{label}: two-point"),
                // Looking down the page's third point lies far below (the boxes under the
                // horizon); looking up far above.
                look => {
                    let p = third.expect("the page's third point");
                    let far = distance(p, [0.5, 0.5]);
                    assert!((1.3..=4.0).contains(&far), "{label}: third at {far}");
                    let down = look == Look::Down;
                    assert_eq!((p[1] > 1.0, p[1] < 0.0), (down, !down), "{label}: {p:?}");
                    assert!(if down { below } else { above }, "{label}");
                }
            }
            match horizon {
                Horizon::Low | Horizon::LowUp | Horizon::LowUpBare => assert!(!below, "{label}"),
                Horizon::High | Horizon::HighDown | Horizon::HighDownBare => {
                    assert!(!above, "{label}");
                }
                Horizon::Middle => {}
            }
        }
    }
}

/// S142: a guided three-point page's faint guides lead to its third point, drawing together
/// towards it inside the frame; a bare page and a level one have none.
#[test]
fn a_guided_page_s_guides_lead_to_its_third_point_and_a_bare_page_has_none() {
    for horizon in Horizon::ALL {
        for seed in 0..100 {
            let label = format!("{horizon:?} {seed}");
            let page = FreePage(PerspectiveBox::on_horizon(horizon, seed), horizon);
            let given = Exercise::generated(Kind::FreeBoxes(horizon), seed).original();
            let guided = horizon.look() != Look::Level && horizon.guided();
            assert_eq!(given.givens().len(), if guided { 3 } else { 1 }, "{label}");
            let Some(third) = page.given_third() else {
                assert!(!guided, "{label}");
                continue;
            };
            let [left, right] = guides(page.0.horizon, third);
            for guide in [left, right] {
                assert!(
                    off_line(third, guide[0], guide[1]) < 1e-9,
                    "{label}: {guide:?}"
                );
                let inside = |p: &Point| p.iter().all(|v| (0.06..=0.94).contains(v));
                assert!(guide.iter().all(inside), "{label}: {guide:?}");
                assert!(distance(guide[0], guide[1]) > 0.4, "{label}: {guide:?}");
            }
            // They draw together towards the point: plainly, at the sheet's view.
            let apart = |k: usize| distance(left[k], right[k]);
            assert!(apart(0) - apart(1) > 0.04, "{label}");
        }
    }
}

#[test]
fn a_long_box_is_two_point_and_one_side_two_and_a_half_to_four_times_its_shortest() {
    let mut longest_up = [0; 3];
    for seed in 0..500 {
        let label = format!("Long {seed}");
        let sides = PerspectiveBox::proportions(BoxView::Long, seed);
        let most = sides.iter().copied().fold(0.0, f64::max);
        let least = sides.iter().copied().fold(f64::INFINITY, f64::min);
        assert!((2.5..=4.0).contains(&(most / least)), "{label}: {sides:?}");
        let at = (0..3).find(|k| sides[*k] == most).expect("its longest");
        longest_up[at] += 1;
        // Not a sliver on the sheet: its image's longest edge stays well beside its shortest.
        let b = PerspectiveBox::generate(BoxView::Long, seed);
        let lengths: Vec<f64> = b.edges().iter().map(|(a, z, _)| distance(*a, *z)).collect();
        let shortest = lengths.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(shortest >= 0.04, "{label}: {shortest}");
    }
    // A plank lying or a slab, its longest side wide or deep, and now and then a tower (a tall
    // box's narrow faces fail V大's rules more often).
    assert!(longest_up.iter().all(|n| *n > 30), "{longest_up:?}");
    // Today's boxes keep within 2.5 to 1.
    for seed in 0..100 {
        let sides = PerspectiveBox::proportions(BoxView::ThreePoint, seed);
        let most = sides.iter().copied().fold(0.0, f64::max);
        let least = sides.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(most / least <= 2.5, "{sides:?}");
    }
}

#[test]
fn a_box_drawn_through_hides_three_edges_meeting_at_its_far_corner_clear_inside_it() {
    for seed in 0..500 {
        let label = format!("Through {seed}");
        let b = PerspectiveBox::generate(BoxView::Through, seed);
        let hidden: Vec<(usize, usize)> = (EDGES.iter().zip(b.hidden()))
            .filter(|(_, hidden)| *hidden)
            .map(|((a, z, _), _)| (*a, *z))
            .collect();
        assert_eq!(hidden.len(), 3, "{label}");
        let far = (0..8)
            .find(|c| hidden.iter().all(|(a, z)| a == c || z == c))
            .expect("one far corner");
        // Clear of every visible edge, as each hidden edge's middle: drawn, it reads as its own.
        let seen: Vec<(Point, Point)> = (b.edges().into_iter().zip(b.hidden()))
            .filter(|(_, hidden)| !hidden)
            .map(|((a, z, _), _)| (a, z))
            .collect();
        let middles = hidden.iter().map(|(a, z)| {
            let [p, q] = [b.corners[*a], b.corners[*z]];
            [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
        });
        for p in middles.chain([b.corners[far]]) {
            for (a, z) in &seen {
                assert!(to_segment(p, *a, *z) >= 0.04, "{label}: {p:?}");
            }
        }
        let sides = PerspectiveBox::proportions(BoxView::Through, seed);
        let most = sides.iter().copied().fold(0.0, f64::max);
        let least = sides.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(most / least <= 3.6, "{label}: {sides:?}");
    }
}

/// How far `p` lies from the segment `a` to `b`.
fn to_segment(p: Point, a: Point, b: Point) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
    let t = t.clamp(0.0, 1.0);
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}

#[test]
fn the_exercises_carry_their_boxes_and_their_codes() {
    let exercise = Exercise::generated(Kind::Box(BoxView::ThreePoint), 0x5EED_0084);
    let Some(b) = exercise.original().downcast::<PerspectiveBox>().copied() else {
        panic!("a box")
    };
    assert_eq!(
        b,
        PerspectiveBox::generate(BoxView::ThreePoint, 0x5EED_0084)
    );
    let free = Exercise::generated(Kind::FreeBoxes(Horizon::High), 7).original();
    let page = free.downcast::<FreePage>().expect("a page");
    assert!((page.0.horizon - 0.28).abs() < 1e-12);
    // A box drawn through carries its box; a three-point page its look.
    let through = Exercise::generated(Kind::Box(BoxView::Through), 7).original();
    let drawn = through
        .downcast::<PerspectiveBox>()
        .expect("a box drawn through");
    assert_eq!(*drawn, PerspectiveBox::generate(BoxView::Through, 7));
    let down = Exercise::generated(Kind::FreeBoxes(Horizon::HighDown), 7).original();
    let page = down.downcast::<FreePage>().expect("a page");
    assert_eq!(page.1, Horizon::HighDown);
    // Codes: the views and horizons appended keep every earlier one's.
    let codes = [
        (Kind::Box(BoxView::TwoPoint), (8, 0)),
        (Kind::Box(BoxView::ThreePoint), (8, 1)),
        (Kind::Box(BoxView::Long), (8, 2)),
        (Kind::Box(BoxView::Through), (8, 3)),
        (Kind::FreeBoxes(Horizon::Low), (9, 0)),
        (Kind::FreeBoxes(Horizon::Middle), (9, 1)),
        (Kind::FreeBoxes(Horizon::High), (9, 2)),
        (Kind::FreeBoxes(Horizon::HighDown), (9, 3)),
        (Kind::FreeBoxes(Horizon::LowUp), (9, 4)),
        (Kind::FreeBoxes(Horizon::HighDownBare), (9, 5)),
        (Kind::FreeBoxes(Horizon::LowUpBare), (9, 6)),
    ];
    for (kind, code) in codes {
        assert_eq!(kind.code(), code);
        assert_eq!(Kind::of_code(code.0, code.1), Some(kind));
    }
    assert_eq!((Kind::of_code(8, 4), Kind::of_code(9, 7)), (None, None));
    assert_eq!(EDGES.len(), 12);
}
