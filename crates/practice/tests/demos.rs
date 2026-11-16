//! Demonstrations (S92): every kind's How plays V大's way on an example of its own, at a hand's
//! pace, ending on Compare's marks; the constructions it draws land on the exact answers.

use openpaint_practice::{
    BoxView, CELLS, Carve, Combination, CurvedRotation, Demo, DemoLayout, Drawn, Estimate, Guide,
    Horizon, Kind, Lightness, LineTier, Lit, LitColour, Marker, ObjectTask, On, Placement, Point,
    Purity, Rotation, ScaleTone, Solid, Texture, Throw, Tier, Tone, Trick, distance, meet,
};

const COLOUR: Tone = Tone {
    purity: Purity::Muted,
    lightness: Lightness::Middle,
};

/// Each kind's steps, Compare the last.
const STEPS: [(Kind, usize); 38] = [
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

/// The red answer's lines and crosses (its rings mark what is off, S98).
fn answers(demo: &Demo) -> Vec<&Drawn> {
    (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer && !matches!(m.drawn, Drawn::Ring(_)))
        .map(|m| &m.drawn)
        .collect()
}

#[test]
fn a_tricks_construction_lands_on_its_exact_answer() {
    // The centre: the diagonals drawn with Sketch cross at the red answer.
    let centre = demo(Kind::Space(Trick::Centre));
    let lines = pencil_lines(&centre);
    let crossed = meet(lines[0], lines[1]).expect("the diagonals cross");
    let [Drawn::Cross(exact)] = answers(&centre)[..] else {
        panic!("the centre's answer is a cross");
    };
    assert!(distance(crossed, *exact) < 1e-9, "{crossed:?} {exact:?}");
    // The mirror: its last construction line stands where the red answer is.
    let mirror = demo(Kind::Space(Trick::Mirror));
    let lines = pencil_lines(&mirror);
    let [Drawn::Cross(exact)] = answers(&mirror)[..] else {
        panic!("the mirror's answer is a cross");
    };
    let up = lines[lines.len() - 1];
    assert!(distance(up[1], *exact) < 1e-9, "{up:?} {exact:?}");
    // Halves: the line from the near edge through the centre heads where the red line lies.
    let halves = demo(Kind::Space(Trick::Halves));
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
    for kind in [Kind::Solid(Solid::Cylinder), Kind::Solid(Solid::Cone)] {
        let demo = demo(kind);
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
        assert!(dots.len() >= 4, "{kind:?}");
        for dot in dots {
            let (_, off) = openpaint_practice::nearest(&red, dot).expect("a red answer");
            assert!(off < 1e-4, "{kind:?}: {dot:?} off {off}");
        }
    }
    let hidden = |kind| demo(kind).marks.iter().any(|m| m.pen == Marker::Hidden);
    assert!(hidden(Kind::Solid(Solid::Cylinder)) && !hidden(Kind::Solid(Solid::Sphere)));
}

#[test]
fn a_combination_is_painted_area_by_area_with_its_pens_in_their_order() {
    let areas = demo(Kind::Crop(Tier::Simple));
    let example = &areas.areas.as_ref().expect("the combination painted").areas;
    let painted: Vec<(&Vec<openpaint_practice::Rgb8>, bool, usize)> = (areas.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .filter_map(|m| match &m.drawn {
            Drawn::Areas { colours, soft } => Some((colours, *soft, areas.step_at(m.from))),
            _ => None,
        })
        .collect();
    // Fill frame's tone, then the areas with Big (the largest, then its neighbours) and Small, their
    // edges hard as the hard tip lays them; then Soft softens the edges soft in the picture (S100).
    assert_eq!(areas.steps.len(), 6, "tone, Big, Big, Small, Soft, Compare");
    let (tone, _, first) = painted[0];
    assert!(first == 0 && tone.iter().all(|c| c == &tone[0]));
    let steps: Vec<usize> = painted.iter().map(|(_, _, step)| *step).collect();
    assert!(steps.windows(2).all(|w| w[0] <= w[1]) && steps.contains(&3));
    let (last, softened, step) = painted[painted.len() - 1];
    let hard = painted[..painted.len() - 1].iter().all(|p| !p.1);
    assert!(softened && step == 4 && hard, "Soft last, its own step");
    for (copy, original) in last.iter().zip(&example.colours) {
        assert!(openpaint_practice::delta_e(*copy, *original) < 0.03);
    }
    // Some edges soft in the picture and some hard: Soft looks only at the soft ones.
    let soft = (example.neighbours.iter())
        .filter(|(a, b)| example.soft[*a].max(example.soft[*b]) > 0.0)
        .count();
    assert!(soft > 0 && soft < example.neighbours.len(), "{soft}");
    let looked = (areas.marks.iter())
        .filter(|m| m.pen == Marker::Pencil && areas.step_at(m.from) == 4)
        .count();
    assert!(looked >= soft, "each soft edge pencilled");
}

#[test]
fn an_objects_example_shows_its_picture_beside_the_box_and_its_corners_on_the_construction() {
    let demo = demo(Kind::Object(Carve::Cut));
    assert_eq!(demo.layout, DemoLayout::Pair);
    let shaded = demo.shaded.as_ref().expect("the example object");
    let task = shaded.downcast::<ObjectTask>().expect("an object");
    assert_eq!(task.stages.len(), 2, "big to small: two changes");
    // Each step reads on the card: a dozen lines at most, no round part's square and middles.
    assert!(task.squares.is_empty());
    assert!(task.stages.iter().all(|stage| stage.lines.len() <= 12));
    // Its construction is drawn as construction (dark in its step, faint after: the UI's).
    let lines = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Construction && matches!(m.drawn, Drawn::Line(_)))
        .count();
    assert_eq!(lines, task.construction().len());
    let shaded = |m: &&openpaint_practice::Mark| m.drawn == Drawn::Shaded;
    let picture = demo.marks.iter().find(shaded).expect("the picture");
    assert_eq!(
        (picture.on, picture.to),
        (On::Original, 0.0),
        "shown from the start"
    );
    // Each corner found is dotted where its construction finds it, the object's corner projected.
    let dots: Vec<Point> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Dot(p) if m.pen == Marker::Construction => Some(p),
            _ => None,
        })
        .collect();
    for stage in &task.stages {
        for (found, at) in stage.found.iter().zip(&stage.at) {
            assert!(distance(*found, task.projected(*at)) < 1e-9);
            assert!(dots.iter().any(|d| distance(*d, *found) < 1e-12));
        }
    }
    // Compare's red is the visible edges (and its rings what is off, S98).
    let red = answers(&demo).len();
    let seen = (task.answer.outlines.iter().flatten())
        .filter(|s| !s.hidden)
        .count();
    assert_eq!(red, task.answer.sides.len() + seen);
}

