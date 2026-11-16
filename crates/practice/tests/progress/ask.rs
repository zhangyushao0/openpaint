//! Which pair a lesson's end asks (S147): never today's copy, two copies of the lesson's skill and
//! level, the newer from the last two weeks, the older three to six weeks before it, the pair whose
//! difference is least sure; a test sheet's copies only at a test's end; a pair skipped today, or a
//! copy paired already, not again. Among several skills the least sure of all; a Pomodoro's block's
//! end (S148) among its lessons' skills, or its end re-test's own pairs.

use openpaint_practice::{
    Answer, BlindPair, CopyRef, Exercise, Kind, LineTier, Outcome, PairAsked, Pictures, Purpose,
    Sheet, Skill, SkillFit, Within, ask, block_pairs,
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
        &[lines()],
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
        assert!(laid.first.is_some(), "a compared copy");
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

/// Copies compared and never judged still make a pair (S150); no fit is needed to choose it.
#[test]
fn copies_never_judged_still_pair() {
    let mut sheets = records();
    for laid in (sheets.iter_mut()).flat_map(|sheet| {
        sheet
            .lessons
            .iter_mut()
            .flat_map(|l| l.exercises.iter_mut())
    }) {
        laid.result = None;
    }
    let pictures = Pictures::default();
    let asked = ask(
        &[lines()],
        (&sheets, &[]),
        (ZERO + 52, 0),
        &within(&pictures, &[], false),
    )
    .expect("a pair of copies never judged");
    for copy in [asked.older, asked.newer] {
        let (_, laid) = copy.find(&sheets).expect("a copy");
        assert!(laid.first.is_some() && laid.result.is_none());
    }
    let newer = copy_day(&sheets, asked.newer);
    assert!((38..=51).contains(&newer), "the newer drawn on day {newer}");
}

#[test]
fn a_test_s_copies_a_skipped_pair_and_a_copy_paired_already_are_left_out() {
    let sheets = records();
    let pictures = Pictures::default();
    let today = ZERO + 52;
    let first = ask(
        &[lines()],
        (&sheets, &[]),
        (today, 0),
        &within(&pictures, &[], false),
    )
    .expect("a pair");
    let skipped = [first];
    let again = ask(
        &[lines()],
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
        &[lines()],
        (&sheets, &pairs),
        (today, 0),
        &within(&pictures, &[], false),
    )
    .expect("another pair");
    assert_ne!(next.newer, first.newer);
    // At a test's end a test's copy may be the newer.
    let tested = ask(
        &[lines()],
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
            &[lines()],
            (&fresh, &[]),
            (ZERO + 1, 0),
            &within(&pictures, &[], false)
        )
        .is_none()
    );
}

/// `sheet` with its lesson's exercises all of `kind`.
fn of_kind(mut sheet: Sheet, kind: Kind) -> Sheet {
    for laid in &mut sheet.lessons[0].exercises {
        laid.exercise = Exercise {
            kind,
            ..laid.exercise.clone()
        };
    }
    sheet
}

/// The records with a lesson of combined lines on day 20 too: each skill has a pair to ask.
fn two_skills() -> (Vec<Sheet>, Skill) {
    let combined = Kind::Combined(openpaint_practice::Combination::ALL[0]);
    let mut sheets = records();
    let near = [Outcome::Near, Outcome::Clean, Outcome::Near];
    sheets.push(of_kind(lesson(11, day(20), 0, &near), combined));
    (sheets, Skill::of(combined).expect("combined lines"))
}

#[test]
fn among_several_skills_the_pair_asked_is_the_least_sure_of_all_their_pairs() {
    let (sheets, combined) = two_skills();
    let pictures = Pictures::default();
    let (today, none) = ((ZERO + 52, 0), within(&pictures, &[], false));
    let spread = |skill, pair: PairAsked| {
        let fit = SkillFit::of(skill, &sheets, &[], 0).expect("a fit");
        let days = (copy_day(&sheets, pair.older), copy_day(&sheets, pair.newer));
        fit.spread(ZERO + days.0, ZERO + days.1).expect("practised")
    };
    let of = |skills: &[Skill]| ask(skills, (&sheets, &[]), today, &none);
    let line = of(&[lines()]).expect("a pair of lines");
    let joined = of(&[combined]).expect("a pair of combined lines");
    let (a, b) = (spread(lines(), line), spread(combined, joined));
    assert!((a - b).abs() > 1e-9, "{a} and {b} tie");
    let least_sure = if a > b { line } else { joined };
    assert_eq!(of(&[lines(), combined]), Some(least_sure));
    assert_eq!(of(&[combined, lines()]), Some(least_sure), "in any order");
    assert_eq!(of(&[]), None);
}

#[test]
fn a_block_asks_among_its_lessons_skills_under_its_last_lesson_or_its_end_re_test_s_pairs() {
    let (mut sheets, _) = two_skills();
    let pictures = Pictures::default();
    let (today, none) = ((ZERO + 52, 0), within(&pictures, &[], false));
    // The block of today's lesson of lines (sheet 10): one pair of lines, never today's copy.
    let lines_only = ask(&[lines()], (&sheets, &[]), today, &none);
    let asked = block_pairs(day(52), (&sheets, &[]), today, &none);
    assert_eq!(asked, Some(((10, 0), lines_only.into_iter().collect())));
    let pair = lines_only.expect("a pair");
    assert!(pair.newer.sheet != 10 && pair.older.sheet != 10);
    // A block after every lesson holds none.
    assert_eq!(block_pairs(day(53), (&sheets, &[]), today, &none), None);
    // The first lesson's skills ask nothing.
    let mut first = Sheet::begun(
        12,
        (Purpose::Day, day(52) + 1000),
        openpaint_practice::schedule::first(1),
    );
    first.lessons[0].finished = Some(day(52) + 1100);
    sheets.push(first);
    let since = day(52) + 950;
    let asked = block_pairs(since, (&sheets, &[]), today, &none);
    assert_eq!(asked, Some(((12, 0), Vec::new())));
    // A unit's end re-test in the block: its own pairs, under it.
    let mut retest = lesson(13, day(52) + 1200, 0, &[Outcome::Clean, Outcome::Near]);
    retest.lessons[0].picked = openpaint_practice::Picked::Retest { unit: 1, end: true };
    sheets.push(retest.clone());
    let pairs = openpaint_practice::retest_pairs(&retest, 0, &sheets);
    assert_eq!(pairs.len(), 2, "one a copy");
    let asked = block_pairs(since, (&sheets, &[]), today, &none);
    assert_eq!(asked, Some(((13, 0), pairs)));
}
