//! The learner (S122): what the records say of someone's practice. Derived from the sheets every
//! time, never stored, so a record removed (a test lesson) takes its part with it. Every lesson of
//! every sheet but a long colour copy's is taken in the order of (sheet id, lesson index)
//! (`records/lessons.rs`).
//!
//! The dot rule, which the path, a lesson's end and the scheduler all read from here:
//! - A skill holds 0 to `DOTS` dots. Its lessons are laid at `SkillLevel::of_code(dots)`: the first
//!   level until its first dot, the third from two dots on.
//! - An exercise counts once it has a result (`Laid::result`: the app's verdict, or the user's own
//!   where the eye judges) and only if it was laid at the skill's level then (`Laid::level`).
//! - The window is the skill's last `Skill::window()` such exercises (6; 3 where a lesson holds
//!   one). When it is full, at least two thirds of it are `Outcome::Clean` and none is
//!   `Outcome::Redo`, the skill gains a dot and the window empties. Dots never fall.
//! - Records from before S122 hold level 0 and few results: nothing is seeded from them (the
//!   author, 2026-11-11), dots come from what is judged from now on.
//!
//! A redo: an exercise that ended `Outcome::Redo`. It waits, oldest first, until its kind (its
//! variant too) and its picture are laid again in a later lesson, which answers it whatever that
//! one ends with.
//!
//! A unit is open when it is the first, or it holds a skill, the unit before it is open and every
//! skill of that one whose pictures are kept has its first dot; a unit without a skill never is.
//! Any unit may still be entered.

use crate::course::unit::{UNITS, Unit};
use crate::kinds::{Kind, Skill, SkillLevel};
use crate::marks::compared::Outcome;
use crate::packs::pack::ItemRef;
use crate::packs::pictures::Pictures;
use crate::records::lessons::lessons;
use crate::records::record::RECENT;
use crate::records::sheet::{Lesson, Sheet};

/// The dots a skill can hold: one a level.
pub const DOTS: u8 = 3;

/// What the records say of someone's practice.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Learner {
    /// The lessons taken.
    lessons: u32,
    /// Each skill practised, in the order first met.
    skills: Vec<Practised>,
    /// The redos waiting, oldest first.
    redos: Vec<(Skill, Redo)>,
    /// Every picture a lesson showed, each once, and the last `RECENT` lessons' pictures.
    used: Vec<ItemRef>,
    recent: Vec<Vec<ItemRef>>,
}

/// A skill as practised so far.
#[derive(Debug, Clone, PartialEq)]
struct Practised {
    skill: Skill,
    dots: u8,
    /// Its last judged exercises at its level, oldest first.
    window: Vec<Outcome>,
    /// The lessons that held it, and the last of them (its number among all lessons, from 1).
    lessons: u32,
    last: u32,
}

/// An exercise to do again: its kind and the picture it copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redo {
    pub kind: Kind,
    pub picture: Option<ItemRef>,
}

impl Practised {
    /// An exercise laid at `level` ended `outcome`: the dot rule, with a window of `window`.
    fn judged(&mut self, level: u8, outcome: Outcome, window: usize) {
        if SkillLevel::of_code(level) != SkillLevel::of_code(self.dots) {
            return;
        }
        self.window.push(outcome);
        if self.window.len() > window {
            self.window.remove(0);
        }
        let clean = (self.window.iter())
            .filter(|kept| **kept == Outcome::Clean)
            .count();
        let full = self.window.len() == window;
        let redo = self.window.contains(&Outcome::Redo);
        if self.dots < DOTS && full && 3 * clean >= 2 * window && !redo {
            self.dots += 1;
            self.window.clear();
        }
    }
}

