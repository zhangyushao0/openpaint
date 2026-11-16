//! Which two copies a lesson's end asks the learner to compare blind (比一比, S147).
//!
//! - A lesson of a skill the eye judges asks at most one pair, both copies earlier than the lesson
//!   (never the copy just drawn: it is known at a glance, and knowing which is newer bends the
//!   answer), of the same skill and level, each compared and judged by the learner's eye, its
//!   picture still kept: the newer drawn 1 to 14 days before today and never yet in a pair, the
//!   older 21 to 42 days before the newer and in at most one pair. Re-tests' copies are paired at
//!   their own end; a test sheet's only at a test lesson's end; a copy of a pair asked today (one
//!   skipped among them) not again that day.
//! - Among those, the pair whose difference of abilities on its two days is least sure, the largest
//!   posterior variance (ASAP's choice of the most informative comparison, Mikhailiuk et al. 2020,
//!   simplified: the variance alone); ties the older nearest four weeks before the newer, then the
//!   newest newer.
//! - A unit's end re-test asks one pair a skill (`retest_pairs`): its copy just drawn against the
//!   start re-test's of the same task, or, where none was taken, the skill's first copy ever.

use crate::kinds::Skill;
use crate::packs::pictures::Pictures;
use crate::progress::SkillFit;
use crate::progress::observe::copy_day;
use crate::records::lessons::{OnSheet, lessons};
use crate::records::pairs::{BlindPair, CopyRef};
use crate::records::sheet::{Judge, Laid, Picked, Purpose, Sheet};

/// The newer copy's days before today, the older's before the newer, the pairs the older may be
/// in already, and the days a change is read over.
const NEWER: (i64, i64) = (1, 14);
const OLDER: (i64, i64) = (21, 42);
const OLDER_PAIRS: usize = 1;
const FOUR_WEEKS: i64 = 28;

/// A pair a lesson's end asks: its older and its newer copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PairAsked {
    pub older: CopyRef,
    pub newer: CopyRef,
}

/// What else bounds the choice: whether the lesson is a test's, the pictures kept and the pairs
/// skipped today.
#[derive(Debug, Clone, Copy)]
pub struct Within<'a> {
    pub test: bool,
    pub pictures: &'a Pictures,
    pub skipped: &'a [PairAsked],
}

/// A copy that may be asked, with its day and level.
struct Candidate {
    copy: CopyRef,
    day: i64,
    level: u8,
}

/// The pair a lesson of `skill` ending on local day `today` asks, from `sheets` and `pairs` (days on
/// a clock `offset` minutes from UTC); none when no two copies qualify.
#[must_use]
pub fn ask(
    skill: Skill,
    (sheets, pairs): (&[Sheet], &[BlindPair]),
    (today, offset): (i64, i32),
    within: &Within<'_>,
) -> Option<PairAsked> {
    let all = candidates(skill, sheets, offset, within);
    let paired = |copy: CopyRef| pairs.iter().filter(|pair| pair.holds(copy)).count();
    let newer = (all.iter())
        .filter(|c| (today - NEWER.1..=today - NEWER.0).contains(&c.day) && paired(c.copy) == 0);
    let mut best: Option<(f64, i64, i64, PairAsked)> = None;
    let mut fitted: Option<Option<SkillFit>> = None;
    for new in newer {
        let older = (all.iter()).filter(|old| {
            old.level == new.level
                && (new.day - OLDER.1..=new.day - OLDER.0).contains(&old.day)
                && paired(old.copy) <= OLDER_PAIRS
        });
        for old in older {
            let asked = PairAsked {
                older: old.copy,
                newer: new.copy,
            };
            let seen =
                |copy: CopyRef| (within.skipped.iter()).any(|s| s.older == copy || s.newer == copy);
            if seen(asked.older) || seen(asked.newer) {
                continue;
            }
            let fit = fitted.get_or_insert_with(|| SkillFit::of(skill, sheets, pairs, offset));
            let spread = fit
                .as_ref()
                .and_then(|fit| fit.spread(old.day, new.day))
                .unwrap_or(0.0);
            let near = -(new.day - FOUR_WEEKS - old.day).abs();
            let key = (spread, near, new.day, asked);
            let better = best.as_ref().is_none_or(|held| {
                let (s, n, d, a) = held;
                key.0 > *s + 1e-9
                    || ((key.0 - *s).abs() <= 1e-9
                        && (key.1, key.2, ord(asked)) > (*n, *d, ord(*a)))
            });
            if better {
                best = Some(key);
            }
        }
    }
    best.map(|(_, _, _, asked)| asked)
}

