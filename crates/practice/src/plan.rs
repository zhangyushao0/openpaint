//! What a session holds and where it lies: the stages, an exercise's kind and seed (and a single
//! shape's picture), the stages' sessions (`session.rs`), and the sheet the app lays the exercises
//! out on, a pair of frames each (the original at the left, the empty frame for the copy at its
//! right), three pairs a row; a picture fitted inside its frame.

use crate::colour::Rgb8;
use crate::colour_match::Tone;
use crate::combined::{Combination, CombinedLines};
use crate::line::{LineExercise, LineTier};
use crate::pack::{ItemRef, Tier};
use crate::path::Point;

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
}

/// What an exercise asks for, whatever its variant: its instructions, its tools, its line in
/// the summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Line,
    Combined,
    Shape,
    Colour,
}

impl Family {
    /// In a warm-up's order.
    pub const ALL: [Self; 4] = [Self::Line, Self::Combined, Self::Shape, Self::Colour];

    /// Whether its exercises are drawn (with the pens, compared by overlay), not coloured.
    #[must_use]
    pub const fn drawn(self) -> bool {
        !matches!(self, Self::Colour)
    }
}

impl Kind {
    /// The kind and its variant as stored: (1, tier) for a line, (2, tone) for a colour, (3,
    /// arrangement) for combined lines, (4, the picture's tier code) for a single shape. A code
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
        }
    }
}

/// One exercise: its kind, the seed it is generated from and, for a single shape, the picture
/// it copies.
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
            Kind::Shape(_) => Original::Picture(self.picture.clone().unwrap_or_default()),
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
}

/// Where a picture lies in its frame: evenly scaled to fit inside `PICTURE_MARGIN`, centred.
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
    /// The fit of a picture of `size` pixels.
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

    /// The frame point of the picture's pixel point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.scale,
            self.origin[1] + p[1] * self.scale,
        ]
    }
}

/// A square frame on the sheet, in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub origin: Point,
    pub side: f64,
}

impl Frame {
    /// The canvas point of the frame point `p` (frame units).
    #[must_use]
    pub fn to_canvas(&self, p: Point) -> Point {
        [
            self.origin[0] + p[0] * self.side,
            self.origin[1] + p[1] * self.side,
        ]
    }

    /// The frame point (frame units) of the canvas point `p`.
    #[must_use]
    pub fn to_frame(&self, p: Point) -> Point {
        [
            (p[0] - self.origin[0]) / self.side,
            (p[1] - self.origin[1]) / self.side,
        ]
    }

    /// Whether the canvas point `p` lies in the frame grown by `margin` pixels on every side.
    #[must_use]
    pub fn contains(&self, p: Point, margin: f64) -> bool {
        (0..2).all(|axis| {
            p[axis] >= self.origin[axis] - margin
                && p[axis] <= self.origin[axis] + self.side + margin
        })
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
            (self.original.origin[0] + self.copy.origin[0] + self.copy.side) / 2.0,
            self.original.origin[1] + self.original.side / 2.0,
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
                    original: Frame {
                        origin: [x, y],
                        side: FRAME,
                    },
                    copy: Frame {
                        origin: [x + FRAME + PAIR_GAP, y],
                        side: FRAME,
                    },
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
}
