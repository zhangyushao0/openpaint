//! What a kind is and what its description tells (S109, S109b, D111, S123): `Kind` names an
//! exercise's kind with what varies its look, as stored; `Described` is what each kind's folder
//! implements, and every call here goes through `KINDS`.

use std::sync::Arc;

use super::KINDS;
use super::facts::Facts;
use super::original::Original;
use super::skill::SkillLevel;
use crate::colour_areas::areas_generated::Arrangement;
use crate::demos::demo::Demo;
use crate::geometry::ellipse::Guide;
use crate::geometry::perspective::{BoxView, Horizon};
use crate::kinds::colours::colour_match::Tone;
use crate::kinds::combined_lines::combined::Combination;
use crate::kinds::estimates::flat::Estimate;
use crate::kinds::key_shifts::key_shift::Tendency;
use crate::kinds::lines::line::LineTier;
use crate::kinds::objects::object_design::Carve;
use crate::kinds::points_in_space::space::Trick;
use crate::kinds::rotations::rotation::Rotation;
use crate::kinds::scales::scale::ScaleTone;
use crate::kinds::solids::solid::Solid;
use crate::marks::tile::TileMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::packs::pictures::Pictures;
use crate::sheets::exercise::Exercise;

/// The kind of an exercise, with what varies its look; each variant's folder in `kinds/`. Only
/// its folder matches on its variants (`cargo repo-check`): everyone else asks its description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A single line to copy (`lines/`).
    Line(LineTier),
    /// A single colour to match (`colours/`).
    Colour(Tone),
    /// Two to four lines to copy as one shape (`combined_lines/`).
    Combined(Combination),
    /// One object's picture of a tier, copied by its outline as a line drawing (`shapes/`).
    Shape(Tier),
    /// A generated colour combination (S82, 1.5.0 to 1.18.1): kept in its records, its picture
    /// made again from its seed; never planned since S101 (`generated_combinations/`).
    Areas(Arrangement),
    /// A colour combination (V大's 临摹色彩组合, S101): a crop of a real picture of a tier (its
    /// areas' count, `crop::tier_of`), its few colour areas copied by eye (`combinations/`).
    Crop(Tier),
    /// A row of cells (its count, the ends among them) between a given dark and light, filled in
    /// even steps of value (`scales/`).
    Scale(ScaleTone, u8),
    /// A long colour copy of a picture (V大's 色彩临摹): a project beside the lessons, never one
    /// of their exercises (`study/`).
    Study,
    /// A box in perspective given by its front corner and three edges, to complete (V大's
    /// 透视盒子, `boxes/`).
    Box(BoxView),
    /// A page with a horizon, a few boxes drawn freely on it (`free_boxes/`).
    FreeBoxes(Horizon),
    /// An ellipse drawn to fit a guide (Drawabox's tables, V大's cylinder in a box, `ellipses/`).
    Ellipse(Guide),
    /// A point, a proportion or an angle estimated on the flat (V大's 二维抽象网格, `estimates/`).
    Flat(Estimate),
    /// A point in space fixed on a box by one of V大's five tricks (点定位, `points_in_space/`).
    Space(Trick),
    /// A cylinder, a cone or a sphere found in its box (V大's week 3, S93, `solids/`).
    Solid(Solid),
    /// An object made from a box by one or two changes, simplified back into it (V大's weeks 3
    /// and 4, 几何体 + 点定位 概括单体, S94, `objects/`).
    Object(Carve),
    /// A composition's picture of a tier divided by its most obvious lines in three levels
    /// (V大's 画面分割, S95, `divisions/`).
    Division(Tier),
    /// A composition's picture of a tier divided, then copied whole as a line drawing (or in
    /// colour blocks, S96) over its division dimmed (V大's 整图复制, S95, `whole_copies/`).
    WholeCopy(Tier),
    /// A composition's picture of a tier painted as its few big masses in three greys, grouped by
    /// value (V大's 黑白灰 and colour blocks by attribute, S96, `value_masses/`).
    ValueMasses(Tier),
    /// A crop painted again shifted toward a tendency the exercise gives (V大's 变调, S107,
    /// `key_shifts/`).
    KeyShift(Tendency),
    /// A free scribble in one empty frame: the first lesson's first exercise, nothing given,
    /// nothing compared, no result (S122, `free/`).
    Free,
    /// A composition's picture of a tier analysed over it: its strongest place ringed, by what
    /// contrast it lives, the second and the weak, how strength is spread (V大's 抽象对比分析, S112,
    /// `abstract_analysis/`).
    Analysis(Tier),
    /// One of unit 4's objects shown at one angle, drawn at another: turned about its upright,
    /// seen from another height (V大's 结构翻转, S113, `rotations/`).
    Rotation(Rotation),
}

