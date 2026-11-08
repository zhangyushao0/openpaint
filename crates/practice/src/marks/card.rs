//! What the practice card shows of a compared exercise under its title, whatever its kind (S109,
//! D111): a small vocabulary the screen lays out without knowing the kind, words named by keys
//! (`Say`) that the UI's tables read in the language shown.

use crate::compositions::levels::LevelRow;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// Words the card says, named by a key the UI's tables know, with what they count or name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Say {
    pub key: &'static str,
    pub args: Vec<Arg>,
}

impl Say {
    /// `key`'s words, nothing in them.
    #[must_use]
    pub const fn plain(key: &'static str) -> Self {
        Self {
            key,
            args: Vec::new(),
        }
    }

    /// `key`'s words counting `counts`.
    #[must_use]
    pub fn counting(key: &'static str, counts: &[usize]) -> Self {
        Self {
            key,
            args: counts.iter().map(|count| Arg::Count(*count)).collect(),
        }
    }
}

/// What a card's words take: a count, or a word named by its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    Count(usize),
    Word(&'static str),
}

/// The wheel's size on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelSize {
    /// A colour's, the card's widest.
    Large,
    /// A colour copy's and a whole copy's point compared.
    Middle,
    /// A colour combination's, under its chips.
    Small,
}

/// An item of the card, in order under the title.
#[derive(Debug, Clone, PartialEq)]
pub enum CardItem {
    /// That the copy is being read, quietly.
    Reading,
    /// In place or Aligned (while its fit is found, saying so).
    Overlaid,
    /// A line of words.
    Says(Say),
    /// A line after a red dot: a check that failed (S107).
    Failed(Say),
    /// Construction (看画法), on or off (S97).
    Construction,
    /// The Grey view (看黑白), on or off (S107).
    Grey,
    /// How strongly the original shows over the copy.
    Slider,
    /// A chip for each area, the original's colour over the copy's (none: not painted), the
    /// chosen one ringed, the `marked` ones outlined red; a tap chooses one.
    Chips {
        originals: Vec<Rgb8>,
        copy: Option<Vec<Option<Rgb8>>>,
        marked: Vec<usize>,
    },
    /// Two colours on the wheel, the way from one to the other, and the colour table.
    Wheel {
        original: Rgb8,
        copy: Rgb8,
        size: WheelSize,
    },
    /// The hue and chroma disc (Oklab `a`, `b`): the areas' original colours and the copy's (the
    /// chosen one heavier), the tendency where it lies in its colour, and Gurney's gamut mask
    /// (S107), outside which the disc is veiled.
    Disc {
        originals: Vec<Rgb8>,
        copy: Vec<Option<Rgb8>>,
        tendency: (Point, Rgb8),
        mask: [Point; 3],
    },
    /// The lightness bar: the original's lightness over it, the copy's under it (S107).
    Bar { original: Rgb8, copy: Rgb8 },
    /// A division's levels, their lines counted and measured (S95).
    Levels([LevelRow; 3]),
}
