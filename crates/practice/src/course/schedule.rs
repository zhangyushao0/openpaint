//! The lessons offered and laid (S122), from where the learner stands and the pictures kept:
//! the next lesson (`next`), the review on the side when a redo waits (`review`), a lesson of a
//! skill tapped (`offer`, `chosen`) and the first lesson (`first`). An `Offer` is what the home
//! and the path show before anything is laid; a `Planned` is a lesson's exercises, each with the
//! level it is laid at, for `Sheet::begun` or `Sheet::append`.
//!
//! A lesson is one skill at its level, 1 to 6 exercises by its kind's minutes
//! (`Described::lesson`), a few minutes in all; a longer sitting is the Pomodoro's lessons one
//! after another (S124: the full lesson is gone; `Picked::Full` only in old records).
//! Everything follows from the seed, the learner and the pictures, the same for the same three.
//! No streak, quota or gate: any skill of any unit may be tapped.

use crate::course::learner::Learner;
use crate::course::unit::UNITS;
use crate::kinds::lines::line::LineTier;
use crate::kinds::{Kind, Skill, SkillLevel, Turn};
use crate::maths::random::derived;
use crate::packs::pack::ItemRef;
use crate::packs::pictures::Pictures;
use crate::records::sheet::Picked;
use crate::sheets::exercise::Exercise;

/// What the scheduler reads: the pictures kept and where the learner stands.
#[derive(Debug, Clone, Copy)]
pub struct Standing<'a> {
    pub pictures: &'a Pictures,
    pub learner: &'a Learner,
}

/// A lesson offered, as the home, the path and a lesson's end name it before it is laid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub skill: Skill,
    pub level: SkillLevel,
    /// How many exercises it holds, and about how many minutes they take.
    pub exercises: u8,
    pub minutes: u8,
    /// A redo of the skill waits and is laid first in it.
    pub redo: bool,
}

/// A lesson's exercises as planned, each with the level it is laid at (`Laid::level`).
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub picked: Picked,
    pub exercises: Vec<(Exercise, SkillLevel)>,
}

/// A kind with the picture it copies, as a folder lays it before it is seeded.
type Held = (Kind, Option<ItemRef>);

/// The next lesson: among the skills of the units up to the one in hand (`Learner::unit`) whose
/// pictures are kept (`Skill::lacks`), the one with the fewest dots, then the longest unpractised
/// (never practised first), then the first in the course's order. A redo is not put first here:
/// when the skill picked holds one, it is laid first in the lesson (`chosen`) and `Offer::redo`
/// says so. None when no skill can be practised.
#[must_use]
pub fn next(standing: &Standing<'_>) -> Option<Offer> {
    let (learner, pictures) = (standing.learner, standing.pictures);
    let unit = learner.unit(pictures);
    let mut skills: Vec<(usize, Skill)> = (UNITS.iter().filter(|held| held.number <= unit))
        .flat_map(|held| held.skills.iter().copied())
        .filter(|skill| skill.lacks(pictures).is_none())
        .enumerate()
        .collect();
    skills.sort_by_key(|(at, skill)| {
        let last = learner.practised(*skill).unwrap_or(0);
        (learner.dots(*skill), last, *at)
    });
    (skills.into_iter()).find_map(|(_, skill)| offer(skill, standing))
}

/// The review on the side: the skill holding the oldest redo waiting, other than the skill `next`
/// offers; its redo alone where the skill's lesson holds one exercise (a division, a whole copy),
/// else two (`reviewed`), and about how many minutes they take. None while no such redo waits.
#[must_use]
pub fn review(standing: &Standing<'_>) -> Option<Offer> {
    let (skill, level, held) = review_held(0, standing)?;
    let (exercises, minutes) = timed(&chosen_of(0, held, level));
    Some(Offer {
        skill,
        level,
        exercises,
        minutes,
        redo: true,
    })
}

/// A lesson of `skill` as tapped on the path, at the learner's level: its exercises and about how
/// many minutes they take (each kind's `Facts::seconds`); none while its pictures are not kept.
#[must_use]
pub fn offer(skill: Skill, standing: &Standing<'_>) -> Option<Offer> {
    let level = standing.learner.level(skill);
    let (exercises, minutes) = timed(&lesson(skill, level, 0, standing)?);
    Some(Offer {
        skill,
        level,
        exercises,
        minutes,
        redo: standing.learner.redo(skill).is_some(),
    })
}

/// How many exercises `planned` holds, and about how many minutes they take.
fn timed(planned: &Planned) -> (u8, u8) {
    let exercises = u8::try_from(planned.exercises.len()).unwrap_or(u8::MAX);
    let seconds: u32 = (planned.exercises.iter())
        .map(|(exercise, _)| u32::from(exercise.kind.facts().seconds))
        .sum();
    let minutes = u8::try_from(seconds.div_ceil(60).max(1)).unwrap_or(u8::MAX);
    (exercises, minutes)
}