#[test]
fn a_division_is_drawn_red_then_green_then_blue_and_a_whole_copy_over_it_dimmed() {
    use openpaint_practice::Level;
    let division = demo(Kind::Division(Tier::Simple));
    assert_eq!(division.layout, DemoLayout::Pair);
    assert!(division.masses.is_some() && division.dimmed.is_none());
    let drawn = |demo: &Demo, level: Level| -> Vec<f64> {
        let before = demo.steps[demo.steps.len() - 1];
        (demo.marks.iter())
            .filter(|m| m.pen == Marker::Level(level) && m.from < before)
            .map(|m| m.from)
            .collect()
    };
    let [main, second, third] = Level::ALL.map(|level| drawn(&division, level));
    assert!(!main.is_empty() && !second.is_empty() && !third.is_empty());
    let last = |times: &[f64]| times.iter().copied().fold(0.0, f64::max);
    let first = |times: &[f64]| times.iter().copied().fold(f64::MAX, f64::min);
    assert!(last(&main) < first(&second) && last(&second) < first(&third));
    // The turning point first: a dot before any main line.
    let dot = (division.marks.iter()).find(|m| matches!(m.drawn, Drawn::Dot(_)));
    assert!(dot.is_some_and(|d| d.from <= first(&main)));
    // Every line lies on the picture, which both frames are cropped to (S99): no edge drawn.
    let rect = (division.marks.iter())
        .find_map(|m| match m.drawn {
            Drawn::Masses(rect) if m.on == On::Original => Some(rect),
            _ => None,
        })
        .expect("the picture");
    assert_eq!(division.view, Some(rect));
    assert_eq!(rect, [[0.0, 0.0], [1.0, 0.75]], "a landscape four by three");
    assert!(division.marks.iter().all(|m| m.pen != Marker::Rule));
    let [[left, top], [right, bottom]] = rect;
    for mark in &division.marks {
        if let (Marker::Level(_), Drawn::Line(line)) = (mark.pen, &mark.drawn) {
            assert!(line.iter().all(|p| {
                (left - 0.01..=right + 0.01).contains(&p[0])
                    && (top - 0.01..=bottom + 0.01).contains(&p[1])
            }));
        }
    }
    // Compare: the picture faint over the copy, the lines in their colours over it.
    let compare = division.steps[3];
    let faint = (division.marks.iter()).find(|m| m.pen == Marker::Faint);
    assert!(faint.is_some_and(|m| m.from >= compare && matches!(m.drawn, Drawn::Masses(_))));
    // A whole copy: the division, then Draw's ink over it dimmed from that step, gone at Compare.
    let whole = demo(Kind::WholeCopy(Tier::Simple));
    let [draw, compare] = whole.dimmed.expect("dimmed");
    assert_eq!([draw, compare], [whole.steps[1], whole.steps[2]]);
    let ink = (whole.marks.iter()).filter(|m| m.pen == Marker::Ink);
    assert!(ink.clone().count() >= 5 && ink.clone().all(|m| m.from >= draw));
    let divided = (whole.marks.iter()).filter(|m| matches!(m.pen, Marker::Level(_)));
    assert!(divided.clone().count() >= 4 && divided.clone().all(|m| m.to <= draw));
}
