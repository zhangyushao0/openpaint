//! What a session holds: the stages, an exercise's kind and seed (and a single shape's or a colour
//! copy's picture) and its original; the stages' sessions are `session.rs`, the sheet they lie
//! on `sheet.rs`.

use crate::areas::{Areas, Arrangement};
use crate::colour::Rgb8;
use crate::colour_match::Tone;
use crate::combined::{Combination, CombinedLines};
use crate::line::{LineExercise, LineTier};
use crate::pack::{ItemRef, Tier};
use crate::path::Point;
use crate::scale::{ScaleTone, ValueScale};

/// The course's stages, after V大's eight weeks.
pub const STAGES: u8 = 8;
/// The stages whose exercises exist so far: a session can be started in these.
pub const STAGES_READY: u8 = 2;
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
    /// A small abstract picture of a few colour areas, copied by eye (V大's 临摹色彩组合).
    Areas(Arrangement),
    /// A row of cells between a given dark and light, filled in even steps of value.
    Scale(ScaleTone),
    /// A long colour copy of a picture (V大's 色彩临摹): a project beside the sessions, never
    /// one of their exercises.
    Study,
}

/// What an exercise asks for, whatever its variant: its instructions, its tools, its line in
/// the summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Line,
    Combined,
    Shape,
    Colour,
    Areas,
    Scale,
    Study,
}

impl Family {
    /// In a warm-up's order.
    pub const ALL: [Self; 7] = [
        Self::Line,
        Self::Combined,
        Self::Shape,
        Self::Colour,
        Self::Areas,
        Self::Scale,
        Self::Study,
    ];

    /// Whether its exercises are drawn with the pens as lines and compared by overlay (lines,
    /// combined lines, single shapes), not coloured.
    #[must_use]
    pub const fn drawn(self) -> bool {
        matches!(self, Self::Line | Self::Combined | Self::Shape)
    }

    /// Whether its exercises are painted with brushes and read from the copy's pixels (colour
    /// combinations, value scales, colour copies).
    #[must_use]
    pub const fn painted(self) -> bool {
        matches!(self, Self::Areas | Self::Scale | Self::Study)
    }
}

impl Kind {
    /// The kind and its variant as stored: (1, tier) for a line, (2, tone) for a colour, (3,
    /// arrangement) for combined lines, (4, the picture's tier code) for a single shape, (5,
    /// arrangement) for a colour combination, (6, tone) for a value scale, (7, 0) for a colour
    /// copy. A code names one generator for good; a changed generator is a new code.
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
            Self::Scale(tone) => (6, index(ScaleTone::ALL.iter().position(|t| *t == tone))),
            Self::Study => (7, 0),
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
            6 => ScaleTone::ALL.get(at).map(|t| Self::Scale(*t)),
            7 => (variant == 0).then_some(Self::Study),
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
            Self::Areas(_) => Family::Areas,
            Self::Scale(_) => Family::Scale,
            Self::Study => Family::Study,
        }
    }
}

/// One exercise: its kind, the seed it is generated from and, for a single shape or a colour
/// copy, the picture it copies.
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
            Kind::Shape(_) | Kind::Study => {
                Original::Picture(self.picture.clone().unwrap_or_default())
            }
            Kind::Areas(arrangement) => Original::Areas(Areas::generate(arrangement, self.seed)),
            Kind::Scale(tone) => Original::Scale(ValueScale::generate(tone, self.seed)),
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
    /// A pack's picture (an empty reference: none was kept).
    Picture(ItemRef),
    /// A colour combination's areas.
    Areas(Areas),
    /// A value scale's ends and cells.
    Scale(ValueScale),
}
