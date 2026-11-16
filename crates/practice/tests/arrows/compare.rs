//! An organic arrow's givens and Compare (S156l): the path dashed grey, the arrowhead and its ticks
//! printed in the one frame; Compare lays the exact ribbon over the copy, its edges and folds red
//! and what it hides dashed, the widths to lay with it, and leaves the result to the eye (D123):
//! the copy's strokes are never read.

use openpaint_practice::{
    Arrow, ArrowCompared, ArrowTask, Asked, CardItem, Choices, Compare, Copied, DrawnCopy,
    Exercise, Figure, In, Kind, Learner, Level, LinePen, Pictures, Point, Rgb8, Seen, SheetMark,
    Skill, SkillLevel, Standing, UNITS, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("an organic arrow's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("an organic arrow's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("an organic arrow's copy is never read")
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

/// The lines among `marks` in `pen` in the one frame: whole and dashed.
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
fn compare_lays_the_ribbon_for_the_eye_and_never_reads_the_copy() {
    for arrow in Arrow::ALL {
        let exercise = Exercise::generated(Kind::Arrow(arrow), 0x5EED_0156);
        let original = exercise.original();
        let task = original.downcast::<ArrowTask>().expect("an arrow");
        let asked = |copy: &'static dyn Copied| Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        assert!(original.compare(&asked(&Empty)).is_none(), "nothing drawn");
        let Some(Compare::Checked(shown)) = original.compare(&asked(&Unread)) else {
            panic!("{arrow:?}: checked, its widths to lay");
        };
        assert!(shown.downcast::<ArrowCompared>().is_some());
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        // The ribbon: its edges and folds red where seen, dashed red where it hides them.
        let answer = &task.answer;
        let stretches = (answer.stretches.iter().flatten()).chain(answer.folds.iter().flatten());
        let hidden = stretches.clone().filter(|s| s.hidden).count();
        let whole = stretches.count() - hidden;
        let marks = shown.marks(&seen(false));
        assert_eq!(lines(&marks, LinePen::Answer), [whole, hidden], "{arrow:?}");
        assert!(hidden > 0 && answer.folds.len() >= 3, "{arrow:?}");
        assert_eq!(lines(&marks, LinePen::Faint), [0, 0]);
        let ringed = |mark: &SheetMark| matches!(mark.drawn, Figure::Ring(_));
        assert!(!marks.iter().any(ringed));
        // 看画法: the width laid across at even steps, faint, dashed where hidden.
        let constructed = shown.marks(&seen(true));
        let [faint, dashed] = lines(&constructed, LinePen::Faint);
        assert_eq!(faint + dashed, answer.widths.len(), "{arrow:?}");
        assert!(faint >= 12, "{arrow:?}: a width at each step");
        assert!(constructed.iter().all(|mark| mark.on == In::Copy));
        // Its answer as a hand draws it: each seen stretch once.
        assert_eq!(original.answer().len(), whole);
    }
}

#[test]
fn the_givens_are_the_path_dashed_and_the_arrowhead_with_its_widths_ticked() {
    for arrow in Arrow::ALL {
        let original = Exercise::generated(Kind::Arrow(arrow), 0x5EED_0156).original();
        let task = original.downcast::<ArrowTask>().expect("an arrow");
        let givens = original.givens();
        assert!(givens.iter().all(|mark| mark.on == In::Copy), "one frame");
        let [printed, dashed] = lines(&givens, LinePen::Printed);
        // The head, and the near width ticked; at 入门 the far width too.
        let ticks = if arrow == Arrow::Gentle { 4 } else { 2 };
        assert_eq!((printed, dashed), (1 + ticks, 1), "{arrow:?}");
        assert_eq!(task.ticks.len(), ticks);
        let Figure::Line { points, .. } = &givens[0].drawn else {
            panic!("the path");
        };
        assert_eq!(points, &task.path);
        let facts = Kind::Arrow(arrow).facts();
        assert!(facts.compared && facts.constructed && !facts.aligned);
    }
}

#[test]
fn a_lesson_lays_two_arrows_of_its_level_first_in_unit_eight() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("organic-arrow").expect("the skill");
    assert_eq!(skill.window(), 6);
    assert_eq!(
        UNITS[7].skills.first(),
        Some(&skill),
        "unit 8's first skill"
    );
    let laid = |level| -> Vec<Kind> {
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        planned.exercises.iter().map(|(e, _)| e.kind).collect()
    };
    let two = |arrow| vec![Kind::Arrow(arrow); 2];
    assert_eq!(laid(SkillLevel::First), two(Arrow::Gentle));
    assert_eq!(laid(SkillLevel::Second), two(Arrow::Crossing));
    assert_eq!(laid(SkillLevel::Third), two(Arrow::Turning));
}

#[test]
fn an_arrow_is_the_same_per_seed_and_stored_by_its_variant() {
    for (place, arrow) in Arrow::ALL.into_iter().enumerate() {
        let task = ArrowTask::generate(arrow, 9);
        assert_eq!(task, ArrowTask::generate(arrow, 9));
        let kind = Kind::Arrow(arrow);
        assert_eq!(kind.code(), (33, place as u8));
        assert_eq!(Kind::of_code(33, place as u8), Some(kind));
        let exercise = Exercise::generated(kind, 9);
        assert!(exercise.original().downcast::<ArrowTask>() == Some(&task));
    }
    assert_eq!(Kind::of_code(33, 3), None);
}
