//! The reason beside the app's result (S138): for each kind whose check finds which way the copy
//! is off, when the result is not right, one line naming what is off, which way, and a little
//! (within the check's near band) or clearly (beyond), a value scale's cell which way alone
//! (S139); none for a right answer.

use openpaint_practice::{
    AreasCompared, AreasCopy, AreasRead, Arg, CardItem, ColourCompared, Compared, Flat, FlatCheck,
    Metric, NEAR_ANGLE, NEAR_FACE, NEAR_OFF, Outcome, Point, Rgb8, Say, ScaleTone, Seen,
    SpaceAnswer, SpaceCheck, SpaceTask, Trick, ValueScale, Verdict, from_oklch, oklch, value,
};

/// A reason's words: what, which way, how far, and its number if it has one.
fn words(say: &Say) -> Vec<String> {
    (say.args.iter())
        .map(|arg| match arg {
            Arg::Word(word) => (*word).to_owned(),
            Arg::Count(count) => count.to_string(),
        })
        .collect()
}

fn reason_of(said: Option<Say>) -> Vec<String> {
    let say = said.expect("a reason");
    assert_eq!(say.key, "reason");
    words(&say)
}

/// A dot at `p`: a mark.
fn dot(p: Point) -> Vec<Point> {
    vec![[p[0] - 0.001, p[1]], [p[0] + 0.001, p[1]]]
}

fn flat_reason(flat: &Flat, lines: &[Vec<Point>]) -> Option<Say> {
    let lines: Vec<&[Point]> = lines.iter().map(Vec::as_slice).collect();
    FlatCheck::of(flat, &lines).reason(flat)
}

#[test]
fn a_point_says_the_way_its_mark_lies_off_and_how_far() {
    let flat = Flat::Point { at: [0.5, 0.5] };
    assert!(flat_reason(&flat, &[dot([0.5, 0.5])]).is_none(), "right");
    // Within the near band (`NEAR_OFF`) and well past it.
    let (near, far) = (0.6 * NEAR_OFF, 2.0 * NEAR_OFF);
    let cases = [
        ([0.5 - near, 0.505], ["point", "left", "little"]),
        ([0.5 + far, 0.49], ["point", "right", "clearly"]),
        ([0.505, 0.5 - near], ["point", "up", "little"]),
        ([0.49, 0.5 + far], ["point", "down", "clearly"]),
    ];
    for (at, said) in cases {
        assert_eq!(reason_of(flat_reason(&flat, &[dot(at)])), said, "{at:?}");
    }
}

#[test]
fn a_proportion_says_whether_its_mark_lies_too_far_from_the_ringed_end_or_too_near() {
    let flat = Flat::Proportion {
        original: [[0.1, 0.2], [0.9, 0.2]],
        share: 0.25,
        copy: [[0.2, 0.6], [0.8, 0.6]],
    };
    let tick = |x: f64| vec![[x, 0.57], [x, 0.63]];
    assert!(flat_reason(&flat, &[tick(0.35)]).is_none(), "right");
    assert_eq!(
        reason_of(flat_reason(&flat, &[tick(0.37)])),
        ["mark", "far", "little"]
    );
    assert_eq!(
        reason_of(flat_reason(&flat, &[tick(0.30)])),
        ["mark", "near", "clearly"]
    );
}

/// A line of `heading` degrees (clockwise on the screen) through `through`.
fn line(heading: f64, through: Point) -> Vec<Point> {
    let (sin, cos) = heading.to_radians().sin_cos();
    vec![
        [through[0] - 0.3 * cos, through[1] - 0.3 * sin],
        [through[0] + 0.3 * cos, through[1] + 0.3 * sin],
    ]
}

