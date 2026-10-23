//! What a session holds and where it lies: the stages, an exercise's kind and seed (and a single
//! shape's or a colour copy's picture), the stages' sessions (`session.rs`), and the sheet the app
//! lays the exercises out on, a pair of frames each (the original at the left, the empty frame
//! for the copy at its right), three pairs a row; a long colour copy's sheet, one pair of frames
//! shaped as its picture; a picture fitted inside its frame.

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
/// The margin a picture keeps inside its frame, a share of the frame's side.
pub const PICTURE_MARGIN: f64 = 0.05;

/// A frame's side on the sheet, in canvas pixels.
pub const FRAME: f64 = 400.0;
/// A long colour copy's frames' longer side, in canvas pixels: room for 10 to 20 hours of detail.
pub const STUDY_SIDE: f64 = 1600.0;
/// Between the original's frame and the copy's.
const PAIR_GAP: f64 = 40.0;
/// Between pairs, across and down, and round the sheet's edge.
const COLUMN_GAP: f64 = 120.0;
const ROW_GAP: f64 = 120.0;
const EDGE: f64 = 80.0;
/// Pairs a row.
const COLUMNS: usize = 3;

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

/// Where a picture lies in its frame: evenly scaled to fit inside `PICTURE_MARGIN`, centred, or
/// filling a frame of its own shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PictureFit {
    /// Frame units per picture pixel.
    pub scale: f64,
    /// The picture's top left corner, in frame units.
    pub origin: Point,
    /// Its width and height, in frame units.
    pub size: Point,
}

impl PictureFit {
    /// The fit of a picture of `size` pixels in a square frame.
    #[must_use]
    pub fn of(size: [u32; 2]) -> Self {
        let [width, height] = size.map(|side| f64::from(side.max(1)));
        let room = 1.0 - 2.0 * PICTURE_MARGIN;
        let scale = room / width.max(height);
        let shown = [width * scale, height * scale];
        Self {
            scale,
            origin: [(1.0 - shown[0]) / 2.0, (1.0 - shown[1]) / 2.0],
            size: shown,
        }
    }

    /// The fit of a picture of `size` pixels filling a frame of its own shape (a colour copy's).
    #[must_use]
    pub fn filling(size: [u32; 2]) -> Self {
        let [width, height] = size.map(|side| f64::from(side.max(1)));
        Self {
            scale: 1.0 / width,
            origin: [0.0, 0.0],
            size: [1.0, height / width],
        }
    }

    /// The frame point of the picture's pixel point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.scale,
            self.origin[1] + p[1] * self.scale,
        ]
    }

    /// The picture's pixel point of the frame point `p`.
    #[must_use]
    pub fn to_picture(&self, p: Point) -> Point {
        [
            (p[0] - self.origin[0]) / self.scale,
            (p[1] - self.origin[1]) / self.scale,
        ]
    }
}

/// A frame on the sheet, in canvas pixels: square, or a colour copy's of its picture's shape.
/// Frame units run evenly on both axes, one to the frame's width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub origin: Point,
    /// Width and height.
    pub size: Point,
}

impl Frame {
    /// A square frame of `side` pixels at `origin`.
    #[must_use]
    pub const fn square(origin: Point, side: f64) -> Self {
        Self {
            origin,
            size: [side, side],
        }
    }

    /// Canvas pixels per frame unit: the frame's width.
    #[must_use]
    pub const fn unit(&self) -> f64 {
        self.size[0]
    }

    /// The canvas point of the frame point `p` (frame units).
    #[must_use]
    pub fn to_canvas(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.unit(),
            self.origin[1] + p[1] * self.unit(),
        ]
    }

    /// The frame point (frame units) of the canvas point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            (p[0] - self.origin[0]) / self.unit(),
            (p[1] - self.origin[1]) / self.unit(),
        ]
    }

    /// Whether the canvas point `p` lies in the frame grown by `margin` pixels on every side.
    #[must_use]
    pub fn contains(&self, p: Point, margin: f64) -> bool {
        (0..2).all(|axis| {
            p[axis] >= self.origin[axis] - margin
                && p[axis] <= self.origin[axis] + self.size[axis] + margin
        })
    }

    /// Its corners, top left first, clockwise, in canvas pixels.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        let ([x, y], [w, h]) = (self.origin, self.size);
        [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    }
}

/// One exercise's frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub original: Frame,
    pub copy: Frame,
}

impl Pair {
    /// The middle of both frames, in canvas pixels.
    #[must_use]
    pub fn centre(&self) -> Point {
        [
            (self.original.origin[0] + self.copy.origin[0] + self.copy.size[0]) / 2.0,
            self.original.origin[1] + self.original.size[1] / 2.0,
        ]
    }
}

/// The sheet a session's exercises lie on.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// Width and height in canvas pixels.
    pub size: [u32; 2],
    /// Each exercise's frames, in order: three pairs a row, rows down the sheet.
    pub pairs: Vec<Pair>,
}

impl Sheet {
    /// The sheet for `count` exercises.
    #[must_use]
    pub fn of(count: usize) -> Self {
        let pair_width = 2.0 * FRAME + PAIR_GAP;
        let rows = count.div_ceil(COLUMNS).max(1);
        let pairs = (0..count)
            .map(|index| {
                let (column, row) = ((index % COLUMNS) as f64, (index / COLUMNS) as f64);
                let x = EDGE + column * (pair_width + COLUMN_GAP);
                let y = EDGE + row * (FRAME + ROW_GAP);
                Pair {
                    original: Frame::square([x, y], FRAME),
                    copy: Frame::square([x + FRAME + PAIR_GAP, y], FRAME),
                }
            })
            .collect();
        let width = 2.0 * EDGE + COLUMNS as f64 * pair_width + (COLUMNS - 1) as f64 * COLUMN_GAP;
        let height = 2.0 * EDGE + rows as f64 * FRAME + (rows - 1) as f64 * ROW_GAP;
        Self {
            size: [width as u32, height as u32],
            pairs,
        }
    }

    /// A long colour copy's sheet for a picture of `picture` pixels: one pair of frames of the
    /// picture's shape, their longer side `STUDY_SIDE`.
    #[must_use]
    pub fn study(picture: [u32; 2]) -> Self {
        let [width, height] = picture.map(|side| f64::from(side.max(1)));
        let scale = STUDY_SIDE / width.max(height);
        let size = [(width * scale).round(), (height * scale).round()];
        let original = Frame {
            origin: [EDGE, EDGE],
            size,
        };
        let copy = Frame {
            origin: [EDGE + size[0] + PAIR_GAP, EDGE],
            size,
        };
        let sheet = [2.0 * EDGE + 2.0 * size[0] + PAIR_GAP, 2.0 * EDGE + size[1]];
        Self {
            size: sheet.map(|side| side as u32),
            pairs: vec![Pair { original, copy }],
        }
    }
}
