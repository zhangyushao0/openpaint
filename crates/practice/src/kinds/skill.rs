//! A skill and its levels (S122): what the path shows as a tile with three dots and a lesson
//! practises. A skill is a kind's folder, named by its stored code (`Described::code`), so the
//! flow and the screens hold one without knowing which kind it is: 17 of the folders are skills
//! (`course/unit.rs` lists them by unit); a long colour copy and the first lesson's free scribble
//! are none, and a generated combination (old records) counts as a combination.
//!
//! A level is how hard a skill's lesson is laid (入门, 熟练, 稳): harder variants, mixing or
//! volume, as its folder's `Described::lesson` says. It is stored with every exercise
//! (`Laid::level`), so a level's results stay its own whatever the folder lays later.

use super::kind::{Described, Kind};
use super::{KINDS, Needs};
use crate::course::unit::UNITS;
use crate::marks::tile::TileMark;
use crate::packs::pictures::Pictures;

/// A skill: a kind's folder, by its stored code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Skill(pub(crate) u8);

/// How hard a lesson of a skill is laid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillLevel {
    /// 入门.
    First,
    /// 熟练.
    Second,
    /// 稳.
    Third,
}

impl SkillLevel {
    pub const ALL: [Self; 3] = [Self::First, Self::Second, Self::Third];

    /// As an exercise stores it (`Laid::level`): 0, 1, 2.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::First => 0,
            Self::Second => 1,
            Self::Third => 2,
        }
    }

    /// The level stored as `code`; also the level a skill of `code` dots is practised at: the
    /// first until its first dot, the third from two dots on.
    #[must_use]
    pub const fn of_code(code: u8) -> Self {
        match code {
            0 => Self::First,
            1 => Self::Second,
            _ => Self::Third,
        }
    }
}

impl Skill {
    /// The skill `kind` practises; none for a long colour copy and a free scribble.
    #[must_use]
    pub fn of(kind: Kind) -> Option<Self> {
        let folder = match kind.code().0 {
            // A generated combination (1.18.1 and before) is a combination.
            GENERATED => COMBINATIONS,
            code => code,
        };
        Self::all().find(|skill| skill.0 == folder)
    }

    /// Every skill, in the course's order (`UNITS`).
    pub fn all() -> impl Iterator<Item = Self> {
        UNITS.iter().flat_map(|unit| unit.skills.iter().copied())
    }

    /// The key its words go by in the UI's tables, a scenario's `lesson` step and the test chore
    /// name it by: its one topic's key (`Facts::key`: `line`, `colour`, `box`…), or its own where
    /// it holds several topics (`estimates`, `points-in-space`, `solids`).
    #[must_use]
    pub fn key(self) -> &'static str {
        self.described().key()
    }

    /// The skill keyed `key`, if any.
    #[must_use]
    pub fn of_key(key: &str) -> Option<Self> {
        Self::all().find(|skill| skill.key() == key)
    }

    /// What its lessons need kept before one starts (a pack's pictures, compositions, crops).
    #[must_use]
    pub fn needs(self) -> Needs {
        let facts = self.described().facts();
        facts.first().map_or(Needs::Nothing, |facts| facts.needs)
    }

    /// What `pictures` lack for a lesson of it; none when one can start. A skill lacking its
    /// pictures shows greyed on the path, is never offered and is left out of what opens the
    /// next unit.
    #[must_use]
    pub fn lacks(self, pictures: &Pictures) -> Option<Needs> {
        let needs = self.needs();
        let met = match needs {
            Needs::Nothing => true,
            Needs::Pictures => !pictures.is_empty(),
            Needs::Compositions => pictures.has_compositions(),
            Needs::Masses => pictures.has_masses(),
            Needs::Crops => pictures.has_crops(),
        };
        (!met).then_some(needs)
    }

    /// How many of its last judged exercises at its level its next dot is earned from: 6, or 3
    /// where a lesson holds one exercise (`Described::window`).
    #[must_use]
    pub fn window(self) -> usize {
        self.described().window()
    }

    /// Its tile's picture on the path, in `TILE`'s units.
    ///
    /// Skeleton body: its first topic's tile. S122a: one glyph a folder.
    #[must_use]
    pub fn glyph(self) -> Vec<TileMark> {
        let described = self.described();
        let topic = described.facts().first();
        topic.map_or_else(Vec::new, |facts| described.tile(facts.family))
    }

    /// Its folder's description.
    pub(crate) fn described(self) -> &'static dyn Described {
        (KINDS.iter().copied())
            .find(|kind| kind.code() == self.0)
            .unwrap_or(KINDS[0])
    }
}

/// The folders' stored codes a generated combination goes from and to.
const GENERATED: u8 = 5;
const COMBINATIONS: u8 = 18;