#[test]
fn an_angle_says_the_way_its_line_turns_or_passes_the_dot() {
    let heading = (-0.2_f64).atan2(0.6).to_degrees();
    let flat = Flat::Angle {
        original: [[0.2, 0.6], [0.8, 0.4]],
        through: [0.5, 0.5],
    };
    assert!(flat_reason(&flat, &[line(heading, [0.5, 0.5])]).is_none());
    let turned = |by: f64| flat_reason(&flat, &[line(heading + by, [0.5, 0.5])]);
    assert_eq!(
        reason_of(turned(0.75 * NEAR_ANGLE)),
        ["line", "clockwise", "little"]
    );
    assert_eq!(
        reason_of(turned(-2.0 * NEAR_ANGLE)),
        ["line", "anticlockwise", "clearly"]
    );
    // At the angle, passing above the dot or well below it.
    let passed = |down: f64| flat_reason(&flat, &[line(heading, [0.5, 0.5 + down])]);
    assert_eq!(reason_of(passed(-0.03)), ["line", "up", "little"]);
    assert_eq!(reason_of(passed(0.08)), ["line", "down", "clearly"]);
}

fn space_lines(task: &SpaceTask) -> Vec<[Point; 2]> {
    match task.answer() {
        SpaceAnswer::Lines(lines) => lines,
        SpaceAnswer::Point(_) => panic!("lines"),
    }
}

/// `task` copied by its answer moved `by` on the picture: the mark, or the lines the trick reads
/// (a next face's upright alone).
fn moved(task: &SpaceTask, by: Point) -> Vec<Vec<Point>> {
    let shift = |p: Point| [p[0] + by[0], p[1] + by[1]];
    match (task.trick, task.answer()) {
        (_, SpaceAnswer::Point(p)) => vec![dot(shift(p))],
        (Trick::Step, _) => vec![space_lines(task)[1].map(shift).to_vec()],
        _ => (space_lines(task).into_iter())
            .map(|line| line.map(shift).to_vec())
            .collect(),
    }
}

#[test]
fn a_point_in_space_says_the_way_its_answer_was_missed_on_the_picture() {
    let cases = [
        (Trick::Centre, [-0.02, 0.0], "point", "left"),
        (Trick::Mirror, [0.0, 0.02], "point", "down"),
        (Trick::Halves, [0.0, -0.02], "line", "up"),
        (Trick::Thirds, [0.02, 0.0], "upright", "right"),
        (Trick::Step, [-0.01, 0.0], "upright", "left"),
    ];
    for (trick, by, what, way) in cases {
        let (mut little, mut clearly) = (0, 0);
        for seed in 0..40 {
            let task = SpaceTask::generate(trick, seed);
            let read = |by: Point| {
                let lines = moved(&task, by);
                let lines: Vec<&[Point]> = lines.iter().map(Vec::as_slice).collect();
                SpaceCheck::of(&task, &lines)
            };
            assert!(read([0.0, 0.0]).reason(&task).is_none(), "{trick:?} {seed}");
            for scale in [1.0, 2.0, 4.0, 8.0] {
                let check = read([by[0] * scale, by[1] * scale]);
                let verdict = check.verdict(&task);
                if !matches!(verdict, Verdict::App(Outcome::Near | Outcome::Redo)) {
                    continue;
                }
                let off = value(&check.metrics, Metric::SpaceOff).expect("measured");
                let degree = if off <= NEAR_FACE {
                    little += 1;
                    "little"
                } else {
                    clearly += 1;
                    "clearly"
                };
                let said = reason_of(check.reason(&task));
                assert_eq!(said, [what, way, degree], "{trick:?} {seed} x{scale}");
            }
        }
        // A next face recedes so far that a pen's width off its upright lies past the near band
        // on most boxes: its copies are rarely near.
        let both = little > 0 || trick == Trick::Step;
        assert!(both && clearly > 0, "{trick:?}: {little} {clearly}");
    }
}

/// The colour of `[l, c, h]` (Oklab, degrees) in 8 bits.
fn lch(l: f64, c: f64, h: f64) -> Rgb8 {
    from_oklch([l, c, h])
}

fn first_said(compared: &dyn Compared) -> Option<Vec<String>> {
    let seen = Seen {
        construction: false,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        fit: None,
        probe: None,
    };
    match compared.card(&seen).first() {
        Some(CardItem::Says(say)) if say.key == "reason" => Some(words(say)),
        _ => None,
    }
}

fn colour_said(original: Rgb8, copy: Rgb8) -> Option<Vec<String>> {
    first_said(&ColourCompared {
        original,
        first: copy,
        last: copy,
        metrics: Vec::new(),
    })
}

