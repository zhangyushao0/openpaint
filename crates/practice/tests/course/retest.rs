//! A unit's re-tests (S147): the same tasks at its start and its end, laid at the first level;
//! no practice at the learner's level (no dot, no redo, no turn); the start due from unit 2 on until
//! begun or three of the unit's lessons pass, the end once the unit's seal is earned until begun.

use openpaint_practice::{
    Learner, Outcome, Picked, Pictures, Purpose, Sheet, SkillLevel, Standing, retest,
};

use super::{CLEAN, NOON, after, ended, skill};

/// `learner` with the skill keyed `key`'s first dot.
fn dotted(learner: &Learner, key: &str) -> Learner {
    let mut learner = learner.clone();
    for _ in 0..4 {
        if learner.dots(skill(key)) > 0 {
            break;
        }
        learner = after(&learner, key, SkillLevel::First, &CLEAN);
    }
    assert!(learner.dots(skill(key)) > 0, "{key} dotted");
    learner
}

/// A learner past the first lesson with each of unit 1's skills dotted: unit 2 in hand.
fn past_unit_one() -> Learner {
    let first = ended(openpaint_practice::schedule::first(1), &[]);
    let learner = Learner::default().with(&first);
    ["line", "combined", "colour"]
        .iter()
        .fold(learner, |learner, key| dotted(&learner, key))
}

#[test]
fn a_unit_s_re_test_lays_the_same_tasks_at_its_start_and_its_end() {
    let pictures = Pictures::default();
    let start = retest::retest(3, false, &[], &pictures).expect("unit 3's re-test");
    assert_eq!(
        start.picked,
        Picked::Retest {
            unit: 3,
            end: false
        }
    );
    let skills: Vec<_> = (start.exercises.iter())
        .filter_map(|(exercise, _)| openpaint_practice::Skill::of(exercise.kind))
        .collect();
    assert_eq!(
        skills,
        ["estimates", "points-in-space", "solids"].map(skill)
    );
    assert!(
        start
            .exercises
            .iter()
            .all(|(_, level)| *level == SkillLevel::First)
    );
    let mut sheet = Sheet::begun(5, (Purpose::Day, NOON), start.clone());
    sheet.canvas = 5;
    let end = retest::retest(3, true, &[sheet], &pictures).expect("its end");
    assert_eq!(end.picked, Picked::Retest { unit: 3, end: true });
    let tasks = |planned: &openpaint_practice::Planned| -> Vec<_> {
        planned.exercises.iter().map(|(e, _)| e.clone()).collect()
    };
    assert_eq!(tasks(&end), tasks(&start), "the start's own tasks");
    let unstarted = retest::retest(3, true, &[], &pictures).expect("an end without a start");
    assert_eq!(tasks(&unstarted), tasks(&start), "the same set without one");
    // Unit 8 holds a skill needing no pack since its organic arrows (S156l); no unit is 9.
    assert!(
        retest::retest(9, false, &[], &pictures).is_none(),
        "a unit without a skill"
    );
    let arrows = retest::retest(8, false, &[], &pictures).expect("unit 8's re-test");
    let first = arrows.exercises.first().map(|(e, _)| e.kind);
    assert_eq!(
        first.and_then(openpaint_practice::Skill::of),
        Some(skill("organic-arrow"))
    );
}

#[test]
fn a_re_test_fills_no_window_leaves_no_redo_and_takes_no_turn() {
    let pictures = Pictures::default();
    let planned = retest::retest(3, false, &[], &pictures).expect("unit 3's re-test");
    let lesson = ended(planned, &[Outcome::Redo, Outcome::Redo, Outcome::Redo]);
    let learner = Learner::default().with(&lesson);
    assert_eq!(learner.lessons(), 1);
    assert!(learner.redos().next().is_none());
    assert_eq!(learner.lessons_of(skill("solids")), 0);
    assert!(learner.retested(3, false) && !learner.retested(3, true));
}

