//! The lessons offered and laid (S122), from where the learner stands and the pictures kept:
//! the next lesson (`next`), the review on the side when a redo waits (`review`), a lesson of a
//! skill tapped (`offer`, `chosen`), a unit's full lesson (`full_offer`, `full`) and the first
//! lesson (`first`). An `Offer` is what the home and the path show before anything is laid; a
//! `Planned` is a lesson's exercises, each with the level it is laid at, for `Sheet::begun` or
//! `Sheet::append`.
//!
//! A lesson is one skill at its level, 1 to 6 exercises by its kind's minutes
//! (`Described::lesson`), a few minutes in all; a full lesson is about 30 minutes (its offer
//! says how many its kinds take): a warm-up of three from the skills longest unpractised, then
//! each skill of the unit. Everything follows from the seed, the learner and the pictures, the
//! same for the same three. No streak, quota or gate: any skill of any unit may be tapped.

use crate::course::learner::Learner;
use crate::course::unit::{UNITS, Unit};
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

/// How many exercises a full lesson's warm-up holds, one of a skill each.
const WARM_UP: usize = 3;

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
/// offers; two exercises. None while no such redo waits.
#[must_use]
pub fn review(standing: &Standing<'_>) -> Option<Offer> {
    let offered = next(standing).map(|offer| offer.skill);
    let learner = standing.learner;
    let (skill, _) = learner.redos().find(|(skill, _)| Some(*skill) != offered)?;
    Some(Offer {
        skill,
        level: learner.level(skill),
        exercises: 2,
        minutes: 2,
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

/// The full lesson of `unit` as the path offers it before it is laid: its exercises and about
/// how many minutes they take (each kind's `Facts::seconds`), its skill and level the first of
/// the unit's skills that can be practised; none where `full` lays none.
#[must_use]
pub fn full_offer(unit: u8, standing: &Standing<'_>) -> Option<Offer> {
    let (exercises, minutes) = timed(&full(unit, 0, standing)?);
    let skill = (Unit::of(unit)?.skills.iter().copied())
        .find(|skill| skill.lacks(standing.pictures).is_none())?;
    Some(Offer {
        skill,
        level: standing.learner.level(skill),
        exercises,
        minutes,
        redo: false,
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
/// and picture, then one more of its kind at the skill's level (its level's first of the redo's
/// family and another picture, else its first; none where the skill lays nothing now). None while
/// `review` offers none.
#[must_use]
pub fn reviewed(seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    let offer = review(standing)?;
    let redo = standing.learner.redo(offer.skill)?;
    let family = redo.kind.family();
    let held = laid(offer.skill, offer.level, seed, standing);
    let other = |picture: &Option<ItemRef>| picture.is_none() || *picture != redo.picture;
    let more = (held.iter())
        .find(|(kind, picture)| kind.family() == family && other(picture))
        .or_else(|| held.first());
    let exercises = std::iter::once((redo.kind, redo.picture.clone())).chain(more.cloned());
    Some(chosen_of(seed, exercises.collect(), offer.level))
}

/// The full lesson of `unit` seeded `seed` (`Picked::Full`, its `stage` the unit): a warm-up of
/// one exercise from each of the three skills longest unpractised (those of the units before it
/// first, never practised the longest; the unit's own where fewer are there), then each skill of
/// the unit, each at the learner's level (about 30 minutes); none for a unit not built or without
/// anything to practise. No picture comes twice in it while others are left.
#[must_use]
pub fn full(unit: u8, seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    let unit = Unit::of(unit).filter(|unit| unit.built())?;
    let (learner, pictures) = (standing.learner, standing.pictures);
    let kept = |skill: &Skill| skill.lacks(pictures).is_none();
    let mut warm: Vec<(usize, Skill)> = (UNITS.iter().filter(|held| held.number <= unit.number))
        .flat_map(|held| held.skills.iter().copied())
        .filter(kept)
        .enumerate()
        .collect();
    warm.sort_by_key(|(at, skill)| {
        let own = unit.skills.contains(skill);
        (own, learner.practised(*skill).unwrap_or(0), *at)
    });
    let mut laying = Laying::of(seed, standing);
    let mut warm_up = 0;
    for (_, skill) in warm {
        if warm_up == WARM_UP {
            break;
        }
        let first = laying.part(skill).into_iter().take(1);
        warm_up += laying.lay(skill, first);
    }
    for skill in unit.skills.iter().copied().filter(kept) {
        let held = laying.part(skill);
        laying.lay(skill, held);
    }
    let (held, levels): (Vec<Held>, Vec<SkillLevel>) = laying.exercises.into_iter().unzip();
    (!held.is_empty()).then(|| Planned {
        picked: Picked::Full {
            stage: unit.number,
            warm_up: u8::try_from(warm_up).unwrap_or(u8::MAX),
        },
        exercises: seeded(seed, held.into_iter())
            .into_iter()
            .zip(levels)
            .collect(),
    })
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

/// A full lesson as it is laid, part by part: its exercises with their levels, the pictures shown
/// before and in it so far.
struct Laying<'a> {
    seed: u64,
    standing: &'a Standing<'a>,
    exercises: Vec<(Held, SkillLevel)>,
    used: Vec<ItemRef>,
    recent: Vec<ItemRef>,
}

impl<'a> Laying<'a> {
    fn of(seed: u64, standing: &'a Standing<'a>) -> Self {
        Self {
            seed,
            standing,
            exercises: Vec::new(),
            used: standing.learner.used().to_vec(),
            recent: standing.learner.recent(),
        }
    }

    /// `skill`'s lesson at the learner's level, seeded by its place in the full lesson, its
    /// pictures others than those shown so far.
    fn part(&self, skill: Skill) -> Vec<Held> {
        let learner = self.standing.learner;
        let turn = Turn {
            turn: learner.lessons_of(skill) as usize,
            seed: derived(self.seed, u64::MAX - self.exercises.len() as u64),
            pictures: self.standing.pictures,
            used: &self.used,
            recent: &self.recent,
        };
        skill.described().lesson(learner.level(skill), &turn)
    }

    /// `held` of `skill` laid at its level: how many.
    fn lay(&mut self, skill: Skill, held: impl IntoIterator<Item = Held>) -> usize {
        let level = self.standing.learner.level(skill);
        let before = self.exercises.len();
        for (kind, picture) in held {
            self.used.extend(picture.clone());
            self.recent.extend(picture.clone());
            self.exercises.push(((kind, picture), level));
        }
        self.exercises.len() - before
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