impl Kind {
    /// Its description, the folder's that names its variant.
    pub(crate) fn described(self) -> &'static dyn Described {
        (KINDS.iter().copied())
            .find(|kind| kind.variant(self).is_some())
            .unwrap_or(KINDS[0])
    }

    /// The kind and its variant as stored: each kind's code is its place in `KINDS` (1 to 22), its
    /// variant its description's (`Described::variant`). A code names one generator for good; a
    /// changed generator is a new code.
    #[must_use]
    pub fn code(self) -> (u8, u8) {
        let described = self.described();
        (described.code(), described.variant(self).unwrap_or(0))
    }

    #[must_use]
    pub fn of_code(kind: u8, variant: u8) -> Option<Self> {
        let described = KINDS.iter().find(|described| described.code() == kind)?;
        described.of_variant(variant)
    }

    /// What the app and the UI read of its topic: its key, its tools, what the eyedropper reads
    /// and the rest (`Facts`).
    #[must_use]
    pub fn facts(self) -> &'static Facts {
        self.described().topic(self)
    }

    /// Its How (S92), for an exercise of `cells` cells in hand (a value scale's row, S97): its
    /// topic's; none for a kind without one.
    #[must_use]
    pub fn how(self, cells: usize) -> Option<Demo> {
        self.described().how(self, cells)
    }
}

/// A lesson's place among its skill's (from 0), its seed, the pictures kept, those every earlier
/// lesson showed (what it picks others than) and those the last `RECENT` lessons showed (what a
/// composition is picked others than).
pub(crate) struct Turn<'a> {
    pub(crate) turn: usize,
    pub(crate) seed: u64,
    pub(crate) pictures: &'a Pictures,
    pub(crate) used: &'a [ItemRef],
    pub(crate) recent: &'a [ItemRef],
}

/// A kind's description (S109, S109b, D111, S123): everything about a kind lives in its folder,
/// told in two halves of one description. This half is the folder itself, one line in `KINDS`,
/// asked without an exercise in hand: its stored codes, its topics' facts as data (`Facts`: key,
/// tools, picks…), its How, its lesson at each level, its tile. Its generator (`generate`) makes
/// the other half, the exercise's original: a value of the folder's own type behind `Original`
/// (`original.rs`), which says what it gives, how its copy is compared and read and its answer.
///
/// Why two halves and not one trait: what a kind is asked splits by what is in hand. A skill's
/// tile, a lesson to lay or a code to read have no exercise; what an exercise prints and how its
/// copy is compared need its generated task, typed. One trait would give either every per-exercise
/// method an untyped value each kind downcasts, or every kind-wide one an exercise made up for it.
/// The two meet in `generate`, and nothing outside a folder matches on which kind it is: the flow
/// and the screens read `Facts` and call the original's methods.
///
/// A new kind is a folder holding a type for this (a unit struct) and a type for its original,
/// its variant in `Kind`, and its line in `KINDS` (with its words and its place in a unit).
pub(crate) trait Described: Sync {
    /// Its stored code, its place in `KINDS` from 1.
    fn code(&self) -> u8;
    /// Its topics' facts: most kinds one, an estimate's three, the tricks' five, the solids'
    /// three; none for a kind kept only for old records, which goes by another's.
    fn facts(&self) -> &'static [Facts];
    /// Its variant's stored code; none for another kind.
    fn variant(&self, kind: Kind) -> Option<u8>;
    /// Its kind of the stored variant `variant`; none for a variant it never stored.
    fn of_variant(&self, variant: u8) -> Option<Kind>;
    /// `kind`'s topic's facts (one of its own): its first by default.
    fn topic(&self, _kind: Kind) -> &'static Facts {
        &self.facts()[0]
    }
    /// `exercise`'s original, the same for the same kind and seed (and picture).
    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original>;
    /// `kind`'s How (S92) for an exercise of `cells` cells in hand (a value scale's row, S97);
    /// none for a kind without one.
    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        None
    }
    /// The key it goes by as a skill (`Skill::key`): its one topic's, or its own where it holds
    /// several topics.
    fn key(&self) -> &'static str {
        self.facts().first().map_or("", |facts| facts.key)
    }
    /// How many of its last judged exercises at its level its next dot is earned from
    /// (`Skill::window`): 6, or 3 in a folder whose lesson holds one exercise.
    fn window(&self) -> usize {
        6
    }
    /// A lesson of it at `level`, the `turn`th of its skill (S122): its kinds and their pictures,
    /// 1 to 6 exercises by its minutes (6 lines, 3 colours, 1 whole copy), about four to five
    /// minutes; none for a kind that is no skill, or where it finds no picture to practise on.
    /// The first level is its plainest variants, the second harder ones or its topics mixed, the
    /// third its hardest, mixed, at the lesson's full count; a folder of several topics spreads
    /// them over the levels in the course's order. Exercise `i` is seeded `derived(seed, i)`
    /// (`schedule::lesson`).
    ///
    /// The First-prefix rule: at `SkillLevel::First` a folder of one topic lays a prefix of what
    /// it laid at the same turn before S122 (the same kinds in the same order, its pictures
    /// picked alike), so a sheet started before and after S122 holds the same first exercises
    /// (`tests/course.rs` pins them).
    fn lesson(&self, _level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        Vec::new()
    }
    /// Its tile on the path (`Skill::glyph`, S91, `marks/tile.rs`); none for a kind that is no
    /// tile.
    fn glyph(&self) -> Vec<TileMark> {
        Vec::new()
    }
}

/// `item`'s place in `all`, as a stored variant (0 where it is not there).
pub(crate) fn place<T: PartialEq>(all: &[T], item: &T) -> u8 {
    all.iter()
        .position(|held| held == item)
        .map_or(0, |at| u8::try_from(at).unwrap_or(0))
}