#[test]
fn the_start_is_due_from_unit_two_until_begun_or_three_lessons_pass() {
    let pictures = Pictures::default();
    let standing = |learner: &Learner| {
        retest::start_due(&Standing {
            pictures: &pictures,
            learner,
        })
    };
    let first = ended(openpaint_practice::schedule::first(1), &[]);
    let fresh = Learner::default().with(&first);
    assert_eq!(
        standing(&fresh),
        None,
        "unit 1's before is the first lesson"
    );
    let past = past_unit_one();
    // Unit 2's skills that need no pack: value scales, boxes, free boxes, ellipses.
    assert_eq!(past.unit(&pictures), 2);
    assert_eq!(standing(&past), Some(2));
    let planned = retest::retest(2, false, &[], &pictures).expect("unit 2's");
    let begun = past.with(&ended(planned, &[]));
    assert_eq!(standing(&begun), None, "begun");
    let mut tapped = past.clone();
    for key in ["box", "ellipse", "scale"] {
        assert_eq!(standing(&tapped), Some(2), "before {key}");
        tapped = after(&tapped, key, SkillLevel::First, &[Outcome::Near; 6]);
    }
    assert_eq!(
        standing(&tapped),
        None,
        "three lessons in: no longer a before"
    );
}

#[test]
fn the_end_is_due_once_the_unit_s_seal_is_earned_until_begun() {
    let pictures = Pictures::default();
    let first = ended(openpaint_practice::schedule::first(1), &[]);
    assert_eq!(retest::end_due(&Learner::default().with(&first)), None);
    let past = past_unit_one();
    assert_eq!(retest::end_due(&past), Some(1));
    let offer = retest::offer(1, true, &[], &pictures).expect("unit 1's end");
    assert_eq!(offer.skills, ["line", "combined", "colour"].map(skill));
    assert!(offer.minutes > 0);
    let planned = retest::retest(1, true, &[], &pictures).expect("unit 1's end");
    let begun = past.with(&ended(planned, &[]));
    assert_eq!(retest::end_due(&begun), None);
}

/// Sheet `id` begun at `at` holding `planned`, each exercise compared then.
fn compared(id: u64, at: u64, planned: openpaint_practice::Planned) -> Sheet {
    let mut sheet = Sheet::begun(id, (Purpose::Day, at), planned);
    sheet.canvas = id;
    for index in 0..sheet.exercises().count() {
        let laid = sheet.laid_mut(index).expect("an exercise");
        laid.compared(openpaint_practice::First::of(at + 60, 1, Vec::new()));
    }
    sheet
}

#[test]
fn a_unit_s_end_re_test_pairs_each_copy_with_its_start_or_else_the_skill_s_first() {
    let pictures = Pictures::default();
    let start = retest::retest(3, false, &[], &pictures).expect("unit 3's start");
    let end = retest::retest(3, true, &[], &pictures).expect("its end");
    let before = compared(2, NOON, start);
    let after = compared(9, NOON + 40 * 86_400, end);
    let all = [before.clone(), after.clone()];
    let pairs = openpaint_practice::retest_pairs(&after, 0, &all);
    assert_eq!(pairs.len(), 3, "one a skill");
    for (at, pair) in pairs.iter().enumerate() {
        let index = u16::try_from(at).expect("small");
        assert_eq!(
            pair.newer,
            openpaint_practice::CopyRef {
                sheet: 9,
                exercise: index
            }
        );
        assert_eq!(
            pair.older,
            openpaint_practice::CopyRef {
                sheet: 2,
                exercise: index
            }
        );
    }
    // Without a start, a skill's first copy ever: an earlier lesson of estimates.
    let pictures = Pictures::default();
    let learner = Learner::default();
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let lesson =
        openpaint_practice::schedule::lesson(skill("estimates"), SkillLevel::First, 4, &standing)
            .expect("a lesson of estimates");
    let earlier = compared(1, NOON - 86_400, lesson);
    let all = [earlier, after.clone()];
    let pairs = openpaint_practice::retest_pairs(&after, 0, &all);
    assert_eq!(pairs.len(), 1, "only estimates had a copy before");
    assert_eq!(pairs[0].older.sheet, 1);
    assert!(
        openpaint_practice::retest_pairs(&before, 0, std::slice::from_ref(&before)).is_empty(),
        "a start asks none"
    );
}