/// A pair's order among ties: the lower copies last.
fn ord(asked: PairAsked) -> (std::cmp::Reverse<CopyRef>, std::cmp::Reverse<CopyRef>) {
    (
        std::cmp::Reverse(asked.newer),
        std::cmp::Reverse(asked.older),
    )
}

/// Every copy of `skill` that may be asked: in a skill's lesson (no first lesson, re-test or colour
/// copy), compared and judged by eye, its picture kept, a test sheet's only for a test.
fn candidates(skill: Skill, sheets: &[Sheet], offset: i32, within: &Within<'_>) -> Vec<Candidate> {
    let mut all = Vec::new();
    for on in lessons(sheets) {
        let test = on.sheet.purpose == Purpose::Test;
        if on.is_copy() || on.lesson.picked != Picked::Chosen || (test && !within.test) {
            continue;
        }
        let first = on.first_exercise();
        for (at, laid) in on.lesson.exercises.iter().enumerate() {
            let judged = matches!(laid.result, Some((_, Judge::Eye))) && laid.first.is_some();
            let kept =
                (laid.exercise.picture.as_ref()).is_none_or(|item| within.pictures.holds(item));
            if judged && kept && Skill::of(laid.exercise.kind) == Some(skill) {
                all.push(Candidate {
                    copy: CopyRef::of(&on, first + at),
                    day: copy_day(on.sheet, laid, offset),
                    level: laid.level,
                });
            }
        }
    }
    all
}

/// The pairs a unit's end re-test, lesson `lesson` of `sheet`, asks: each of its compared copies
/// against its skill's start re-test copy of the same unit, else the skill's first compared copy
/// ever; in the lesson's order.
#[must_use]
pub fn retest_pairs(sheet: &Sheet, lesson: usize, sheets: &[Sheet]) -> Vec<PairAsked> {
    let Some(on) = OnSheet::of(sheet, lesson) else {
        return Vec::new();
    };
    let Some((unit, true)) = on.lesson.retest() else {
        return Vec::new();
    };
    let all = lessons(
        sheets
            .iter()
            .filter(|held| held.id != sheet.id)
            .chain([sheet]),
    );
    let compared = |laid: &Laid| laid.first.is_some();
    let first = on.first_exercise();
    (on.lesson.exercises.iter().enumerate())
        .filter(|(_, laid)| compared(laid))
        .filter_map(|(at, laid)| {
            let skill = Skill::of(laid.exercise.kind)?;
            let alike = |held: &&OnSheet<'_>, laid: &Laid| {
                !held.is_copy() && compared(laid) && Skill::of(laid.exercise.kind) == Some(skill)
            };
            let copies = |want: Option<(u8, bool)>| {
                (all.iter())
                    .filter(move |held| want.is_none() || held.lesson.retest() == want)
                    .filter(|held| held.order() < on.order())
                    .flat_map(|held| {
                        let start = held.first_exercise();
                        (held.lesson.exercises.iter().enumerate())
                            .filter(move |(_, laid)| alike(&held, laid))
                            .map(move |(i, _)| CopyRef::of(held, start + i))
                    })
            };
            let older = (copies(Some((unit, false))).next()).or_else(|| copies(None).next())?;
            Some(PairAsked {
                older,
                newer: CopyRef::of(&on, first + at),
            })
        })
        .collect()
}
