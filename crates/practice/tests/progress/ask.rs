//! Which pair a lesson's end asks (S147): never today's copy, two copies of the lesson's skill and
//! level, the newer from the last two weeks, the older three to six weeks before it, the pair whose
//! difference is least sure; a test sheet's copies only at a test's end; a pair skipped today, or a
//! copy paired already, not again.

use openpaint_practice::{
    Answer, BlindPair, CopyRef, Exercise, Judge, Kind, LineTier, Outcome, PairAsked, Pictures,
    Purpose, Sheet, SkillFit, Within, ask,
};

use super::lines;
use super::sim::{DAY, NOON, lesson};

/// NOON's local day in UTC.
const ZERO: i64 = 20_731;

fn day(of: u64) -> u64 {
    NOON + of * DAY
}

/// Lessons of lines on days 0, 10, 20, 30, 40 and 50, a lesson of combined lines on day 50, a
/// test's lesson of lines on day 45, one of lines at the second level on day 15, and today's (52).
fn records() -> Vec<Sheet> {
    let near = [Outcome::Near, Outcome::Clean, Outcome::Near];
    let mut sheets: Vec<Sheet> = [0, 10, 20, 30, 40, 50]
        .iter()
        .enumerate()
        .map(|(i, d)| lesson(i as u64 + 1, day(*d), 0, &near))
        .collect();
    let mut combined = lesson(7, day(50), 0, &near);
    for laid in combined.lessons[0].exercises.iter_mut() {
        laid.exercise = Exercise {
            kind: Kind::Combined(openpaint_practice::Combination::ALL[0]),
            ..laid.exercise.clone()
        };
    }
    sheets.push(combined);
    let mut test = lesson(8, day(45), 0, &near);
    test.purpose = Purpose::Test;
    sheets.push(test);
    sheets.push(lesson(9, day(15), 1, &near));
    sheets.push(lesson(10, day(52), 0, &near));
    sheets
}

fn within<'a>(pictures: &'a Pictures, skipped: &'a [PairAsked], test: bool) -> Within<'a> {
    Within {
        test,
        pictures,
        skipped,
    }
}

fn copy_day(sheets: &[Sheet], copy: CopyRef) -> i64 {
    let (_, laid) = copy.find(sheets).expect("the copy");
    let at = laid.first.as_ref().expect("compared").at;
    i64::try_from(at / DAY).expect("a day") - ZERO
}

#[test]
fn a_pair_is_two_earlier_copies_of_the_skill_weeks_apart_the_least_sure() {
    let sheets = records();
    let pictures = Pictures::default();
    let today = ZERO + 52;
    let asked = ask(
        lines(),
        (&sheets, &[]),
        (today, 0),
        &within(&pictures, &[], false),
    )
    .expect("a pair");
    let (older, newer) = (
        copy_day(&sheets, asked.older),
        copy_day(&sheets, asked.newer),
    );
    assert!((38..=51).contains(&newer), "the newer drawn on day {newer}");
    assert!((21..=42).contains(&(newer - older)), "{older} and {newer}");
    for copy in [asked.older, asked.newer] {
        let (sheet, laid) = copy.find(&sheets).expect("a copy");
        assert_eq!(
            sheet.purpose,
            Purpose::Day,
            "a test's copy at a day's lesson"
        );
        assert_eq!(
            laid.exercise.kind,
            Kind::Line(LineTier::Straight),
            "the lesson's skill"
        );
        assert_eq!(laid.level, 0);
        assert_eq!(laid.result.map(|(_, judge)| judge), Some(Judge::Eye));
    }
    // The largest posterior variance among every pair the rules allow.
    let fit = SkillFit::of(lines(), &sheets, &[], 0).expect("a fit");
    let mut best = 0.0_f64;
    for new in [40, 50] {
        for old in [0, 10, 20, 30] {
            if (21..=42).contains(&(new - old)) {
                best = best.max(fit.spread(ZERO + old, ZERO + new).expect("practised"));
            }
        }
    }
    let chosen = fit.spread(ZERO + older, ZERO + newer).expect("practised");
    assert!((chosen - best).abs() < 1e-12, "{chosen} of {best}");
}

#[test]
fn a_test_s_copies_a_skipped_pair_and_a_copy_paired_already_are_left_out() {
    let sheets = records();
    let pictures = Pictures::default();
    let today = ZERO + 52;
    let first = ask(
        lines(),
        (&sheets, &[]),
        (today, 0),
        &within(&pictures, &[], false),
    )
    .expect("a pair");
    let skipped = [first];
    let again = ask(
        lines(),
        (&sheets, &[]),
        (today, 0),
        &within(&pictures, &skipped, false),
    );
    assert_ne!(again, Some(first), "skipped today");
    // The newer copy answered once is never asked again; every copy of a pair is the newer once.
    let answered = BlindPair {
        asked: day(52),
        lesson: (10, 0),
        older: first.older,
        newer: first.newer,
        older_left: true,
        answer: Answer::Alike,
    };
    let pairs = [answered];
    let next = ask(
        lines(),
        (&sheets, &pairs),
        (today, 0),
        &within(&pictures, &[], false),
    )
    .expect("another pair");
    assert_ne!(next.newer, first.newer);
    // At a test's end a test's copy may be the newer.
    let tested = ask(
        lines(),
        (&sheets, &[]),
        (today, 0),
        &within(&pictures, &[], true),
    )
    .expect("a pair");
    let (_, laid) = tested.newer.find(&sheets).expect("a copy");
    assert!(laid.result.is_some());
    // Nothing the day after the first lesson.
    let fresh = [lesson(1, day(0), 0, &[Outcome::Clean])];
    assert!(
        ask(
            lines(),
            (&fresh, &[]),
            (ZERO + 1, 0),
            &within(&pictures, &[], false)
        )
        .is_none()
    );
}