impl Learner {
    /// The learner `sheets` tell of.
    #[must_use]
    pub fn of<'a>(sheets: impl IntoIterator<Item = &'a Sheet>) -> Self {
        let mut learner = Self::default();
        for on in lessons(sheets).iter().filter(|on| !on.is_copy()) {
            learner.take(on.lesson);
        }
        learner
    }

    /// The learner after `lesson` (a day's or a test's, never a copy's) as well: what a lesson's
    /// end sets its dots beside (`before.with(lesson)` against `before`).
    #[must_use]
    pub fn with(&self, lesson: &Lesson) -> Self {
        let mut learner = self.clone();
        learner.take(lesson);
        learner
    }

    /// Takes `lesson` in: its skills practised, its results by the dot rule, the redos it answers
    /// and those it leaves, its pictures (what the seals' tally folds too, `records/seals.rs`).
    pub(crate) fn take(&mut self, lesson: &Lesson) {
        self.lessons += 1;
        let at = self.lessons;
        let (mut met, mut redone, mut pictures) = (Vec::new(), Vec::new(), Vec::new());
        for laid in &lesson.exercises {
            let exercise = &laid.exercise;
            pictures.extend(exercise.picture.clone());
            let Some(skill) = Skill::of(exercise.kind) else {
                continue;
            };
            let again = Redo {
                kind: exercise.kind,
                picture: exercise.picture.clone(),
            };
            let waiting = |(of, redo): &(Skill, Redo)| *of == skill && *redo == again;
            if let Some(answered) = self.redos.iter().position(waiting) {
                self.redos.remove(answered);
            }
            let practised = self.practised_mut(skill);
            if !met.contains(&skill) {
                met.push(skill);
                practised.lessons += 1;
                practised.last = at;
            }
            if let Some((outcome, _)) = laid.result {
                practised.judged(laid.level, outcome, skill.window());
                if outcome == Outcome::Redo {
                    redone.push((skill, again));
                }
            }
        }
        self.redos.extend(redone);
        for item in &pictures {
            if !self.used.contains(item) {
                self.used.push(item.clone());
            }
        }
        self.recent.push(pictures);
        if self.recent.len() > RECENT {
            self.recent.remove(0);
        }
    }

    fn practised_of(&self, skill: Skill) -> Option<&Practised> {
        self.skills.iter().find(|kept| kept.skill == skill)
    }

    fn practised_mut(&mut self, skill: Skill) -> &mut Practised {
        let at = (self.skills.iter()).position(|kept| kept.skill == skill);
        let at = at.unwrap_or_else(|| {
            self.skills.push(Practised {
                skill,
                dots: 0,
                window: Vec::new(),
                lessons: 0,
                last: 0,
            });
            self.skills.len() - 1
        });
        &mut self.skills[at]
    }

    /// `skill`'s dots, 0 to `DOTS`.
    #[must_use]
    pub fn dots(&self, skill: Skill) -> u8 {
        self.practised_of(skill).map_or(0, |kept| kept.dots)
    }

    /// The level `skill`'s next lesson is laid at.
    #[must_use]
    pub fn level(&self, skill: Skill) -> SkillLevel {
        SkillLevel::of_code(self.dots(skill))
    }

    /// The lessons taken, a long colour copy's left out: the next one's number less one.
    #[must_use]
    pub const fn lessons(&self) -> u32 {
        self.lessons
    }

    /// Whether nothing was practised yet: the first lesson is due.
    #[must_use]
    pub const fn fresh(&self) -> bool {
        self.lessons == 0
    }

    /// The lessons that held `skill`: its next lesson's turn (`Described::lesson`).
    #[must_use]
    pub fn lessons_of(&self, skill: Skill) -> u32 {
        self.practised_of(skill).map_or(0, |kept| kept.lessons)
    }

    /// The last lesson that held `skill`, its number among all lessons from 1; none while it was
    /// never practised (which is longer unpractised than any number).
    #[must_use]
    pub fn practised(&self, skill: Skill) -> Option<u32> {
        self.practised_of(skill).map(|kept| kept.last)
    }

    /// `skill`'s oldest redo waiting.
    #[must_use]
    pub fn redo(&self, skill: Skill) -> Option<&Redo> {
        self.redos()
            .find_map(|(of, redo)| (of == skill).then_some(redo))
    }

    /// Every redo waiting with its skill, oldest first.
    pub fn redos(&self) -> impl Iterator<Item = (Skill, &Redo)> {
        self.redos.iter().map(|(skill, redo)| (*skill, redo))
    }

    /// Every picture a lesson showed, each once: what the next lesson picks others than.
    #[must_use]
    pub fn used(&self) -> &[ItemRef] {
        &self.used
    }

    /// The pictures the last `RECENT` lessons showed: what a composition is picked others than.
    #[must_use]
    pub fn recent(&self) -> Vec<ItemRef> {
        self.recent.iter().flatten().cloned().collect()
    }

    /// Whether `unit` is open with the `pictures` kept: the first always; another when it holds a
    /// skill, the unit before it is open and each skill of that one that can be practised has its
    /// first dot. A unit without a skill (6 to 8) is never open.
    #[must_use]
    pub fn open(&self, unit: u8, pictures: &Pictures) -> bool {
        if !Unit::of(unit).is_some_and(Unit::built) {
            return false;
        }
        if unit == 1 {
            return true;
        }
        let Some(before) = unit.checked_sub(1).and_then(Unit::of) else {
            return false;
        };
        let dotted = (before.skills.iter())
            .filter(|skill| skill.lacks(pictures).is_none())
            .all(|skill| self.dots(*skill) > 0);
        dotted && self.open(before.number, pictures)
    }

    /// The unit in hand with the `pictures` kept: the first built one holding a skill that can be
    /// practised and is short of its first dot; the last built one once none is.
    #[must_use]
    pub fn unit(&self, pictures: &Pictures) -> u8 {
        let short = |skill: &Skill| skill.lacks(pictures).is_none() && self.dots(*skill) == 0;
        let built = || UNITS.iter().filter(|unit| unit.built());
        let in_hand = built().find(|unit| unit.skills.iter().any(short));
        in_hand
            .or(built().next_back())
            .map_or(1, |unit| unit.number)
    }
}
