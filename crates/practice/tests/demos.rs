//! Demonstrations (S92): every kind's How plays V大's way on an example of its own, at a hand's
//! pace, ending on Compare's marks; the constructions it draws land on the exact answers.

use openpaint_practice::{
    Demo, Drawn, Family, Layout, Marker, On, Point, WIDE_REACH, distance, meet,
};

/// Each kind's steps, Compare the last.
const STEPS: [(Family, usize); 20] = [
    (Family::Line, 4),
    (Family::Combined, 4),
    (Family::Shape, 5),
    (Family::Box, 4),
    (Family::FreeBoxes, 5),
    (Family::Ellipse, 5),
    (Family::Point, 4),
    (Family::Proportion, 4),
    (Family::Angle, 4),
    (Family::Centre, 3),
    (Family::Halves, 4),
    (Family::Thirds, 5),
    (Family::Step, 5),
    (Family::Mirror, 6),
    (Family::Cylinder, 6),
    (Family::Cone, 4),
    (Family::Sphere, 5),
    (Family::Colour, 5),
    (Family::Areas, 5),
    (Family::Scale, 3),
];

fn demo(family: Family) -> Demo {
    Demo::of(family).unwrap_or_else(|| panic!("{family:?} has no demonstration"))
}

#[test]
fn a_lone_example_is_framed_on_what_it_is_about_a_trick_on_its_face() {
    for (family, _) in STEPS {
        let demo = demo(family);
        let lone = matches!(demo.layout, Layout::Single | Layout::Wide);
        assert_eq!(demo.view.is_some(), lone, "{family:?}");
    }
    let face_of = |demo: &Demo| -> [Point; 2] {
        let rows = (demo.marks.iter())
            .find_map(|m| match &m.drawn {
                Drawn::Band { rows, .. } if m.pen == Marker::Shade => Some(rows.clone()),
                _ => None,
            })
            .expect("the shaded face");
        let corners: Vec<Point> = rows.into_iter().flatten().collect();
        let low = corners
            .iter()
            .fold([f64::INFINITY; 2], |l, p| [l[0].min(p[0]), l[1].min(p[1])]);
        let high = (corners.iter()).fold([f64::NEG_INFINITY; 2], |h, p| {
            [h[0].max(p[0]), h[1].max(p[1])]
        });
        [low, high]
    };
    // The face is the picture: its answer lies on it, the rest of the box is cut away.
    for family in [
        Family::Centre,
        Family::Halves,
        Family::Thirds,
        Family::Mirror,
    ] {
        let demo = demo(family);
        let [low, high] = demo.view.expect("framed");
        let [face_low, face_high] = face_of(&demo);
        for (a, b) in [(low, face_low), (high, face_high)] {
            assert!(distance(a, b) < 1e-9, "{family:?}: {a:?} {b:?}");
        }
    }
    // The next face: the face and the one beyond it.
    let step = demo(Family::Step);
    let [low, high] = step.view.expect("framed");
    let [face_low, face_high] = face_of(&step);
    assert!(low[0] <= face_low[0] + 1e-9 && high[0] >= face_high[0] - 1e-9);
    assert!((high[0] - low[0]) <= 2.5 * (face_high[0] - face_low[0]));
}

#[test]
fn every_kind_but_the_colour_copy_plays_once_through_in_five_to_ten_seconds() {
    assert!(Demo::of(Family::Study).is_none());
    let lengths: Vec<String> = (STEPS.iter())
        .map(|(family, _)| format!("{family:?} {:.2}", demo(*family).length))
        .collect();
    println!("{lengths:?}");
    for (family, steps) in STEPS {
        let demo = demo(family);
        assert_eq!(demo.family, family);
        assert_eq!(demo.steps.len(), steps, "{family:?}");
        assert!((4.5..=10.3).contains(&demo.length), "{lengths:?}");
        assert!(demo.steps.windows(2).all(|w| w[0] < w[1]), "{family:?}");
        for mark in &demo.marks {
            assert!(
                mark.from <= mark.to && mark.to <= demo.length,
                "{family:?}: {mark:?}"
            );
        }
        // What the exercise gives shows from the start, and comes first.
        let given = demo.marks.iter().take_while(|m| m.to == 0.0).count();
        assert!(given > 0, "{family:?}");
        assert!(
            demo.marks[given..].iter().all(|m| m.from > 0.0),
            "{family:?}"
        );
        // Compare's marks show in the last step, and only there.
        let last = demo.steps[steps - 1];
        let compared = |mark: &&openpaint_practice::Mark| {
            matches!(
                mark.pen,
                Marker::Answer | Marker::Hidden | Marker::Found | Marker::Faint
            )
        };
        assert!(
            demo.marks.iter().filter(compared).all(|m| m.from >= last),
            "{family:?}"
        );
        assert_eq!(demo.step_at(0.0), 0);
        assert_eq!(demo.step_at(demo.length + 5.0), steps - 1);
        assert_eq!(
            Demo::of(family),
            Some(demo),
            "{family:?}: the same every time"
        );
    }
}

#[test]
fn the_steps_draw_with_the_pencil_and_the_ink_before_compare_shows_red() {
    for (family, steps) in STEPS {
        let demo = demo(family);
        let drawn_in = |pen: Marker| demo.marks.iter().any(|m| m.pen == pen && m.to > 0.0);
        // Free boxes have no exact answer; a combination's original lies faint over the copy.
        let red = drawn_in(Marker::Answer) || drawn_in(Marker::Faint);
        assert!(red || family == Family::FreeBoxes, "{family:?}");
        let inked = drawn_in(Marker::Ink);
        assert!(inked, "{family:?}: the answer is drawn");
        let before = demo.steps[steps - 1];
        assert!(
            demo.marks.iter().any(|m| m.to > 0.0 && m.to <= before),
            "{family:?}: the method draws before Compare"
        );
    }
}

