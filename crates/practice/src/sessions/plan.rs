//! What a session holds: the stages, an exercise's kind and seed (and a single shape's or a colour
//! copy's picture) and its original; a kind's family is `family.rs`, the stages' sessions are
//! `session.rs`, the sheet they lie on `sheet.rs`.

use crate::areas_generated::{Arrangement, GeneratedAreas};
use crate::colour::Rgb8;
use crate::colour_match::Tone;
use crate::combined::{Combination, CombinedLines};
use crate::ellipse::{EllipseGuide, Guide};
pub use crate::family::Family;
use crate::flat::{Estimate, Flat};
use crate::key_shift::Tendency;
use crate::line::{LineExercise, LineTier};
use crate::object::ObjectTask;
use crate::object_design::Carve;
use crate::pack::{ItemRef, Tier};
use crate::path::Point;
use crate::perspective::{BoxView, Horizon, PerspectiveBox};
use crate::scale::{CELLS, FIRST_CELLS, ScaleTone, ValueScale};
use crate::solid::{Solid, SolidTask};
use crate::space::{SpaceTask, Trick};

/// The course's stages, after V大's eight weeks.
pub const STAGES: u8 = 8;
/// The stages whose exercises exist so far: a session can be started in these.
pub const STAGES_READY: u8 = 5;
/// About how many sessions a stage holds; only a guide, never a quota.
pub const STAGE_SESSIONS: u32 = 15;

/// The kind of an exercise, with what varies its look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A single line to copy.
    Line(LineTier),
    /// A single colour to match.
    Colour(Tone),
    /// Two to four lines to copy as one shape.
    Combined(Combination),
    /// One object's picture of a tier, copied by its outline as a line drawing.
    Shape(Tier),
    /// A generated colour combination (S82, 1.5.0 to 1.18.1): kept in its records, its picture
    /// made again from its seed; never planned since S101.
    Areas(Arrangement),
    /// A colour combination (V大's 临摹色彩组合, S101): a crop of a real picture of a tier (its
    /// areas' count, `crop::tier_of`), its few colour areas copied by eye.
    Crop(Tier),
    /// A row of cells (its count, the ends among them) between a given dark and light, filled in
    /// even steps of value.
    Scale(ScaleTone, u8),
    /// A long colour copy of a picture (V大's 色彩临摹): a project beside the sessions, never
    /// one of their exercises.
    Study,
    /// A box in perspective given by its front corner and three edges, to complete (V大's
    /// 透视盒子).
    Box(BoxView),
    /// A page with a horizon, a few boxes drawn freely on it.
    FreeBoxes(Horizon),
    /// An ellipse drawn to fit a guide (Drawabox's tables, V大's cylinder in a box).
    Ellipse(Guide),
    /// A point, a proportion or an angle estimated on the flat (V大's 二维抽象网格).
    Flat(Estimate),
    /// A point in space fixed on a box by one of V大's five tricks (点定位).
    Space(Trick),
    /// A cylinder, a cone or a sphere found in its box (V大's week 3, S93).
    Solid(Solid),
    /// An object made from a box by one or two changes, simplified back into it (V大's weeks 3
    /// and 4, 几何体 + 点定位 概括单体, S94).
    Object(Carve),
    /// A composition's picture of a tier divided by its most obvious lines in three levels
    /// (V大's 画面分割, S95).
    Division(Tier),
    /// A composition's picture of a tier divided, then copied whole as a line drawing (or in
    /// colour blocks, S96) over its division dimmed (V大's 整图复制, S95).
    WholeCopy(Tier),
    /// A composition's picture of a tier painted as its few big masses in three greys, grouped by
    /// value (V大's 黑白灰 and colour blocks by attribute, S96).
    ValueMasses(Tier),
    /// A crop painted again shifted toward a tendency the exercise gives (V大's 变调, S107).
    KeyShift(Tendency),
}

