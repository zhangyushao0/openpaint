//! What a kind is and what its description tells (S109, D111): `Kind` names an exercise's kind with
//! what varies its look, `Original` what it gives, `Facts` what the app and the UI read of each of
//! its families (its key, its work, its frames), and `Described` what each kind's folder implements:
//! its stored code, generator, How and drill. Every call here goes through `KINDS`.

use std::sync::Arc;

use super::KINDS;
use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::demos::demo::Demo;
use crate::geometry::ellipse::{EllipseGuide, Guide};
use crate::geometry::perspective::{BoxView, Horizon, PerspectiveBox};
use crate::kinds::colours::colour_match::Tone;
use crate::kinds::combined_lines::combined::Combination;
use crate::kinds::estimates::flat::{Estimate, Flat};
use crate::kinds::key_shifts::key_shift::Tendency;
use crate::kinds::lines::line::LineTier;
use crate::kinds::objects::object::ObjectTask;
use crate::kinds::objects::object_design::Carve;
use crate::kinds::points_in_space::space::{SpaceTask, Trick};
use crate::kinds::scales::scale::{ScaleTone, ValueScale};
use crate::kinds::solids::solid::{Solid, SolidTask};
use crate::marks::compared::{Compared, Comparison, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Before;

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
}

impl Kind {
    /// Its description, the folder's that names its variant.
    pub(crate) fn described(self) -> &'static dyn Described {
        (KINDS.iter().copied())
            .find(|kind| kind.variant(self).is_some())
            .unwrap_or(KINDS[0])
    }

    /// The kind and its variant as stored: each kind's code is its place in `KINDS` (1 to 19), its
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
}

/// What an exercise's original is.
#[derive(Debug, Clone, PartialEq)]
pub enum Original {
    /// One line or a group of them, each in frame units.
    Lines(Vec<Vec<Point>>),
    /// A swatch.
    Colour(Rgb8),
    /// A pack's picture (an empty reference: none was kept); a crop's with its areas (S101).
    Picture(ItemRef),
    /// A generated colour combination's picture and areas (a record of 1.18.1 and before).
    Areas(GeneratedAreas),
    /// A value scale's ends and cells.
    Scale(ValueScale),
    /// A box to complete: the exact box, its front corner's three edges given.
    Box(PerspectiveBox),
    /// A free-boxes page: its horizon is the example box's.
    FreeBoxes(PerspectiveBox),
    /// An ellipse's guide and the exact ellipse.
    Ellipse(EllipseGuide),
    /// An estimate's givens: its answer follows from them.
    Flat(Flat),
    /// A box with a face to fix a point in space on.
    Space(Box<SpaceTask>),
    /// A box with a solid to find in it.
    Solid(Box<SolidTask>),
    /// An object to simplify into its box.
    Object(Box<ObjectTask>),
}

/// How a family's copy is made and compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Figure with the pens as lines and laid over the original, in place or aligned (lines,
    /// combined lines, single shapes).
    Lines,
    /// Figure as lines and checked as geometry against an exact answer or rule, never aligned
    /// (boxes, ellipses, estimates, points in space, solids, objects).
    Geometry,
    /// Figure with a division's pens in V大's three colours: no answer, the picture laid under the
    /// lines (a division, a whole copy's division, S95).
    Divided,
    /// A single colour matched by picking.
    Colour,
    /// Painted with brushes and read from the copy's pixels (colour combinations, value scales,
    /// colour copies, value masses, key shifts).
    Painted,
}

/// What the app and the UI read of a family: the key its words go by in the UI's tables
/// (`ui/src/text/practice/`; its tile, its test drill's name too), how its copy is made and
/// compared, whether its answer is built from given parts in the one frame drawn in (S97, S98:
/// the original's frame beside it left empty, room for what Compare draws beyond the frame) and
/// whether it copies a composition's picture (S95, S96).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub family: Family,
    pub key: &'static str,
    pub work: Work,
    pub constructed: bool,
    pub composed: bool,
}

impl Facts {
    /// A family of `work` keyed `key`, neither constructed nor composed.
    pub(crate) const fn of(family: Family, key: &'static str, work: Work) -> Self {
        Self {
            family,
            key,
            work,
            constructed: false,
            composed: false,
        }
    }

    pub(crate) const fn constructed(self) -> Self {
        Self {
            constructed: true,
            ..self
        }
    }

    pub(crate) const fn composed(self) -> Self {
        Self {
            composed: true,
            ..self
        }
    }
}

/// A drill's place among its kind's (from 0), its seed and what came before it.
pub(crate) struct Turn<'a> {
    pub(crate) turn: usize,
    pub(crate) seed: u64,
    pub(crate) before: &'a Before<'a>,
}

/// What a kind's folder tells of it (S109, D111).
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
    /// A drill of `family` at `turn` (S91, D110): its kinds and their pictures; none for a kind
    /// no stage drills.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
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
}

/// `item`'s place in `all`, as a stored variant (0 where it is not there).
pub(crate) fn place<T: PartialEq>(all: &[T], item: &T) -> u8 {
    all.iter()
        .position(|held| held == item)
        .map_or(0, |at| u8::try_from(at).unwrap_or(0))
}

impl Original {
    /// A value scale's count of cells; none for another original.
    #[must_use]
    pub fn cells(&self) -> Option<usize> {
        match self {
            Self::Scale(scale) => Some(scale.cells),
            _ => None,
        }
    }
}

impl Kind {
    /// A key shift's tendency; none for another kind.
    #[must_use]
    pub const fn tendency(self) -> Option<Tendency> {
        match self {
            Self::KeyShift(tendency) => Some(tendency),
            _ => None,
        }
    }
}