/// The lesson of `skill` seeded `seed` at the learner's level (`Picked::Chosen`): its oldest redo
/// waiting, the same kind and picture, laid first, then the level's exercises to the lesson's
/// size (the redo in place of the one laid with its picture, else of the last); none while its
/// pictures are not kept.
#[must_use]
pub fn chosen(skill: Skill, seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    let level = standing.learner.level(skill);
    let mut held = laid(skill, level, seed, standing);
    if held.is_empty() {
        return None;
    }
    if let Some(redo) = standing.learner.redo(skill) {
        let same = (held.iter())
            .position(|(_, picture)| picture.is_some() && *picture == redo.picture)
            .unwrap_or(held.len() - 1);
        held.remove(same);
        held.insert(0, (redo.kind, redo.picture.clone()));
    }
    Some(chosen_of(seed, held, level))
}

/// The lesson of `skill` at `level` whatever the learner's (`Picked::Chosen`): what a scenario's
/// `lesson` step and the test chore start. Its folder's `Described::lesson` at the skill's next
/// turn, exercise `i` seeded `derived(seed, i)`; none while its pictures are not kept.
#[must_use]
pub fn lesson(
    skill: Skill,
    level: SkillLevel,
    seed: u64,
    standing: &Standing<'_>,
) -> Option<Planned> {
    let held = laid(skill, level, seed, standing);
    (!held.is_empty()).then(|| chosen_of(seed, held, level))
}

/// The review's lesson seeded `seed` (`Picked::Chosen`): the redo `review` offers, the same kind
/// and picture, then, where the skill's lesson holds more than one exercise, one more of its kind
/// at the skill's level (its level's first of the redo's topic and another picture, else its
/// first; none where the skill lays nothing now). None while `review` offers none.
#[must_use]
pub fn reviewed(seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    let (_, level, held) = review_held(seed, standing)?;
    Some(chosen_of(seed, held, level))
}

/// The review's skill, its level and its exercises seeded `seed` (`reviewed`).
fn review_held(seed: u64, standing: &Standing<'_>) -> Option<(Skill, SkillLevel, Vec<Held>)> {
    let offered = next(standing).map(|offer| offer.skill);
    let learner = standing.learner;
    let (skill, redo) = learner.redos().find(|(skill, _)| Some(*skill) != offered)?;
    let level = learner.level(skill);
    let topic = redo.kind.facts().key;
    let held = laid(skill, level, seed, standing);
    let other = |picture: &Option<ItemRef>| picture.is_none() || *picture != redo.picture;
    let more = (held.iter())
        .find(|(kind, picture)| kind.facts().key == topic && other(picture))
        .or_else(|| held.first())
        .filter(|_| held.len() > 1);
    let exercises = std::iter::once((redo.kind, redo.picture.clone())).chain(more.cloned());
    Some((skill, level, exercises.collect()))
}

/// The first lesson seeded `seed` (`Picked::First`), about two minutes: a free scribble, a
/// straight line, an arc; exercise `i` seeded `derived(seed, i)`.
#[must_use]
pub fn first(seed: u64) -> Planned {
    let kinds = [
        Kind::Free,
        Kind::Line(LineTier::Straight),
        Kind::Line(LineTier::Arc),
    ];
    Planned {
        picked: Picked::First,
        exercises: (seeded(seed, kinds.into_iter().map(|kind| (kind, None))).into_iter())
            .map(|exercise| (exercise, SkillLevel::First))
            .collect(),
    }
}

/// What `skill`'s folder lays at `level` at its next turn seeded `seed`, its pictures picked
/// others than those shown before; none while its pictures are not kept.
fn laid(skill: Skill, level: SkillLevel, seed: u64, standing: &Standing<'_>) -> Vec<Held> {
    if skill.lacks(standing.pictures).is_some() {
        return Vec::new();
    }
    let learner = standing.learner;
    let recent = learner.recent();
    let turn = Turn {
        turn: learner.lessons_of(skill) as usize,
        seed,
        pictures: standing.pictures,
        used: learner.used(),
        recent: &recent,
    };
    skill.described().lesson(level, &turn)
}

/// `held` as a lesson chosen, seeded `seed`, each exercise at `level`.
fn chosen_of(seed: u64, held: Vec<Held>, level: SkillLevel) -> Planned {
    Planned {
        picked: Picked::Chosen,
        exercises: (seeded(seed, held.into_iter()).into_iter())
            .map(|exercise| (exercise, level))
            .collect(),
    }
}

/// `kinds` (with their pictures) as exercises, exercise `i` seeded `derived(seed, i)`.
fn seeded(seed: u64, kinds: impl Iterator<Item = Held>) -> Vec<Exercise> {
    kinds
        .enumerate()
        .map(|(index, (kind, picture))| Exercise {
            kind,
            seed: derived(seed, index as u64),
            picture,
        })
        .collect()
}
