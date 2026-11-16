//! The demonstrations of the kinds built in one frame (S97): a box's Compare the exact box red and
//! its points ringed, free boxes' the horizon run on, nothing read from the ink and nothing blue
//! (S145), their vanishing points in the room the card gives; an ellipse's the exact one alone; a
//! value scale of nine numbered in the order of halving and painted a round at a time, its uneven
//! cell outlined with its even one.

use openpaint_practice::{
    BoxView, CELLS, Carve, Demo, DemoLayout, Drawn, Estimate, Guide, Horizon, Kind, Marker, On,
    Point, ScaleTone, Solid, Trick, WIDE_REACH,
};

fn demo(kind: Kind) -> Demo {
    kind.how(CELLS)
        .unwrap_or_else(|| panic!("{kind:?} has no demonstration"))
}

/// The box and free pages whose demonstrations show a box in perspective: two-point, and (S142)
/// three-point looking down and up.
const BOXES: [Kind; 4] = [
    Kind::Box(BoxView::ThreePoint),
    Kind::FreeBoxes(Horizon::Middle),
    Kind::FreeBoxes(Horizon::HighDown),
    Kind::FreeBoxes(Horizon::LowUp),
];

/// A demonstration's Compare marks by pen: red rings, red lines, dashed red run-ons, faint lines.
fn compared(demo: &Demo) -> [usize; 4] {
    let last = demo.steps[demo.steps.len() - 1];
    let at = demo.marks.iter().filter(|m| m.from >= last);
    let count = |pen: Marker, ring: bool| {
        (at.clone())
            .filter(|m| m.pen == pen && matches!(m.drawn, Drawn::Ring(_)) == ring)
            .count()
    };
    [
        count(Marker::Answer, true),
        count(Marker::Answer, false),
        count(Marker::Missed, false),
        count(Marker::Faint, false),
    ]
}

#[test]
fn a_box_s_compare_lays_its_answer_and_rings_its_points_and_free_boxes_run_the_horizon_on() {
    for kind in BOXES {
        let [rings, red, missed, faint] = compared(&demo(kind));
        assert_eq!(missed, 0, "{kind:?}: nothing run on from the ink");
        match kind {
            Kind::Box(_) => {
                // Its visible edges red (seven across the horizon, the hidden dashed), its two
                // points ringed.
                assert_eq!((rings, faint), (2, 0), "{kind:?}");
                assert!(red >= 7, "{kind:?}: {red}");
            }
            _ => assert_eq!(
                (rings, red, faint),
                (0, 0, 1),
                "{kind:?}: the horizon run on"
            ),
        }
    }
}

#[test]
fn an_ellipses_compare_lays_the_exact_one_alone() {
    assert_eq!(compared(&demo(Kind::Ellipse(Guide::Plane))), [0, 1, 0, 0]);
}

/// What a demonstration's Compare marks beside its red answer (S98): rings and dashed red lines.
fn off_marks(demo: &Demo) -> (usize, usize) {
    let [rings, _, missed, _] = compared(demo);
    (rings, missed)
}

/// S98: the tricks and the estimates compared as their exercises are, by their own checks on the
/// example's ink: what the hand drew off on purpose is marked, a ring or an edge run on, and
/// nothing else; the solids and the object lay their answer alone, the hand's ink drawn off on
/// purpose beside it (S145); a trick's and a solid's givens printed in their one frame.
#[test]
fn the_tricks_and_estimates_mark_what_the_hand_drew_off_and_the_solids_lay_their_answer() {
    let expected = [
        (Kind::Flat(Estimate::Point), (1, 0)),
        (Kind::Flat(Estimate::Proportion), (1, 0)),
        (Kind::Flat(Estimate::Angle), (0, 1)),
        (Kind::Space(Trick::Centre), (1, 0)),
        (Kind::Space(Trick::Halves), (1, 0)),
        (Kind::Space(Trick::Thirds), (1, 0)),
        (Kind::Space(Trick::Step), (1, 0)),
        (Kind::Space(Trick::Mirror), (1, 0)),
    ];
    for (kind, marks) in expected {
        assert_eq!(off_marks(&demo(kind)), marks, "{kind:?}");
    }
    for kind in [
        Kind::Solid(Solid::Cylinder),
        Kind::Solid(Solid::Cone),
        Kind::Solid(Solid::Sphere),
        Kind::Object(Carve::Cut),
    ] {
        let [rings, red, missed, _] = compared(&demo(kind));
        assert_eq!(
            (rings, missed),
            (0, 0),
            "{kind:?}: nothing read from the ink"
        );
        assert!(red >= 1, "{kind:?}: the answer laid");
    }
    for kind in [Kind::Space(Trick::Centre), Kind::Solid(Solid::Cylinder)] {
        let demo = demo(kind);
        assert_eq!(demo.layout, DemoLayout::Single, "{kind:?}: one frame");
        let printed = (demo.marks.iter()).filter(|m| m.to == 0.0 && m.pen == Marker::Given);
        assert_eq!(printed.count(), 0, "{kind:?}: its givens printed grey");
    }
}

