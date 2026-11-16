//! A curved rotation's givens and Compare (S154f): the picture of view A beside view B's box in the
//! copy's frame as the level gives it (the box, the axis and the joints ticked; the box and the
//! axis; the axis, the near upright and the small angle box); Compare lays B's answer over the
//! copy, seen lines red and hidden ones dashed, a construction to lay with it, and leaves the
//! result to the eye (D123): the copy's strokes are never read.

use openpaint_practice::{
    Asked, CardItem, Choices, Compare, Copied, CurvedRotation, CurvedTask, DrawnCopy, Exercise,
    Figure, Handed, In, Kind, Learner, Level, LinePen, OBJECT_SIDE, Pictures, Point, Rgb8, Seen,
    SheetMark, Skill, SkillLevel, Standing, UNITS, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a curved rotation's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a curved rotation's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a curved rotation's copy is never read")
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
fn compare_lays_the_asked_answer_for_the_eye_and_never_reads_the_copy() {
    for curved in CurvedRotation::ALL {
        let exercise = Exercise::generated(Kind::CurvedRotation(curved), 0x5EED_0154);
        let original = exercise.original();
        let task = original
            .downcast::<CurvedTask>()
            .expect("a curved rotation");
        let asked = |copy: &'static dyn Copied| Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        assert!(original.compare(&asked(&Empty)).is_none(), "nothing drawn");
        let Some(Compare::Checked(shown)) = original.compare(&asked(&Unread)) else {
            panic!("{curved:?}: checked, a construction to lay");
        };
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        // The answer: B's seen lines red, what its parts hide dashed red; nothing ringed.
        let answer = &task.asked.answer;
        let stretches = answer.curves.iter().flat_map(|c| &c.stretches);
        let seen_curves = stretches.clone().filter(|s| !s.hidden).count();
        let hidden_curves = stretches.filter(|s| s.hidden).count();
        let marks = shown.marks(&seen(false));
        let [whole, dashed] = lines(&marks, LinePen::Answer);
        assert_eq!(whole, answer.sides.len() + seen_curves, "{curved:?}");
        assert_eq!(dashed, answer.hidden.len() + hidden_curves, "{curved:?}");
        assert!(dashed > 0, "{curved:?}: a far rim at least");
        assert!(
            !marks
                .iter()
                .any(|mark| matches!(mark.drawn, Figure::Ring(_)))
        );
        assert_eq!(lines(&marks, LinePen::Pencil), [0, 0]);
        // 看画法: the whole box, the sections found and their ellipses, what stands on the side.
        let constructed = shown.marks(&seen(true));
        let [pencil, dashed] = lines(&constructed, LinePen::Pencil);
        let hidden_box = task.asked.seen.hidden().iter().filter(|h| **h).count();
        let built: usize = task.asked.stages.iter().map(|s| s.lines.len()).sum();
        let (edges, guides) = &task.asked.attached;
        let ellipses = task.asked.sections.len() + guides.len();
        assert_eq!(
            pencil,
            12 - hidden_box + edges.len() + ellipses,
            "{curved:?}"
        );
        assert_eq!(dashed, hidden_box + built, "{curved:?}");
        let middles = (constructed.iter())
            .filter(|mark| matches!(mark.drawn, Figure::Dot { .. }))
            .count();
        assert_eq!(middles, 4 * task.asked.sections.len());
        assert!(constructed.iter().all(|mark| mark.on == In::Copy));
    }
}

#[test]
fn the_givens_are_the_picture_and_b_s_box_its_axis_and_ticks_as_the_level_gives_them() {
    for curved in CurvedRotation::ALL {
        let original = Exercise::generated(Kind::CurvedRotation(curved), 0x5EED_0154).original();
        let task = original
            .downcast::<CurvedTask>()
            .expect("a curved rotation");
        let givens = original.givens();
        let picture = givens.iter().filter(|mark| mark.on == In::Original);
        assert!(matches!(
            picture.map(|mark| &mark.drawn).collect::<Vec<_>>()[..],
            [Figure::Picture { .. }]
        ));
        let horizon = givens.iter().find_map(|mark| match mark.drawn {
            Figure::Horizon(height) => Some(height),
            _ => None,
        });
        assert_eq!(
            horizon,
            Some(task.asked.seen.horizon),
            "the asked view's horizon"
        );
        let [printed, axis] = lines(&givens, LinePen::Printed);
        let [angle, _] = lines(&givens, LinePen::PrintedHalf);
        let ticks = task.asked.joints.len();
        let expected = match curved.handed {
            Handed::Ticked => 3 + ticks,
            Handed::Axis => 3,
            Handed::Bare => 1,
        };
        assert_eq!((printed, axis), (expected, 1), "{curved:?}");
        assert_eq!(angle > 0, curved.handed == Handed::Bare, "{curved:?}");
        let painted = original.painted(None).expect("view A's picture");
        assert_eq!(painted.0, [OBJECT_SIDE; 2]);
    }
}

#[test]
fn each_level_lays_one_object_of_its_variants_in_turn_first_in_unit_seven() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("curved-rotation").expect("the skill");
    assert_eq!(skill.window(), 3);
    assert_eq!(
        UNITS[6].skills.first(),
        Some(&skill),
        "unit 7's first skill"
    );
    let laid = |level, seed| -> Vec<Kind> {
        let planned = schedule::lesson(skill, level, seed, &standing).expect("a lesson");
        planned.exercises.iter().map(|(e, _)| e.kind).collect()
    };
    let one = |k: usize| vec![Kind::CurvedRotation(CurvedRotation::ALL[k])];
    assert_eq!(laid(SkillLevel::First, 7), one(0));
    assert_eq!(laid(SkillLevel::Second, 7), one(2));
    assert_eq!(laid(SkillLevel::Third, 7), one(4));
}

#[test]
fn a_curved_rotation_is_the_same_per_seed_and_stored_by_its_variant() {
    for (place, curved) in CurvedRotation::ALL.into_iter().enumerate() {
        let task = CurvedTask::generate(curved, 9);
        assert_eq!(task, CurvedTask::generate(curved, 9));
        let kind = Kind::CurvedRotation(curved);
        assert_eq!(kind.code(), (28, place as u8));
        assert_eq!(Kind::of_code(28, place as u8), Some(kind));
        let exercise = Exercise::generated(kind, 9);
        assert!(exercise.original().downcast::<CurvedTask>() == Some(&task));
    }
    assert_eq!(Kind::of_code(28, 6), None);
    let facts = Kind::CurvedRotation(CurvedRotation::ALL[0]).facts();
    assert!(facts.compared && !facts.aligned && !facts.constructed);
}
