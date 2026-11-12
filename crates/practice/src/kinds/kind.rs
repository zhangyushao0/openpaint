//! What a kind is and what its description tells (S109, S109b, D111): `Kind` names an exercise's
//! kind with what varies its look, `Described` what each kind's folder implements: its stored
//! code, its families' facts (`facts.rs`), its generator, How, a lesson at each level, givens, how
//! its copy is compared, the picture it draws, the answer a hand would draw, its tile and glyph.
//! Every call here goes through `KINDS`.

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
use crate::kinds::scales::scale::ScaleTone;
use crate::kinds::solids::solid::Solid;
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::compared::{Compared, Comparison, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::TileMark;
use crate::maths::path::Point;
use crate::packs::pack::{ItemRef, Tier};
use crate::packs::pictures::Pictures;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use crate::sheets::sheet_picture::SheetPicture;

/// The kind of an exercise, with what varies its look; each variant's folder in `kinds/`.
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
    /// A long colour copy of a picture (V大's 色彩临摹): a project beside the sessions, never
    /// one of their exercises (`study/`).
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
}

impl Kind {
    /// Its description, the folder's that names its variant.
    pub(crate) fn described(self) -> &'static dyn Described {
        (KINDS.iter().copied())
            .find(|kind| kind.variant(self).is_some())
            .unwrap_or(KINDS[0])
    }

    /// The kind and its variant as stored: each kind's code is its place in `KINDS` (1 to 20), its
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

    #[must_use]
    pub fn family(self) -> Family {
        self.described().family(self)
    }

    /// What the app and the UI read of its family.
    #[must_use]
    pub fn facts(self) -> &'static Facts {
        self.family().facts()
    }

    /// Its How (S92), for an exercise of `cells` cells in hand (a value scale's row, S97); none
    /// for a kind without one. What the flow and the screens ask a How by (S122): the kind in
    /// hand, never its family.
    #[must_use]
    pub fn how(self, cells: usize) -> Option<Demo> {
        self.described().how_of(self.family(), cells)
    }

    /// `original`'s answer (an exercise of this kind's) as a hand would draw it, a stroke at a
    /// time (frame units); none for a kind without one.
    #[must_use]
    pub fn answer(self, original: &Original) -> Vec<Vec<Point>> {
        self.family().answer(original)
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

/// What a kind's folder tells of it (S109, S109b, D111).
pub(crate) trait Described: Sync {
    /// Its stored code, its place in `KINDS` from 1.
    fn code(&self) -> u8;
    /// Its families: most kinds one, an estimate's three, the tricks' five, the solids' three; none
    /// for a kind kept only for old records, which goes by another's family.
    fn facts(&self) -> &'static [Facts];
    /// Its variant's stored code; none for another kind.
    fn variant(&self, kind: Kind) -> Option<u8>;
    /// Its kind of the stored variant `variant`; none for a variant it never stored.
    fn of_variant(&self, variant: u8) -> Option<Kind>;
    /// `kind`'s family (one of its own).
    fn family(&self, _kind: Kind) -> Family {
        self.facts()[0].family
    }
    /// `exercise`'s original, the same for the same kind and seed.
    fn original(&self, exercise: &Exercise) -> Original;
    /// `family`'s How (S92); none for a kind without one.
    fn how(&self, _family: Family) -> Option<Demo> {
        None
    }
    /// `family`'s How for an exercise of `cells` cells in hand (a value scale's row, S97).
    fn how_of(&self, family: Family, _cells: usize) -> Option<Demo> {
        self.how(family)
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
    /// its drill laid at the same turn before S122 (the same kinds in the same order, its pictures
    /// picked alike), so a sheet started before and after S122 holds the same first exercises
    /// (`tests/course.rs` pins them).
    fn lesson(&self, _level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        Vec::new()
    }
    /// What `exercise` gives in its frames before Compare (its `original`); a pack's picture is
    /// the sheet's to lay.
    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        Vec::new()
    }
    /// `original`'s copy, drawn as `copy`, checked as geometry (S84 to S98); none for a kind not
    /// checked so.
    fn check(&self, _original: &Original, _copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        None
    }
    /// How `asked`'s copy is compared; none while there is nothing to compare (S109b). By
    /// default its drawn lines, once anything lies in its frame, checked as geometry.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let checked = self.check(asked.original, &asked.copy.drawn())?;
        Some(Compare::Checked(Comparison(checked)))
    }
    /// The picture it draws for `original` (its pack's `picture`, if any): a generated
    /// combination's, an object's, value masses' answer; none for most kinds.
    fn painted(&self, _original: &Original, _picture: Option<&SheetPicture>) -> Option<Painted> {
        None
    }
    /// `original`'s answer as a hand would draw it, a stroke at a time in frame units (a cross on a
    /// point, a stroke along each line, once round each ellipse): what `bench practice` draws a
    /// little off; none for a kind without one.
    fn answer(&self, _original: &Original) -> Vec<Vec<Point>> {
        Vec::new()
    }
    /// `family`'s tile picture (S91, `marks/tile.rs`).
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        Vec::new()
    }
    /// Its glyph as a skill on the path (`Skill::glyph`): its first topic's tile, or a picture of
    /// its own where it holds several topics.
    fn glyph(&self) -> Vec<TileMark> {
        self.facts()
            .first()
            .map_or_else(Vec::new, |facts| self.tile(facts.family))
    }
}

impl Exercise {
    /// What it gives in its frames before Compare, as its kind draws it.
    #[must_use]
    pub fn givens(&self, original: &Original) -> Vec<SheetMark> {
        self.kind.described().givens(self, original)
    }

    /// Its copy, drawn as `copy`, checked as geometry; none for a kind not checked so.
    #[must_use]
    pub fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Comparison> {
        self.kind.described().check(original, copy).map(Comparison)
    }

    /// How its copy is compared, as `asked` (whose exercise it is); none while there is nothing
    /// to compare.
    #[must_use]
    pub fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        self.kind.described().compare(asked)
    }

    /// The picture the practice crate draws for it (`original` its original, `picture` its
    /// pack's): a generated combination's, an object's, value masses' answer.
    #[must_use]
    pub fn painted(&self, original: &Original, picture: Option<&SheetPicture>) -> Option<Painted> {
        self.kind.described().painted(original, picture)
    }
}

impl Family {
    /// Its tile picture (S91), as its kind's folder draws it.
    #[must_use]
    pub fn tile(self) -> Vec<TileMark> {
        self.described().tile(self)
    }

    /// `original`'s answer (an exercise of this family's) as a hand would draw it, a stroke at a
    /// time (frame units); none for a kind without one.
    #[must_use]
    pub fn answer(self, original: &Original) -> Vec<Vec<Point>> {
        self.described().answer(original)
    }
}

/// `item`'s place in `all`, as a stored variant (0 where it is not there).
pub(crate) fn place<T: PartialEq>(all: &[T], item: &T) -> u8 {
    all.iter()
        .position(|held| held == item)
        .map_or(0, |at| u8::try_from(at).unwrap_or(0))
}
