//! A rotation's givens and Compare (S113): the picture of view A beside the asked box printed in
//! the copy's frame (at 稳 its near upright alone beside the small angle box); Compare lays view
//! B's answer over the copy, seen edges red and hidden ones dashed, a construction to lay with it,
//! and leaves the result to the eye (the author, 2026-11-16): the copy's strokes are never read.

use openpaint_practice::{
    Asked, CardItem, Choices, Compare, Copied, DrawnCopy, Exercise, Figure, In, Kind, Learner,
    Level, LinePen, Pictures, Point, Rgb8, Rotation, RotationTask, Seen, SheetMark, Skill,
    SkillLevel, Standing, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a rotation's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a rotation's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a rotation's copy is never read")
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
    for rotation in Rotation::ALL {
        let exercise = Exercise::generated(Kind::Rotation(rotation), 0x5EED_0113);
        let original = exercise.original();
        let task = original.downcast::<RotationTask>().expect("a rotation");
        let asked = |copy: &'static dyn Copied| Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        assert!(original.compare(&asked(&Empty)).is_none(), "nothing drawn");
        let Some(Compare::Checked(shown)) = original.compare(&asked(&Unread)) else {
            panic!("{rotation:?}: checked, a construction to lay");
        };
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        // The answer: B's visible edges and seen curves red, what its parts hide dashed red.
        let answer = &task.asked.answer;
        let curves = answer.outlines.iter().flatten();
        let seen_curves = curves.clone().filter(|s| !s.hidden).count();
        let hidden_curves = curves.filter(|s| s.hidden).count();
        let marks = shown.marks(&seen(false));
        let [whole, dashed] = lines(&marks, LinePen::Answer);
        assert_eq!(whole, answer.sides.len() + seen_curves, "{rotation:?}");
        assert_eq!(
            dashed,
            task.asked.hidden.len() + hidden_curves,
            "{rotation:?}"
        );
        assert!(
            !task.asked.hidden.is_empty(),
            "{rotation:?}: the box's far edges at least"
        );
        assert!(
            marks
                .iter()
                .all(|mark| !matches!(mark.drawn, Figure::Ring(_)))
        );
        assert_eq!(lines(&marks, LinePen::Pencil), [0, 0]);
        // 看画法: the whole box and V大's construction at B, in the copy's frame.
        let constructed = shown.marks(&seen(true));
        let [box_seen, dashed] = lines(&constructed, LinePen::Pencil);
        let whole_box = 12 - task.asked.seen.hidden().iter().filter(|h| **h).count();
        assert_eq!(
            [box_seen, dashed],
            [whole_box, 12 - whole_box + task.asked.construction().len()]
        );
        let found = (constructed.iter())
            .filter(|mark| matches!(mark.drawn, Figure::Found(_)))
            .count();
        let corners: usize = task.asked.stages.iter().map(|s| s.found.len()).sum();
        assert_eq!(found, corners);
        assert!(constructed.iter().all(|mark| mark.on == In::Copy));
    }
}

#[test]
fn the_givens_are_the_picture_and_the_asked_box_or_at_the_third_level_its_upright_and_angle() {
    for rotation in Rotation::ALL {
        let task = RotationTask::generate(rotation, 0x5EED_0113);
        let original = Exercise::generated(Kind::Rotation(rotation), 0x5EED_0113).original();
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
        let [printed, _] = lines(&givens, LinePen::Printed);
        let [small, _] = lines(&givens, LinePen::PrintedHalf);
        let bare = rotation.turning == openpaint_practice::Turning::Bare;
        assert_eq!(printed, if bare { 1 } else { 3 }, "{rotation:?}");
        assert_eq!(small > 0, bare, "{rotation:?}: the small angle box");
        let painted = original.painted(None).expect("view A's picture");
        assert_eq!(painted.0, [openpaint_practice::OBJECT_SIDE; 2]);
    }
}

#[test]
fn each_level_lays_one_object_of_its_variants_in_turn() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("structure-rotation").expect("the skill");
    assert_eq!(skill.window(), 3);
    let laid = |level| -> Vec<Kind> {
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        planned.exercises.iter().map(|(e, _)| e.kind).collect()
    };
    let one = |k: usize| vec![Kind::Rotation(Rotation::ALL[k])];
    assert_eq!(laid(SkillLevel::First), one(0));
    assert_eq!(laid(SkillLevel::Second), one(2));
    assert_eq!(laid(SkillLevel::Third), one(4));
}
