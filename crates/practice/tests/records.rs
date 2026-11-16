//! A sheet's record (S120, S122): a lesson begun with each exercise's level, how it was picked and
//! what it practises, a first Compare and its corrections, a result the eye chose kept once and
//! never over the app's, the pictures shown before and lately, and a long colour copy kept beside
//! the lessons, counting in none.

use openpaint_practice::{
    Exercise, First, ItemRef, Judge, Kind, Learner, LineTier, Outcome, Picked, Pictures, Planned,
    Purpose, ScaleTone, Sheet, Skill, SkillLevel, Standing, Tier, recent, schedule, used,
};

fn item(id: &str) -> ItemRef {
    ItemRef {
        pack: "test-shapes".to_owned(),
        item: id.to_owned(),
    }
}

/// A lesson of `exercises`, each at `level`, the `id`th sheet.
fn lesson(id: u64, exercises: Vec<Exercise>, level: SkillLevel) -> Sheet {
    let planned = Planned {
        picked: Picked::Chosen,
        exercises: exercises.into_iter().map(|e| (e, level)).collect(),
    };
    Sheet::begun(id, (Purpose::Day, 1_791_201_600), planned)
}

/// A shape copying `id`.
fn shape(id: &str) -> Exercise {
    Exercise {
        picture: Some(item(id)),
        ..Exercise::generated(Kind::Shape(Tier::Simple), 3)
    }
}

#[test]
fn a_long_colour_copy_is_kept_beside_the_lessons_and_counts_in_none() {
    let mut copy = Sheet::copy(5, (77, 100), item("teapot"));
    assert!(copy.is_copy());
    assert_eq!(copy.purpose, Purpose::Copy);
    let held = copy.lesson().expect("its lesson");
    assert_eq!(
        (held.picked, held.skill(), held.retest()),
        (Picked::Chosen, None, None)
    );
    assert_eq!(copy.exercises().count(), 1);
    let laid = copy.laid(0).expect("its exercise");
    assert_eq!(laid.exercise.kind, Kind::Study);
    assert_eq!(laid.exercise.picture, Some(item("teapot")));
    copy.laid_mut(0)
        .expect("its exercise")
        .compared(First::of(1, 40, Vec::new()));
    let copies = std::slice::from_ref(&copy);
    assert!(used(copies).is_empty() && recent(copies).is_empty());
    assert_eq!(Learner::of(copies), Learner::default(), "no lesson taken");
}

#[test]
fn a_lesson_begun_keeps_each_exercise_s_level_and_what_it_practises() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let lines = Skill::of_key("line").expect("lines");
    let planned = schedule::lesson(lines, SkillLevel::Third, 9, &standing).expect("a lesson");
    let sheet = Sheet::begun(2, (Purpose::Day, 0), planned);
    let held = sheet.lesson().expect("its lesson");
    assert_eq!(
        (held.picked, held.skill(), held.retest()),
        (Picked::Chosen, Some(lines), None)
    );
    assert!(
        sheet
            .exercises()
            .all(|laid| laid.level == SkillLevel::Third.code())
    );
    assert_eq!(SkillLevel::of_code(2), SkillLevel::Third);
    // A scale's lesson is the scales' skill; a first lesson practises none of its own.
    let scales = lesson(
        3,
        vec![Exercise::generated(Kind::Scale(ScaleTone::Hue, 5), 3)],
        SkillLevel::First,
    );
    assert_eq!(scales.lessons[0].skill(), Skill::of_key("scale"));
    let first = Sheet::begun(4, (Purpose::Day, 0), schedule::first(1));
    assert_eq!(
        (first.lessons[0].picked, first.lessons[0].skill()),
        (Picked::First, None)
    );
}

#[test]
fn a_first_compare_marks_the_corrections_after_it_and_the_eye_s_result_is_kept_once() {
    let line = Exercise::generated(Kind::Line(LineTier::Arc), 1);
    let mut sheet = lesson(3, vec![line.clone(), line], SkillLevel::First);
    assert_eq!(sheet.at(), 0);
    let laid = sheet.laid_mut(1).expect("an exercise");
    laid.compared(First::of(1_791_201_600, 40, Vec::new()));
    assert!(sheet.first(1).is_some() && sheet.first(0).is_none());
    let laid = sheet.laid(1).expect("compared");
    assert_eq!(laid.compares, 1);
    let first = laid.first.clone().expect("its first Compare");
    assert!(!first.corrects(39) && first.corrects(40) && first.corrects(41));
    // The eye's choice is kept once, never over the app's.
    let laid = sheet.laid_mut(1).expect("an exercise");
    laid.judged(Outcome::Near);
    laid.judged(Outcome::Clean);
    assert_eq!(laid.result, Some((Outcome::Near, Judge::Eye)));
    let laid = sheet.laid_mut(0).expect("an exercise");
    laid.result = Some((Outcome::Redo, Judge::App));
    laid.judged(Outcome::Clean);
    assert_eq!(laid.result, Some((Outcome::Redo, Judge::App)));
}

#[test]
fn the_pictures_shown_are_every_lesson_s_and_the_last_ten_s() {
    let mut sheets: Vec<Sheet> = (1..=12)
        .map(|id| lesson(id, vec![shape(&format!("s-{id}"))], SkillLevel::First))
        .collect();
    sheets.push(lesson(13, vec![shape("s-1")], SkillLevel::Second));
    let shown = used(&sheets);
    assert_eq!(shown.len(), 12, "each once");
    let lately = recent(&sheets);
    assert_eq!(lately.len(), 10, "the last ten lessons'");
    assert!(lately.contains(&item("s-1")) && !lately.contains(&item("s-2")));
    let learner = Learner::of(&sheets);
    assert_eq!(learner.used().len(), 12);
    assert_eq!(learner.recent().len(), 10);
}
