//! Demonstrations (S92): every kind's How plays V大's way on an example of its own, at a hand's
//! pace, ending on Compare's marks; the constructions it draws land on the exact answers
//! (`examples.rs`).

mod examples;

use openpaint_practice::{
    BoxView, CELLS, Carve, Combination, CurvedRotation, Demo, DemoLayout, Drawn, Emphasis,
    Estimate, Guide, Horizon, Kind, Lightness, LineTier, Lit, LitColour, Marker, Placement, Point,
    Purity, Rotation, ScaleTone, Solid, Texture, Throw, Tier, Tone, Trick, distance,
};

const COLOUR: Tone = Tone {
    purity: Purity::Muted,
    lightness: Lightness::Middle,
};

/// Each kind's steps, Compare the last.
const STEPS: [(Kind, usize); 39] = [
    (Kind::Line(LineTier::Arc), 4),
    (Kind::Combined(Combination::Joined), 4),
    (Kind::Shape(Tier::Entry), 5),
    (Kind::Box(BoxView::ThreePoint), 4),
    (Kind::FreeBoxes(Horizon::Middle), 5),
    (Kind::FreeBoxes(Horizon::HighDown), 5),
    (Kind::FreeBoxes(Horizon::LowUpBare), 5),
    (Kind::Ellipse(Guide::Plane), 5),
    (Kind::Flat(Estimate::Point), 4),
    (Kind::Flat(Estimate::Proportion), 4),
    (Kind::Flat(Estimate::Angle), 4),
    (Kind::Space(Trick::Centre), 3),
    (Kind::Space(Trick::Halves), 4),
    (Kind::Space(Trick::Thirds), 5),
    (Kind::Space(Trick::Step), 5),
    (Kind::Space(Trick::Mirror), 6),
    (Kind::Solid(Solid::Cylinder), 6),
    (Kind::Solid(Solid::Cone), 4),
    (Kind::Solid(Solid::Sphere), 5),
    (Kind::Object(Carve::Cut), 5),
    (Kind::Rotation(Rotation::ALL[1]), 5),
    (Kind::Division(Tier::Simple), 4),
    (Kind::WholeCopy(Tier::Simple), 3),
    (Kind::ValueMasses(Tier::Simple), 4),
    (Kind::Analysis(Tier::Simple), 3),
    (Kind::Colour(COLOUR), 5),
    (Kind::Crop(Tier::Simple), 6),
    (Kind::Scale(ScaleTone::Grey, 9), 4),
    (Kind::Placement(Placement::ALL[0]), 5),
    (Kind::Light(Lit::ALL[0]), 4),
    (Kind::Light(Lit::ALL[4]), 5),
    (Kind::Light(Lit::ALL[14]), 6),
    (Kind::Shadows(Throw::Side), 4),
    (Kind::CurvedRotation(CurvedRotation::ALL[0]), 6),
    (Kind::LitColour(LitColour::ALL[7]), 5),
    (Kind::LitColour(LitColour::ALL[10]), 6),
    (Kind::Texture(Texture::ALL[0]), 4),
    (Kind::Texture(Texture::ALL[4]), 5),
    (Kind::Thumbnail(Emphasis::Own), 4),
];

fn demo(kind: Kind) -> Demo {
    kind.how(CELLS)
        .unwrap_or_else(|| panic!("{kind:?} has no demonstration"))
}

