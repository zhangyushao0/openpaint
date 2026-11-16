//! Contours on organic forms (S156k): the lit sausage beside its outline, its spine and ticks as
//! the level gives them; Compare laying each asked section's exact ellipse for the eye without
//! reading the copy; its lessons; its How. The answer's exactness is the folder's own tests'.

use openpaint_practice::{
    Asked, Bend, CELLS, CardItem, Choices, Compare, ContourTask, Copied, DemoLayout, DrawnCopy,
    Exercise, Figure, In, Kind, Learner, Level, LinePen, Marker, OBJECT_SIDE, Pictures, Point,
    Rgb8, Seen, SheetMark, Skill, SkillLevel, Standing, UNITS, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a contour exercise's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a contour exercise's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a contour exercise's copy is never read")
    }
}

/// An empty copy.
struct Empty;

impl Copied for Empty {
    fn holds(&self) -> bool {
        false
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy::default()
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        Vec::new()
    }
}

fn seen(construction: bool) -> Seen<'static> {
    Seen {
        construction,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    }
}

/// The lines among `marks` in `pen` in the copy's frame: whole and dashed.
fn lines(marks: &[SheetMark], pen: LinePen) -> [usize; 2] {
    let of = |dashed: bool| {
        (marks.iter())
            .filter(|mark| mark.on == In::Copy)
            .filter(|mark| {
                matches!(&mark.drawn, Figure::Line { pen: p, dashed: d, .. } if *p == pen && *d == dashed)
            })
            .count()
    };
    [of(false), of(true)]
}

#[test]
fn compare_lays_the_asked_sections_for_the_eye_and_never_reads_the_copy() {
    for bend in Bend::ALL {
        let exercise = Exercise::generated(Kind::Contour(bend), 0x5EED_0156);
        let original = exercise.original();
        let task = original.downcast::<ContourTask>().expect("contours");
        let asked = |copy: &'static dyn Copied| Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        assert!(original.compare(&asked(&Empty)).is_none(), "nothing drawn");
        let Some(Compare::Checked(shown)) = original.compare(&asked(&Unread)) else {
            panic!("{bend:?}: checked, a construction to lay");
        };
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        // Each asked section's ellipse: what faces the eye red, its hidden half dashed red.
        let stretches = task.contours.iter().flat_map(|c| &c.stretches);
        let seen_count = stretches.clone().filter(|s| !s.hidden).count();
        let hidden_count = stretches.filter(|s| s.hidden).count();
        let marks = shown.marks(&seen(false));
        assert_eq!(lines(&marks, LinePen::Answer), [seen_count, hidden_count]);
        assert!(hidden_count > 0, "{bend:?}: a far half at least");
        assert!(!marks.iter().any(|m| matches!(m.drawn, Figure::Ring(_))));
        // 看画法: the spine, each section's square and its middles.
        let constructed = shown.marks(&seen(true));
        let squares = task.contours.len();
        assert_eq!(lines(&constructed, LinePen::Pencil), [1, squares]);
        let middles = (constructed.iter()).filter(|m| matches!(m.drawn, Figure::Dot { .. }));
        assert_eq!(middles.count(), 4 * squares);
    }
}

#[test]
fn the_givens_are_the_picture_the_outline_and_the_ticks_and_the_spine_and_example_by_level() {
    for bend in Bend::ALL {
        let original = Exercise::generated(Kind::Contour(bend), 0x5EED_0156).original();
        let task = original.downcast::<ContourTask>().expect("contours");
        let givens = original.givens();
        let picture = givens.iter().filter(|mark| mark.on == In::Original);
        assert!(matches!(
            picture.map(|mark| &mark.drawn).collect::<Vec<_>>()[..],
            [Figure::Picture { .. }]
        ));
        let ticks: usize = task.contours.iter().map(|c| c.ticks.len()).sum();
        let example = task.example.iter().flat_map(|c| &c.stretches);
        let [example_seen, example_hidden] =
            [false, true].map(|hidden| example.clone().filter(|s| s.hidden == hidden).count());
        let [printed, dashed] = lines(&givens, LinePen::Printed);
        assert_eq!(
            printed,
            task.outline.len() + ticks + example_seen,
            "{bend:?}"
        );
        assert_eq!(dashed, 0);
        let spine = usize::from(bend != Bend::Crossing);
        assert_eq!(
            lines(&givens, LinePen::PrintedHalf),
            [0, spine + example_hidden]
        );
        assert_eq!(task.example.is_some(), bend == Bend::Gentle);
        let painted = original.painted(None).expect("the lit sausage");
        assert_eq!(painted.0, [OBJECT_SIDE; 2]);
    }
}

#[test]
fn each_level_lays_one_sausage_of_its_bend_in_unit_eight_before_contrast_thumbnails() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("contours").expect("the skill");
    assert_eq!(skill.window(), 3);
    let thumbnails = Skill::of_key("contrast-thumbnail").expect("thumbnails");
    let unit = UNITS[7].skills;
    let at = |skill: Skill| unit.iter().position(|held| *held == skill);
    assert!(at(skill).expect("in unit 8") < at(thumbnails).expect("in unit 8"));
    for (level, bend) in SkillLevel::ALL.into_iter().zip(Bend::ALL) {
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        let kinds: Vec<Kind> = planned.exercises.iter().map(|(e, _)| e.kind).collect();
        assert_eq!(kinds, [Kind::Contour(bend)], "{level:?}");
    }
}

#[test]
fn a_sausage_is_the_same_per_seed_and_stored_by_its_variant() {
    for (place, bend) in Bend::ALL.into_iter().enumerate() {
        let kind = Kind::Contour(bend);
        assert_eq!(kind.code(), (34, place as u8));
        assert_eq!(Kind::of_code(34, place as u8), Some(kind));
        let task = ContourTask::generate(bend, 9);
        let exercise = Exercise::generated(kind, 9);
        assert!(exercise.original().downcast::<ContourTask>() == Some(&task));
    }
    let facts = Kind::Contour(Bend::Gentle).facts();
    assert!(facts.compared && !facts.aligned && !facts.constructed);
}

#[test]
fn the_how_plays_the_spine_the_minor_axes_and_the_ellipses_beside_its_picture() {
    let demo = Kind::Contour(Bend::Gentle).how(CELLS).expect("a How");
    assert_eq!(demo.layout, DemoLayout::Pair);
    assert_eq!(demo.steps.len(), 4);
    let shown = demo.shaded.as_ref().expect("the example's picture");
    let task = shown.downcast::<ContourTask>().expect("a sausage");
    let drawn = |pen: Marker, on: In| {
        let on = match on {
            In::Original => openpaint_practice::On::Original,
            In::Copy => openpaint_practice::On::Copy,
        };
        (demo.marks.iter())
            .filter(|m| m.pen == pen && m.on == on && m.to > 0.0)
            .count()
    };
    assert_eq!(drawn(Marker::Pencil, In::Original), 1, "the spine");
    assert_eq!(
        drawn(Marker::Pencil, In::Copy),
        task.contours.len(),
        "the minor axes"
    );
    assert_eq!(
        drawn(Marker::Ink, In::Copy),
        task.contours.len(),
        "the ellipses"
    );
    let red = demo
        .marks
        .iter()
        .filter(|m| m.pen == Marker::Answer)
        .count();
    let stretches = task.contours.iter().flat_map(|c| &c.stretches);
    assert_eq!(red, stretches.filter(|s| !s.hidden).count());
}