/// The How of a scale follows the cell count of the scale in hand (the review of S97): of five,
/// its order 2 1 2 under the inner cells, the 3rd painted first, then the 2nd and 4th, Compare.
#[test]
fn a_scale_of_five_is_numbered_and_painted_as_a_five_cell_row_is() {
    assert_eq!(
        Kind::Scale(ScaleTone::Grey, 9).how(9).expect("a scale's"),
        demo(Kind::Scale(ScaleTone::Grey, 9)),
        "nine's is the kind's"
    );
    let scale = Kind::Scale(ScaleTone::Grey, 9).how(5).expect("a scale's");
    assert_eq!(scale.layout, DemoLayout::Row);
    let numbered: Vec<(f64, u8)> = (scale.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Numeral(at, n) if m.to == 0.0 => Some((at[0], n)),
            _ => None,
        })
        .collect();
    let mut along = numbered.clone();
    along.sort_by(|a, b| a.0.total_cmp(&b.0));
    let left_to_right: Vec<u8> = along.iter().map(|(_, n)| *n).collect();
    assert_eq!(left_to_right, [2, 1, 2]);
    assert_eq!(
        scale.steps.len(),
        3,
        "the middle, the halves' middles, Compare"
    );
    let fills: Vec<f64> = (scale.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .map(|m| m.from)
        .collect();
    let in_step = |at: usize| {
        let (start, end) = (scale.steps[at], scale.steps[at + 1]);
        fills
            .iter()
            .filter(|from| (start..end).contains(*from))
            .count()
    };
    assert_eq!([in_step(0), in_step(1)], [1, 2]);
    let outlined = (scale.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    assert_eq!(outlined, 1, "the uneven cell with its even one");
    assert!((4.5..=10.3).contains(&scale.length), "{}", scale.length);
}

#[test]
fn a_scale_of_nine_is_numbered_and_painted_a_round_at_a_time() {
    let scale = demo(Kind::Scale(ScaleTone::Grey, 9));
    assert_eq!(scale.layout, DemoLayout::Row);
    let numbered: Vec<u8> = (scale.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Numeral(_, n) if m.to == 0.0 => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(numbered, [1, 2, 2, 3, 3, 3, 3]);
    let fills: Vec<f64> = (scale.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .map(|m| m.from)
        .collect();
    assert_eq!(fills.len(), 7, "every cell between the ends");
    let in_step = |at: usize| {
        let (start, end) = (scale.steps[at], scale.steps[at + 1]);
        fills
            .iter()
            .filter(|from| (start..end).contains(*from))
            .count()
    };
    assert_eq!([in_step(0), in_step(1), in_step(2)], [1, 2, 4]);
    let outlined = (scale.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    assert_eq!(outlined, 1, "the uneven cell with its even one");
}

#[test]
fn a_boxs_vanishing_points_lie_in_the_room_the_card_gives() {
    for kind in BOXES {
        let demo = demo(kind);
        assert_eq!(demo.layout, DemoLayout::Wide);
        let rings: Vec<Point> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Pencil)
            .filter_map(|m| match m.drawn {
                Drawn::Ring(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(rings.len(), 2, "{kind:?}");
        for [x, y] in rings {
            assert!(
                (-WIDE_REACH..=1.0 + WIDE_REACH).contains(&x),
                "{kind:?}: {x}"
            );
            assert!((0.0..=1.0).contains(&y), "{kind:?}: {y}");
        }
        // A three-point page's example gives its guides beside its horizon (S142), meeting
        // below the horizon looking down, above looking up.
        let given: Vec<[Point; 2]> = (demo.marks.iter())
            .filter(|m| m.to == 0.0 && m.pen == Marker::Guide)
            .filter_map(|m| match &m.drawn {
                Drawn::Line(points) => Some([points[0], points[points.len() - 1]]),
                _ => None,
            })
            .collect();
        let horizon = given[0][0][1];
        match kind {
            Kind::FreeBoxes(Horizon::HighDown | Horizon::LowUp) => {
                assert_eq!(given.len(), 3, "{kind:?}");
                let meet = openpaint_practice::meet(given[1], given[2]).expect("they meet");
                let below = kind == Kind::FreeBoxes(Horizon::HighDown);
                assert_eq!(meet[1] > horizon + 1.0, below, "{kind:?}: {meet:?}");
                assert_eq!(meet[1] < horizon - 1.0, !below, "{kind:?}: {meet:?}");
            }
            _ => assert_eq!(given.len(), 1, "{kind:?}: the horizon alone"),
        }
    }
}
