//! What a session holds and where it lies: the stages, an exercise's kind and seed, stage 1's
//! session of ten lines and eight colours, and the sheet the app lays the exercises out on, a
//! pair of frames each (the original at the left, the empty frame for the copy at its right),
//! three pairs a row.

use crate::colour::Rgb8;
use crate::colour_match::{Lightness, Purity, Tone};
use crate::line::{LineExercise, LineTier};
use crate::path::Point;
use crate::random::derived;

/// The course's stages, after V大's eight weeks.
pub const STAGES: u8 = 8;
/// About how many sessions a stage holds; only a guide, never a quota.
pub const STAGE_SESSIONS: u32 = 15;

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
}

impl Kind {
    /// The kind and its variant as stored: (1, tier) for a line, (2, tone) for a colour. A code
    /// names one generator for good; a changed generator is a new code.
    #[must_use]
    pub fn code(self) -> (u8, u8) {
        match self {
            Self::Line(tier) => {
                let index = LineTier::ALL.iter().position(|t| *t == tier);
                (1, index.unwrap_or(0) as u8)
            }
            Self::Colour(tone) => (2, tone.code()),
        }
    }

    #[must_use]
    pub fn of_code(kind: u8, variant: u8) -> Option<Self> {
        match kind {
            1 => LineTier::ALL
                .get(usize::from(variant))
                .map(|t| Self::Line(*t)),
            2 => Tone::of_code(variant).map(Self::Colour),
            _ => None,
        }
    }

    /// True for the line kinds.
    #[must_use]
    pub const fn is_line(self) -> bool {
        matches!(self, Self::Line(_))
    }
}

/// One exercise: its kind and the seed it is generated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exercise {
    pub kind: Kind,
    pub seed: u64,
}

/// What an exercise's original is.
#[derive(Debug, Clone, PartialEq)]
pub enum Original {
    /// A line, in frame units.
    Line(LineExercise),
    /// A swatch.
    Colour(Rgb8),
}

impl Exercise {
    /// Its original, the same for the same kind and seed.
    #[must_use]
    pub fn original(&self) -> Original {
        match self.kind {
            Kind::Line(tier) => Original::Line(LineExercise::generate(tier, self.seed)),
            Kind::Colour(tone) => Original::Colour(tone.colour(self.seed)),
        }
    }
}

/// Stage 1's lines, in order: two near-straight ones to warm up, then bends of every kind.
const LINES: [LineTier; 10] = [
    LineTier::Straight,
    LineTier::Straight,
    LineTier::Arc,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Hook,
    LineTier::Wave,
    LineTier::Hook,
];

/// Stage 1's colours, in order: five of eight low in saturation, every lightness.
const COLOURS: [(Purity, Lightness); 8] = [
    (Purity::Muted, Lightness::Middle),
    (Purity::Grey, Lightness::Light),
    (Purity::Middle, Lightness::Dark),
    (Purity::Muted, Lightness::Light),
    (Purity::Vivid, Lightness::Middle),
    (Purity::Grey, Lightness::Dark),
    (Purity::Muted, Lightness::Dark),
    (Purity::Middle, Lightness::Light),
];

/// A session of stage 1 seeded `seed`: ten lines, then eight colours (about 8 and 14 minutes;
/// two sessions are about V大's day of single lines). Exercise `i`'s seed is `derived(seed, i)`.
#[must_use]
pub fn stage_one(seed: u64) -> Vec<Exercise> {
    let lines = LINES.iter().map(|tier| Kind::Line(*tier));
    let colours = COLOURS
        .iter()
        .map(|&(purity, lightness)| Kind::Colour(Tone { purity, lightness }));
    lines
        .chain(colours)
        .enumerate()
        .map(|(index, kind)| Exercise {
            kind,
            seed: derived(seed, index as u64),
        })
        .collect()
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
