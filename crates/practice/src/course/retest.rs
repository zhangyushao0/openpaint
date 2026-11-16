//! Re-tests (S147, 复测; V大: try before you are taught): a unit's fixed set, one exercise of each
//! of its skills at the first level, laid at the unit's start and again, once its seal is earned,
//! at its end, so each skill's two copies of the same task show side by side.
//!
//! - The set: each skill of the unit whose pictures are kept, its first level's first exercise at
//!   its first turn, seeded by the unit and the skill, its picture the first its folder picks with
//!   nothing shown before: the same for the same packs. The end lays the start's own exercises
//!   where it was taken (its pictures still kept), so a pack changed between them changes nothing.
//! - The start: Start offers it as the next lesson once the unit is in hand (`Learner::unit`),
//!   from unit 2 on (unit 1's "before" is the first lesson), until it is begun or three lessons of
//!   the unit's skills were taken without it (no longer a before).
//! - The end: offered beside the next lesson, as the review is, once the unit's seal is earned
//!   (each skill with its first dot), until begun; the first such unit first.
//! - Kept as `Picked::Retest`: its results kept as any, left out of the dots' windows and the
//!   redos (`learner.rs`); its copies paired at its own end (`progress/ask.rs`).

use crate::course::learner::Learner;
use crate::course::schedule::{Planned, Standing};
use crate::course::unit::{UNITS, Unit};
use crate::kinds::{Kind, Skill, SkillLevel, Turn};
use crate::maths::random::derived;
use crate::packs::pack::ItemRef;
use crate::packs::pictures::Pictures;
use crate::records::lessons::lessons;
use crate::records::sheet::{Picked, Sheet};
use crate::sheets::exercise::Exercise;

/// The seed every re-test's tasks follow from, with its unit and skill.
const SEED: u64 = 0x5245_5445_5354_0147;
/// Lessons of a unit's skills after which its start re-test is no longer a before.
const BEFORE: u32 = 3;

/// A re-test offered, as the home names it before it is laid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retest {
    pub unit: u8,
    pub end: bool,
    /// Its skills, in the course's order, and about how many minutes its exercises take.
    pub skills: Vec<Skill>,
    pub minutes: u8,
}

/// The start re-test due: the unit in hand from unit 2 on, while it was never begun and fewer
/// than `BEFORE` lessons of its skills were taken; none else.
#[must_use]
pub fn start_due(standing: &Standing<'_>) -> Option<u8> {
    let (learner, pictures) = (standing.learner, standing.pictures);
    let unit = Unit::of(learner.unit(pictures))?;
    let taken: u32 = unit
        .skills
        .iter()
        .map(|skill| learner.lessons_of(*skill))
        .sum();
    let due = unit.number > 1
        && !learner.fresh()
        && learner.open(unit.number, pictures)
        && !learner.retested(unit.number, false)
        && taken < BEFORE;
    due.then_some(unit.number)
}

/// The end re-test due: the first unit whose skills all hold their first dot and whose end
/// re-test was never begun; none else.
#[must_use]
pub fn end_due(learner: &Learner) -> Option<u8> {
    (UNITS.iter())
        .filter(|unit| unit.built())
        .find(|unit| {
            unit.skills.iter().all(|skill| learner.dots(*skill) > 0)
                && !learner.retested(unit.number, true)
        })
        .map(|unit| unit.number)
}

/// The re-test of `unit` at its start or its `end` as offered: its skills and minutes, from what
/// `retest` would lay; none when it lays nothing.
#[must_use]
pub fn offer(unit: u8, end: bool, sheets: &[Sheet], pictures: &Pictures) -> Option<Retest> {
    let planned = retest(unit, end, sheets, pictures)?;
    let skills = (planned.exercises.iter())
        .filter_map(|(exercise, _)| Skill::of(exercise.kind))
        .collect();
    let seconds: u32 = (planned.exercises.iter())
        .map(|(exercise, _)| u32::from(exercise.kind.seconds()))
        .sum();
    Some(Retest {
        unit,
        end,
        skills,
        minutes: u8::try_from(seconds.div_ceil(60).max(1)).unwrap_or(u8::MAX),
    })
}

/// The re-test of `unit` at its start or its `end` as laid (`Picked::Retest`), each exercise at
/// the first level: at the end the start's own exercises where it was begun and their pictures
/// are kept, else the set; none for a unit without a skill whose pictures are kept.
#[must_use]
pub fn retest(unit: u8, end: bool, sheets: &[Sheet], pictures: &Pictures) -> Option<Planned> {
    let held = Unit::of(unit)?;
    let started: Vec<Exercise> = if end {
        (lessons(sheets).iter())
            .filter(|on| !on.is_copy() && on.lesson.retest() == Some((unit, false)))
            .flat_map(|on| on.lesson.exercises.iter())
            .map(|laid| laid.exercise.clone())
            .collect()
    } else {
        Vec::new()
    };
    let kept = |picture: &Option<ItemRef>| picture.as_ref().is_none_or(|item| pictures.holds(item));
    let exercises: Vec<(Exercise, SkillLevel)> = (held.skills.iter())
        .filter(|skill| skill.lacks(pictures).is_none())
        .filter_map(|skill| {
            let before = (started.iter())
                .find(|exercise| Skill::of(exercise.kind) == Some(*skill))
                .filter(|exercise| kept(&exercise.picture))
                .cloned();
            before.or_else(|| set(unit, *skill, pictures))
        })
        .map(|exercise| (exercise, SkillLevel::First))
        .collect();
    (!exercises.is_empty()).then_some(Planned {
        picked: Picked::Retest { unit, end },
        exercises,
    })
}

/// `skill`'s exercise in unit `unit`'s set: its first level's first at its first turn, nothing
/// shown before.
fn set(unit: u8, skill: Skill, pictures: &Pictures) -> Option<Exercise> {
    let seed = derived(derived(SEED, u64::from(unit)), u64::from(skill.0));
    let turn = Turn {
        turn: 0,
        seed,
        pictures,
        used: &[],
        recent: &[],
    };
    let laid: Vec<(Kind, Option<ItemRef>)> = skill.described().lesson(SkillLevel::First, &turn);
    let (kind, picture) = laid.into_iter().next()?;
    Some(Exercise {
        kind,
        seed: derived(seed, 0),
        picture,
    })
}
