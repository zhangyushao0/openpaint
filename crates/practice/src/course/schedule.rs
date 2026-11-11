//! The lessons offered and laid (S122), from where the learner stands and the pictures kept:
//! the next lesson (`next`), the review on the side when a redo waits (`review`), a lesson of a
//! skill tapped (`offer`, `chosen`), a unit's full lesson (`full`) and the first lesson
//! (`first`). An `Offer` is what the home and the path show before anything is laid; a `Planned`
//! is a lesson's exercises, each with the level it is laid at, for `Sheet::begun` or
//! `Sheet::append`.
//!
//! A lesson is one skill at its level, 1 to 6 exercises by its kind's minutes
//! (`Described::lesson`), a few minutes in all; a full lesson is about 30 minutes: a warm-up of
//! three from the skills longest unpractised, then each skill of the unit. Everything follows
//! from the seed, the learner and the pictures, the same for the same three.
//!
//! The bodies marked so are the skeleton's: S122a writes the scheduler in their place.

use crate::course::learner::Learner;
use crate::course::unit::{UNITS, Unit};
use crate::kinds::lines::line::LineTier;
use crate::kinds::{Kind, Skill, SkillLevel, Turn};
use crate::maths::random::derived;
use crate::packs::pack::ItemRef;
use crate::packs::pictures::Pictures;
use crate::records::sheet::Picked;
use crate::sessions::plan::Exercise;
use crate::sessions::session::{Before, plan, seeded};

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

/// The next lesson: among the skills of the units up to the one in hand (`Learner::unit`) whose
/// pictures are kept (`Skill::lacks`), the one with the fewest dots, then the longest unpractised
/// (never practised first), then the first in the course's order. A redo is not put first here:
/// when the skill picked holds one, it is laid first in the lesson (`chosen`) and `Offer::redo`
/// says so. None when no skill can be practised.
///
/// S122a replaces this skeleton body: the first such skill in the course's order.
#[must_use]
pub fn next(standing: &Standing<'_>) -> Option<Offer> {
    let unit = standing.learner.unit(standing.pictures);
    (UNITS.iter().filter(|held| held.number <= unit))
        .flat_map(|held| held.skills.iter().copied())
        .find_map(|skill| offer(skill, standing))
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

/// A lesson of `skill` as tapped on the path, at the learner's level; none while its pictures are
/// not kept.
///
/// S122a replaces this skeleton body (a minute an exercise) with the kind's minutes.
#[must_use]
pub fn offer(skill: Skill, standing: &Standing<'_>) -> Option<Offer> {
    let level = standing.learner.level(skill);
    let held = lesson(skill, level, 0, standing)?.exercises.len();
    let exercises = u8::try_from(held).unwrap_or(u8::MAX);
    Some(Offer {
        skill,
        level,
        exercises,
        minutes: exercises,
        redo: standing.learner.redo(skill).is_some(),
    })
}

/// The lesson of `skill` seeded `seed` at the learner's level (`Picked::Chosen`): its oldest redo
/// waiting, the same kind and picture, laid first, then the level's exercises to the lesson's
/// size; none while its pictures are not kept.
///
/// S122a replaces this skeleton body: `lesson` at the learner's level, no redo laid.
#[must_use]
pub fn chosen(skill: Skill, seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    lesson(skill, standing.learner.level(skill), seed, standing)
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
    skill.lacks(standing.pictures).is_none().then_some(())?;
    let recent = standing.learner.recent();
    let before = before(standing, &recent);
    let turn = Turn {
        turn: standing.learner.lessons_of(skill) as usize,
        seed,
        before: &before,
    };
    let held = skill.described().lesson(level, &turn);
    (!held.is_empty()).then(|| Planned {
        picked: Picked::Chosen,
        exercises: (seeded(seed, held.into_iter(), 0).into_iter())
            .map(|exercise| (exercise, level))
            .collect(),
    })
}

/// The review's lesson seeded `seed` (`Picked::Chosen`): the redo `review` offers, the same kind
/// and picture, then one more of its kind at the skill's level. None while `review` offers none.
///
/// S122a replaces this skeleton body: the redo's kind and picture twice.
#[must_use]
pub fn reviewed(seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    let offer = review(standing)?;
    let redo = standing.learner.redo(offer.skill)?;
    let again = |turn: u64| Exercise {
        kind: redo.kind,
        seed: derived(seed, turn),
        picture: redo.picture.clone(),
    };
    Some(Planned {
        picked: Picked::Chosen,
        exercises: vec![(again(0), offer.level), (again(1), offer.level)],
    })
}

/// The full lesson of `unit` seeded `seed` (`Picked::Full`, its `stage` the unit): a warm-up of
/// three from the skills longest unpractised, then each skill of the unit at the learner's level,
/// about 30 minutes; none for a unit not built or without anything to practise.
///
/// S122a replaces this skeleton body: the unit's old stage plan without a warm-up.
#[must_use]
pub fn full(unit: u8, seed: u64, standing: &Standing<'_>) -> Option<Planned> {
    Unit::of(unit).filter(|unit| unit.built())?;
    let learner = standing.learner;
    let recent = learner.recent();
    let planned = plan(
        unit,
        learner.lessons() + 1,
        seed,
        &before(standing, &recent),
    );
    let level =
        |kind: Kind| Skill::of(kind).map_or(SkillLevel::First, |skill| learner.level(skill));
    (!planned.exercises.is_empty()).then(|| Planned {
        picked: Picked::Full {
            stage: unit,
            warm_up: u8::try_from(planned.warm_up).unwrap_or(u8::MAX),
        },
        exercises: (planned.exercises.into_iter())
            .map(|exercise| {
                let level = level(exercise.kind);
                (exercise, level)
            })
            .collect(),
    })
}

/// The first lesson seeded `seed` (`Picked::First`), about two minutes: a free scribble, a
/// straight line, an arc; exercise `i` seeded `derived(seed, i)`. Not the skeleton's: it stands.
#[must_use]
pub fn first(seed: u64) -> Planned {
    let kinds = [
        Kind::Free,
        Kind::Line(LineTier::Straight),
        Kind::Line(LineTier::Arc),
    ];
    Planned {
        picked: Picked::First,
        exercises: (seeded(seed, kinds.into_iter().map(|kind| (kind, None)), 0).into_iter())
            .map(|exercise| (exercise, SkillLevel::First))
            .collect(),
    }
}

/// What the old plans and drills read of `standing` (no kind met: no warm-up; no scale met), the
/// last lessons' pictures as `recent` holds them. S122a: goes with them.
fn before<'a>(standing: &Standing<'a>, recent: &'a [ItemRef]) -> Before<'a> {
    Before {
        met: &[],
        pictures: standing.pictures,
        used: standing.learner.used(),
        recent,
        scales: 0,
    }
}
