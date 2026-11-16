//! Progress from the learner's own judgements (S147): per skill, its ability on each day it was
//! practised, fitted over every result, every kept measure and every blind pair as Whole-History
//! Rating fits a player (Coulom 2008; `fit.rs`, its linear algebra `band.rs`), from what the records
//! hold (`observe.rs`); and which two earlier copies a lesson's end asks to compare (`ask.rs`).
//! Derived whenever asked (the book, a lesson's end), never stored, as the learner is.
//!
//! What is shown of it: a curve a skill, its ability less its first day's with one standard
//! deviation of that as its band, no number (`Curve`); and, at a lesson's end, whether the model is
//! sure the skill fell over about four weeks (`Change`), which offers an extra drill.

pub(crate) mod band;
pub(crate) mod fit;
pub(crate) mod observe;
pub(crate) mod terms;

mod ask;

use crate::course::unit::UNITS;
use crate::kinds::{Skill, SkillLevel};
use crate::records::pairs::{BlindPair, CopyRef};
use crate::records::sheet::Sheet;

pub(crate) use fit::Better;
pub(crate) use observe::scaled;

pub use ask::{PairAsked, Within, ask, retest_pairs};

/// How far apart, in days, the two days of a change are: the one nearest `CHANGE_DAYS` before the
/// last, at least `CHANGE_LEAST`.
const CHANGE_DAYS: i64 = 28;
const CHANGE_LEAST: i64 = 21;
/// A change is sure when it passes `CHANGE_RISE` logits and `CHANGE_SURE` of its standard
/// deviation (90 % one-sided).
const CHANGE_RISE: f64 = 0.5;
const CHANGE_SURE: f64 = 1.28;

/// Every skill practised, its curve, in the course's order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    pub curves: Vec<Curve>,
}

/// A skill's curve: its days, its ability on each less the first day's, one standard deviation of
/// that, where each level past the first began, and its re-test's two copies once compared.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub skill: Skill,
    /// Local days, ascending.
    pub days: Vec<i64>,
    pub ability: Vec<f64>,
    pub band: Vec<f64>,
    /// Each level's first day by its index.
    pub levels: Vec<(usize, SkillLevel)>,
    /// The latest re-test's pair at a unit's end answered for it: its older and newer copies with
    /// their days.
    pub retest: Option<[(CopyRef, i64); 2]>,
}

/// A sure change over about four weeks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Rose { weeks: u8 },
    Fell { weeks: u8 },
}

/// One skill's history fitted.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillFit {
    skill: Skill,
    gathered: observe::Gathered,
    fit: fit::Fit,
}

impl Progress {
    /// Every skill's curve in `sheets` and `pairs`, the days local on a clock `offset` minutes from
    /// UTC.
    #[must_use]
    pub fn of(sheets: &[Sheet], pairs: &[BlindPair], offset: i32) -> Self {
        let skills = UNITS.iter().flat_map(|unit| unit.skills.iter().copied());
        let curves = skills
            .filter_map(|skill| SkillFit::of(skill, sheets, pairs, offset))
            .map(|fitted| {
                let mut curve = fitted.curve();
                curve.retest = retested(fitted.skill, sheets, pairs, offset);
                curve
            })
            .collect();
        Self { curves }
    }

    /// `skill`'s curve, if it was practised.
    #[must_use]
    pub fn curve(&self, skill: Skill) -> Option<&Curve> {
        self.curves.iter().find(|curve| curve.skill == skill)
    }
}

impl SkillFit {
    /// `skill`'s history in `sheets` and `pairs` fitted; none while nothing of it was judged.
    #[must_use]
    pub fn of(skill: Skill, sheets: &[Sheet], pairs: &[BlindPair], offset: i32) -> Option<Self> {
        let gathered = observe::gathered(skill, sheets, pairs, offset);
        let fit = fit::Fit::of(&gathered.history)?;
        Some(Self {
            skill,
            gathered,
            fit,
        })
    }

    /// Its curve (no re-test's pair).
    #[must_use]
    pub fn curve(&self) -> Curve {
        let first = self.fit.ability.first().copied().unwrap_or(0.0);
        Curve {
            skill: self.skill,
            days: self.gathered.history.days.clone(),
            ability: self.fit.ability.iter().map(|a| a - first).collect(),
            band: self.fit.band(),
            levels: self.gathered.levels.clone(),
            retest: None,
        }
    }

    /// Its last day against the one nearest four weeks before (three at least): a rise or a fall
    /// past `CHANGE_RISE` and `CHANGE_SURE` of its standard deviation; none else.
    #[must_use]
    pub fn change(&self) -> Option<Change> {
        let days = &self.gathered.history.days;
        let last = days.len().checked_sub(1)?;
        let reach = days[last] - CHANGE_LEAST;
        let before = (0..last)
            .filter(|at| days[*at] <= reach)
            .min_by_key(|at| (days[last] - CHANGE_DAYS - days[*at]).abs())?;
        let moved = self.fit.ability[last] - self.fit.ability[before];
        let sure = CHANGE_SURE * self.fit.spread(before, last);
        let weeks = u8::try_from((days[last] - days[before] + 3) / 7).unwrap_or(u8::MAX);
        if moved > CHANGE_RISE && moved > sure {
            Some(Change::Rose { weeks })
        } else if -moved > CHANGE_RISE && -moved > sure {
            Some(Change::Fell { weeks })
        } else {
            None
        }
    }

    /// One standard deviation of its ability on day `newer` less day `older` (local days); none for
    /// a day it was not practised.
    #[must_use]
    pub fn spread(&self, older: i64, newer: i64) -> Option<f64> {
        let days = &self.gathered.history.days;
        let (a, b) = (
            days.binary_search(&older).ok()?,
            days.binary_search(&newer).ok()?,
        );
        Some(self.fit.spread(a, b))
    }

    /// The Newton steps its fit took.
    #[must_use]
    pub const fn steps(&self) -> usize {
        self.fit.steps
    }

    /// How much harder its second level is than its first, and its third than its second.
    #[must_use]
    pub const fn levels(&self) -> [f64; 2] {
        self.fit.levels
    }
}

/// `skill`'s latest pair asked at a unit's end re-test, its copies with their days.
fn retested(
    skill: Skill,
    sheets: &[Sheet],
    pairs: &[BlindPair],
    offset: i32,
) -> Option<[(CopyRef, i64); 2]> {
    let asked_at_end = |pair: &&BlindPair| {
        let (sheet, lesson) = pair.lesson;
        (sheets.iter().find(|held| held.id == sheet))
            .and_then(|held| held.lessons.get(usize::from(lesson)))
            .and_then(|lesson| lesson.retest())
            .is_some_and(|(_, end)| end)
    };
    let pair = (pairs.iter().filter(asked_at_end))
        .filter(|pair| {
            (pair.newer.find(sheets))
                .is_some_and(|(_, laid)| Skill::of(laid.exercise.kind) == Some(skill))
        })
        .max_by_key(|pair| pair.asked)?;
    let day = |copy: CopyRef| {
        let (sheet, laid) = copy.find(sheets)?;
        Some((copy, observe::copy_day(sheet, laid, offset)))
    };
    Some([day(pair.older)?, day(pair.newer)?])
}