#[test]
fn a_colour_says_its_hue_then_its_lightness_then_its_purity() {
    let original = lch(0.6, 0.1, 60.0);
    assert_eq!(colour_said(original, original), None, "right");
    let [l, c, h] = oklch(original);
    let say = |copy: Rgb8| colour_said(original, copy).expect("a reason");
    assert_eq!(say(lch(l, c, h + 15.0)), ["colour", "yellow", "little"]);
    assert_eq!(say(lch(l, c, h - 40.0)), ["colour", "red", "clearly"]);
    assert_eq!(say(lch(l + 0.035, c, h)), ["colour", "light", "little"]);
    assert_eq!(say(lch(l - 0.1, c, h)), ["colour", "dark", "clearly"]);
    assert_eq!(say(lch(l, c - 0.035, h)), ["colour", "grey", "little"]);
    assert_eq!(say(lch(l, c + 0.07, h)), ["colour", "pure", "clearly"]);
    // The hue first though the lightness is further off; a red's hue turned back leans purple.
    assert_eq!(
        say(lch(l + 0.08, c, h + 15.0)),
        ["colour", "yellow", "little"]
    );
    let red = lch(0.6, 0.12, 35.0);
    let [l, c, h] = oklch(red);
    let back = colour_said(red, lch(l, c, h - 20.0)).expect("a reason");
    assert_eq!(back, ["colour", "purple", "little"]);
}

#[test]
fn a_combination_names_the_area_furthest_off() {
    let colours = vec![
        lch(0.5, 0.05, 200.0),
        lch(0.7, 0.08, 80.0),
        lch(0.4, 0.1, 300.0),
    ];
    let painted = |copy: Vec<Rgb8>| AreasCompared {
        colours: colours.clone(),
        read: Some(AreasRead {
            copy: AreasCopy {
                colours: copy.into_iter().map(Some).collect(),
                contrasts: Vec::new(),
            },
            strong: Vec::new(),
        }),
    };
    assert_eq!(first_said(&painted(colours.clone())), None, "right");
    assert_eq!(painted(colours.clone()).named_area(), None);
    let [l, c, h] = oklch(colours[1]);
    let mut copy = colours.clone();
    copy[1] = lch(l - 0.2, c, h);
    copy[0] = lch(oklch(colours[0])[0] + 0.03, 0.05, 200.0);
    let said = first_said(&painted(copy.clone())).expect("a reason");
    assert_eq!(said, ["colour", "dark", "clearly", "2"]);
    // Its card opens on that area's chip (S139), so the learner finds area 2 without counting.
    assert_eq!(painted(copy).named_area(), Some(1));
}

#[test]
fn a_scale_names_its_cell_furthest_off_and_which_way() {
    let scale = ValueScale::generate(ScaleTone::Grey, 5, 3);
    let targets = scale.targets();
    let step = (targets[4] - targets[0]) / 4.0;
    let checked = |offs: [f64; 5]| {
        let values = (targets.iter().zip(offs))
            .map(|(target, off)| Some(target + off * step))
            .collect();
        scale.check(values)
    };
    assert!(checked([0.0; 5]).reason().is_none(), "even");
    // Which way alone, no degree (S139): a scale is judged by how many cells are uneven, so a
    // cell far off may stand in an Almost and one a little off in an Again.
    let near = checked([0.0, 0.3, 0.0, 0.0, 0.0]);
    assert_eq!(near.verdict(), Verdict::App(Outcome::Near));
    assert_eq!(reason_of(near.reason()), ["cell", "light", "unsaid", "2"]);
    let far = checked([0.0, 0.0, -0.9, 0.0, 0.0]);
    assert_eq!(far.verdict(), Verdict::App(Outcome::Near));
    assert_eq!(reason_of(far.reason()), ["cell", "dark", "unsaid", "3"]);
    let worst = checked([0.0, 0.3, 0.0, -0.7, 0.0]);
    assert_eq!(worst.verdict(), Verdict::App(Outcome::Redo));
    assert_eq!(reason_of(worst.reason()), ["cell", "dark", "unsaid", "4"]);
    // A cell not filled yet: no result, so no reason.
    let mut values: Vec<Option<f64>> = targets.iter().map(|t| Some(*t)).collect();
    values[1] = Some(targets[1] - step);
    values[2] = None;
    assert!(scale.check(values).reason().is_none(), "incomplete");
}