#[test]
fn a_lone_example_is_framed_on_what_it_is_about_a_trick_on_its_face() {
    for (kind, _) in STEPS {
        let demo = demo(kind);
        // An object's frames side by side are cropped to its box (S94), a scale's frame to its
        // rows (S97), a composition's to its picture (S99).
        let cropped = matches!(
            demo.layout,
            DemoLayout::Single | DemoLayout::Wide | DemoLayout::Row
        );
        let lone = [
            "objects",
            "structure-rotation",
            "curved-rotation",
            "division",
            "whole-copy",
            "value-masses",
            "texture",
            "texture-sparse",
        ];
        let lone = cropped || lone.contains(&kind.facts().key);
        assert_eq!(demo.view.is_some(), lone, "{kind:?}");
    }
    let face_of = |demo: &Demo| -> [Point; 2] {
        let rows = (demo.marks.iter())
            .find_map(|m| match &m.drawn {
                // Printed grey in its one frame (S98).
                Drawn::Band { rows, .. } if m.pen == Marker::Printed => Some(rows.clone()),
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
    for kind in [
        Kind::Space(Trick::Centre),
        Kind::Space(Trick::Halves),
        Kind::Space(Trick::Thirds),
        Kind::Space(Trick::Mirror),
    ] {
        let demo = demo(kind);
        let [low, high] = demo.view.expect("framed");
        let [face_low, face_high] = face_of(&demo);
        for (a, b) in [(low, face_low), (high, face_high)] {
            assert!(distance(a, b) < 1e-9, "{kind:?}: {a:?} {b:?}");
        }
    }
    // The next face: the face and the one beyond it.
    let step = demo(Kind::Space(Trick::Step));
    let [low, high] = step.view.expect("framed");
    let [face_low, face_high] = face_of(&step);
    assert!(low[0] <= face_low[0] + 1e-9 && high[0] >= face_high[0] - 1e-9);
    assert!((high[0] - low[0]) <= 2.5 * (face_high[0] - face_low[0]));
}

#[test]
fn every_kind_but_the_colour_copy_plays_once_through_in_five_to_ten_seconds() {
    assert!(Kind::Study.how(CELLS).is_none());
    let lengths: Vec<String> = (STEPS.iter())
        .map(|(kind, _)| format!("{kind:?} {:.2}", demo(*kind).length))
        .collect();
    println!("{lengths:?}");
    for (kind, steps) in STEPS {
        let demo = demo(kind);
        assert_eq!(demo.steps.len(), steps, "{kind:?}");
        assert!((4.5..=10.3).contains(&demo.length), "{lengths:?}");
        assert!(demo.steps.windows(2).all(|w| w[0] < w[1]), "{kind:?}");
        for mark in &demo.marks {
            assert!(
                mark.from <= mark.to && mark.to <= demo.length,
                "{kind:?}: {mark:?}"
            );
        }
        // What the exercise gives shows from the start, and comes first.
        let given = demo.marks.iter().take_while(|m| m.to == 0.0).count();
        assert!(given > 0, "{kind:?}");
        assert!(demo.marks[given..].iter().all(|m| m.from > 0.0), "{kind:?}");
        // Compare's marks show in the last step, and only there.
        let last = demo.steps[steps - 1];
        let compared = |mark: &&openpaint_practice::Mark| {
            matches!(
                mark.pen,
                Marker::Answer | Marker::Hidden | Marker::Missed | Marker::Faint
            )
        };
        assert!(
            demo.marks.iter().filter(compared).all(|m| m.from >= last),
            "{kind:?}"
        );
        assert_eq!(demo.step_at(0.0), 0);
        assert_eq!(demo.step_at(demo.length + 5.0), steps - 1);
        assert_eq!(kind.how(CELLS), Some(demo), "{kind:?}: the same every time");
    }
}

#[test]
fn the_steps_draw_with_the_pencil_and_the_ink_before_compare_shows_red() {
    for (kind, steps) in STEPS {
        let demo = demo(kind);
        let drawn_in = |pen: Marker| demo.marks.iter().any(|m| m.pen == pen && m.to > 0.0);
        // Free boxes have no exact answer; a combination's original lies faint over the copy.
        let red = drawn_in(Marker::Answer) || drawn_in(Marker::Faint);
        assert!(red || matches!(kind, Kind::FreeBoxes(_)), "{kind:?}");
        // A division's lines are its levels' pens' (S95), an analysis's marks too (S112).
        let levels = (demo.marks.iter()).any(|m| matches!(m.pen, Marker::Level(_)) && m.to > 0.0);
        let marked = matches!(kind, Kind::Division(_) | Kind::Analysis(_));
        let inked = drawn_in(Marker::Ink) || (marked && levels);
        assert!(inked, "{kind:?}: the answer is drawn");
        let before = demo.steps[steps - 1];
        assert!(
            demo.marks.iter().any(|m| m.to > 0.0 && m.to <= before),
            "{kind:?}: the method draws before Compare"
        );
    }
}