impl Kind {
    /// The kind and its variant as stored: (1, tier) for a line, (2, tone) for a colour, (3,
    /// arrangement) for combined lines, (4, the picture's tier code) for a single shape, (5,
    /// arrangement) for a generated colour combination (records only since S101), (6, tone + 2 x
    /// cells) for a value scale (S97; 1.5.0 to 1.14.0 stored (6, tone), the cells drawn from the
    /// seed: the store's upgrade turns it into this, `store/src/practice.rs`), (7, 0) for a colour
    /// copy, (8, view) for a box to complete, (9, horizon) for free boxes, (10, guide) for an
    /// ellipse, (11, estimate) for an estimate on the flat, (12, trick) for a point in space, (13,
    /// solid) for a solid in a box, (14, carve) for an object simplified into its box, then the
    /// picture's tier code for a division (15), a whole copy (16), value masses (17) and a colour
    /// combination of a crop (18, S101); (19, the tendency's code) for a key shift (S107). A code
    /// names one generator for good; a changed generator is a new code.
    #[must_use]
    pub fn code(self) -> (u8, u8) {
        let index = |found: Option<usize>| found.unwrap_or(0) as u8;
        match self {
            Self::Line(tier) => (1, index(LineTier::ALL.iter().position(|t| *t == tier))),
            Self::Colour(tone) => (2, tone.code()),
            Self::Combined(combination) => {
                let at = Combination::ALL.iter().position(|c| *c == combination);
                (3, index(at))
            }
            Self::Shape(tier) => (4, tier.code()),
            Self::Areas(arrangement) => {
                let at = Arrangement::ALL.iter().position(|a| *a == arrangement);
                (5, index(at))
            }
            Self::Scale(tone, cells) => {
                let tone = index(ScaleTone::ALL.iter().position(|t| *t == tone));
                (6, tone + 2 * cells)
            }
            Self::Study => (7, 0),
            Self::Box(view) => (8, index(BoxView::ALL.iter().position(|v| *v == view))),
            Self::FreeBoxes(horizon) => (9, index(Horizon::ALL.iter().position(|h| *h == horizon))),
            Self::Ellipse(guide) => (10, index(Guide::ALL.iter().position(|g| *g == guide))),
            Self::Flat(estimate) => (11, index(Estimate::ALL.iter().position(|e| *e == estimate))),
            Self::Space(trick) => (12, index(Trick::ALL.iter().position(|t| *t == trick))),
            Self::Solid(solid) => (13, index(Solid::ALL.iter().position(|s| *s == solid))),
            Self::Object(carve) => (14, index(Carve::ALL.iter().position(|c| *c == carve))),
            Self::Division(tier) => (15, tier.code()),
            Self::WholeCopy(tier) => (16, tier.code()),
            Self::ValueMasses(tier) => (17, tier.code()),
            Self::Crop(tier) => (18, tier.code()),
            Self::KeyShift(tendency) => (19, tendency.code()),
        }
    }

    #[must_use]
    pub fn of_code(kind: u8, variant: u8) -> Option<Self> {
        let at = usize::from(variant);
        match kind {
            1 => LineTier::ALL.get(at).map(|t| Self::Line(*t)),
            2 => Tone::of_code(variant).map(Self::Colour),
            3 => Combination::ALL.get(at).map(|c| Self::Combined(*c)),
            4 => Tier::of_code(variant).map(Self::Shape),
            5 => Arrangement::ALL.get(at).map(|a| Self::Areas(*a)),
            6 => {
                let cells = variant / 2;
                let kept = (usize::from(cells) >= FIRST_CELLS) && usize::from(cells) <= CELLS;
                (ScaleTone::ALL.get(at % 2)).and_then(|t| kept.then_some(Self::Scale(*t, cells)))
            }
            7 => (variant == 0).then_some(Self::Study),
            8 => BoxView::ALL.get(at).map(|v| Self::Box(*v)),
            9 => Horizon::ALL.get(at).map(|h| Self::FreeBoxes(*h)),
            10 => Guide::ALL.get(at).map(|g| Self::Ellipse(*g)),
            11 => Estimate::ALL.get(at).map(|e| Self::Flat(*e)),
            12 => Trick::ALL.get(at).map(|t| Self::Space(*t)),
            13 => Solid::ALL.get(at).map(|s| Self::Solid(*s)),
            14 => Carve::ALL.get(at).map(|c| Self::Object(*c)),
            15 => Tier::of_code(variant).map(Self::Division),
            16 => Tier::of_code(variant).map(Self::WholeCopy),
            17 => Tier::of_code(variant).map(Self::ValueMasses),
            18 => Tier::of_code(variant).map(Self::Crop),
            19 => Tendency::of_code(variant).map(Self::KeyShift),
            _ => None,
        }
    }