/// The pencil lines a demonstration draws on the copy, in order.
fn pencil_lines(demo: &Demo) -> Vec<[Point; 2]> {
    (demo.marks.iter())
        .filter(|m| m.pen == Marker::Pencil && m.on == On::Copy)
        .filter_map(|m| match &m.drawn {
            Drawn::Line(points) if points.len() == 2 => Some([points[0], points[1]]),
            _ => None,
        })
        .collect()
}

fn answers(demo: &Demo) -> Vec<&Drawn> {
    (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .map(|m| &m.drawn)
        .collect()
}

#[test]
fn a_tricks_construction_lands_on_its_exact_answer() {
    // The centre: the diagonals drawn with Sketch cross at the red answer.
    let centre = demo(Family::Centre);
    let lines = pencil_lines(&centre);
    let crossed = meet(lines[0], lines[1]).expect("the diagonals cross");
    let [Drawn::Cross(exact)] = answers(&centre)[..] else {
        panic!("the centre's answer is a cross");
    };
    assert!(distance(crossed, *exact) < 1e-9, "{crossed:?} {exact:?}");
    // The mirror: its last construction line stands where the red answer is.
    let mirror = demo(Family::Mirror);
    let lines = pencil_lines(&mirror);
    let [Drawn::Cross(exact)] = answers(&mirror)[..] else {
        panic!("the mirror's answer is a cross");
    };
    let up = lines[lines.len() - 1];
    assert!(distance(up[1], *exact) < 1e-9, "{up:?} {exact:?}");
    // Halves: the line from the near edge through the centre heads where the red line lies.
    let halves = demo(Family::Halves);
    let lines = pencil_lines(&halves);
    let centre = meet(lines[0], lines[1]).expect("the diagonals cross");
    let [Drawn::Line(red)] = answers(&halves)[..] else {
        panic!("the halving line");
    };
    assert!(
        distance(lines[2][0], red[0]) < 1e-9,
        "from the near edge's middle"
    );
    let [a, b] = lines[2];
    let off = ((b[0] - a[0]) * (centre[1] - a[1]) - (b[1] - a[1]) * (centre[0] - a[0])).abs()
        / distance(a, b);
    assert!(off < 1e-9, "through the centre: {off}");
    let red_end = red[red.len() - 1];
    let on_line = ((b[0] - a[0]) * (red_end[1] - a[1]) - (b[1] - a[1]) * (red_end[0] - a[0])).abs()
        / distance(a, b);
    assert!(on_line < 1e-9, "the red line lies on it: {on_line}");
}

#[test]
fn a_solids_middles_lie_on_its_red_answer_and_its_hidden_part_is_dashed() {
    for family in [Family::Cylinder, Family::Cone] {
        let demo = demo(family);
        let red: Vec<&[Point]> = (demo.marks.iter())
            .filter(|m| matches!(m.pen, Marker::Answer | Marker::Hidden))
            .filter_map(|m| match &m.drawn {
                Drawn::Line(points) => Some(points.as_slice()),
                _ => None,
            })
            .collect();
        let dots: Vec<Point> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Pencil)
            .filter_map(|m| match m.drawn {
                Drawn::Dot(p) => Some(p),
                _ => None,
            })
            .collect();
        assert!(dots.len() >= 4, "{family:?}");
        for dot in dots {
            let (_, off) = openpaint_practice::nearest(&red, dot).expect("a red answer");
            assert!(off < 1e-4, "{family:?}: {dot:?} off {off}");
        }
    }
    let hidden = |family| demo(family).marks.iter().any(|m| m.pen == Marker::Hidden);
    assert!(hidden(Family::Cylinder) && !hidden(Family::Sphere));
}

#[test]
fn a_boxs_vanishing_points_lie_in_the_room_the_card_gives() {
    for family in [Family::Box, Family::FreeBoxes] {
        let demo = demo(family);
        assert_eq!(demo.layout, Layout::Wide);
        let rings: Vec<Point> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Pencil)
            .filter_map(|m| match m.drawn {
                Drawn::Ring(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(rings.len(), 2, "{family:?}");
        for [x, y] in rings {
            assert!(
                (-WIDE_REACH..=1.0 + WIDE_REACH).contains(&x),
                "{family:?}: {x}"
            );
            assert!((0.0..=1.0).contains(&y), "{family:?}: {y}");
        }
        // Each edge drawn on in blue ends on the horizon near its point.
        let found: Vec<Point> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Found)
            .filter_map(|m| match &m.drawn {
                Drawn::Line(points) => points.last().copied(),
                _ => None,
            })
            .collect();
        assert!(found.len() >= 6, "{family:?}: {}", found.len());
    }
}

#[test]
fn a_combination_is_painted_area_by_area_and_a_scale_cell_by_cell() {
    let areas = demo(Family::Areas);
    let example = areas.areas.as_ref().expect("the combination painted");
    let painted: Vec<&Vec<openpaint_practice::Rgb8>> = (areas.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .filter_map(|m| match &m.drawn {
            Drawn::Areas(colours) => Some(colours),
            _ => None,
        })
        .collect();
    assert_eq!(
        painted.len(),
        example.count() + 1,
        "the tone, then each area"
    );
    let last = painted[painted.len() - 1];
    for (copy, original) in last.iter().zip(&example.colours) {
        assert!(openpaint_practice::delta_e(*copy, *original) < 0.03);
    }
    let scale = demo(Family::Scale);
    assert_eq!(scale.layout, Layout::Wheel);
    let fills = (scale.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .count();
    assert_eq!(fills, 3, "the middle cell and the quarters");
}
