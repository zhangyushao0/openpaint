//! The path as one UI frame sees it (S122; `log/path.rs` makes it again as the log changes): the
//! course's eight units with their skills' dots, the unit in hand, the next lesson and the review
//! on the side, whether the first lesson is due, the day's lesson left unfinished; and beside
//! them the packs kept, the long colour copies, the pictures a copy can be begun of and the
//! practice book. The home's column shows the next lesson
//! (its skill, level, exercises and minutes; Start; a unit's start re-test in its place while due,
//! S147), the review while a redo waits (Review), a unit's end re-test while due (Re-test), All
//! units and Practice book; the path page the units, the next lesson and the review. And how a
//! lesson is started from them (`Start`).

use std::sync::Arc;

use openpaint_app::ProjectId;
use openpaint_practice::{ItemRef, Needs, Offer, Retest, Skill, SkillLevel, Tier};

use crate::book::BookProjection;
use crate::copies::{CopyShown, PictureShown};
use crate::lesson::LessonRow;
use crate::pictures::PackShown;

/// What a lesson is started as, from the home, the path or a lesson's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// The next lesson offered (`schedule::next`); the first lesson while nothing was practised.
    Next,
    /// The review on the side (`schedule::review`).
    Review,
    /// A lesson of the skill tapped on the path, at the learner's level.
    Skill(Skill),
    /// A lesson of a skill at a level given, whatever the learner's: a scenario's `lesson` step
    /// and the test chore.
    At(Skill, SkillLevel),
    /// The first lesson, whatever was practised.
    First,
    /// Unit `n`'s re-test at its start or (true) its end (S147).
    Retest(u8, bool),
}

/// The home's practice column, the path page and the practice book.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PathProjection {
    /// The eight units in order.
    pub units: Vec<UnitShown>,
    /// The unit in hand: the one the path page opens on.
    pub unit: u8,
    /// The next lesson (`Start::Next`); none while the first lesson is due or nothing can be
    /// practised.
    pub next: Option<Offer>,
    /// The review on the side (`Start::Review`), while a redo of another skill than the next
    /// lesson's waits.
    pub review: Option<Offer>,
    /// Nothing was practised yet: Start (`Start::Next`) starts the first lesson.
    pub first: bool,
    /// The unit in hand's re-test at its start, while due (S147): Start starts it in the next
    /// lesson's place.
    pub retest: Option<Retest>,
    /// A unit's re-test at its end, while due: offered beside the next lesson.
    pub retest_end: Option<Retest>,
    /// The day's lesson left unfinished: Continue opens its sheet on it.
    pub open: Option<OpenLesson>,
    /// The packs kept, the official ones first.
    pub packs: Vec<PackShown>,
    /// The long colour copies, newest first.
    pub copies: Vec<CopyShown>,
    /// The pictures a colour copy can be begun of: the official packs' by tier, then the user's
    /// own.
    pub pictures: Arc<[PictureShown]>,
    /// The practice book.
    pub book: BookProjection,
}

/// The day's lesson left unfinished, as the home names it: "Lines 3/6".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenLesson {
    /// Its sheet, which Continue opens.
    pub sheet: ProjectId,
    /// The lesson as its row's label names it: its skill, the first or a re-test.
    pub row: LessonRow,
    /// The exercise in hand's place among the lesson's, from 1, and how many there are.
    pub place: (usize, usize),
}

/// A unit on the path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitShown {
    /// From 1.
    pub number: u8,
    /// Whether it is open (`Learner::open`); any unit may still be entered.
    pub open: bool,
    /// Its skills in the course's order; none while its kinds are not built.
    pub skills: Vec<SkillShown>,
}

/// A skill's tile on the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillShown {
    pub skill: Skill,
    /// Its dots, 0 to 3.
    pub dots: u8,
    /// What the packs kept lack for a lesson of it: it shows greyed with why, and starts none.
    pub lacks: Option<Needs>,
    /// A redo of it waits.
    pub redo: bool,
}

impl PathProjection {
    /// The pictures of `tier` of the official packs, or the user's own (`None`).
    pub fn pictures_of(&self, tier: Option<Tier>) -> impl Iterator<Item = &PictureShown> {
        (self.pictures.iter()).filter(move |picture| match tier {
            Some(tier) => !picture.personal && picture.tier == tier,
            None => picture.personal,
        })
    }

    /// The picture `item` names, while a pack kept holds it.
    #[must_use]
    pub fn picture(&self, item: &ItemRef) -> Option<&PictureShown> {
        self.pictures.iter().find(|picture| picture.item == *item)
    }
}