    #[must_use]
    pub const fn family(self) -> Family {
        match self {
            Self::Line(_) => Family::Line,
            Self::Colour(_) => Family::Colour,
            Self::Combined(_) => Family::Combined,
            Self::Shape(_) => Family::Shape,
            Self::Areas(_) | Self::Crop(_) => Family::Areas,
            Self::Scale(..) => Family::Scale,
            Self::Study => Family::Study,
            Self::Box(_) => Family::Box,
            Self::FreeBoxes(_) => Family::FreeBoxes,
            Self::Ellipse(_) => Family::Ellipse,
            Self::Flat(estimate) => estimate.family(),
            Self::Space(trick) => trick.family(),
            Self::Solid(solid) => solid.family(),
            Self::Object(_) => Family::Object,
            Self::Division(_) => Family::Division,
            Self::WholeCopy(_) => Family::WholeCopy,
            Self::ValueMasses(_) => Family::ValueMasses,
            Self::KeyShift(_) => Family::KeyShift,
        }
    }
}

/// One exercise: its kind, the seed it is generated from and, for a single shape, a colour copy,
/// a division or a whole copy, the picture it copies.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Exercise {
    pub kind: Kind,
    pub seed: u64,
    pub picture: Option<ItemRef>,
}

impl Exercise {
    /// A generated exercise of `kind` seeded `seed`, no picture.
    #[must_use]
    pub const fn generated(kind: Kind, seed: u64) -> Self {
        Self {
            kind,
            seed,
            picture: None,
        }
    }

    /// Its original, the same for the same kind and seed.
    #[must_use]
    pub fn original(&self) -> Original {
        match self.kind {
            Kind::Line(tier) => {
                Original::Lines(vec![LineExercise::generate(tier, self.seed).points])
            }
            Kind::Colour(tone) => Original::Colour(tone.colour(self.seed)),
            Kind::Combined(combination) => {
                Original::Lines(CombinedLines::generate(combination, self.seed).lines)
            }
            Kind::Shape(_)
            | Kind::Study
            | Kind::Division(_)
            | Kind::WholeCopy(_)
            | Kind::ValueMasses(_)
            | Kind::Crop(_)
            | Kind::KeyShift(_) => Original::Picture(self.picture.clone().unwrap_or_default()),
            Kind::Areas(arrangement) => {
                Original::Areas(GeneratedAreas::generate(arrangement, self.seed))
            }
            Kind::Scale(tone, cells) => {
                Original::Scale(ValueScale::generate(tone, usize::from(cells), self.seed))
            }
            Kind::Box(view) => Original::Box(PerspectiveBox::generate(view, self.seed)),
            Kind::FreeBoxes(horizon) => {
                Original::FreeBoxes(PerspectiveBox::on_horizon(horizon, self.seed))
            }
            Kind::Ellipse(guide) => Original::Ellipse(EllipseGuide::generate(guide, self.seed)),
            Kind::Flat(estimate) => Original::Flat(Flat::generate(estimate, self.seed)),
            Kind::Space(trick) => Original::Space(Box::new(SpaceTask::generate(trick, self.seed))),
            Kind::Solid(solid) => Original::Solid(Box::new(SolidTask::generate(solid, self.seed))),
            Kind::Object(carve) => {
                Original::Object(Box::new(ObjectTask::generate(carve, self.seed)))
            }
        }
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
