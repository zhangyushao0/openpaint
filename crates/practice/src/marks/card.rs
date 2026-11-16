//! What the practice card shows of a compared exercise under its title, whatever its kind (S109,
//! D111): a small vocabulary the screen lays out without knowing the kind, words named by keys
//! (`Say`) that the UI's tables read in the language shown; a reason's (S138) made alike for
//! every kind that gives one; and what a card asks the user to choose before Compare (`Choices`,
//! S112).

use crate::compositions::levels::LevelRow;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::{Metric, Metrics, value};

/// A reason's degree when it says none (`Say::way`), beside "little" and "clearly".
const UNSAID: &str = "unsaid";

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

    /// The reason beside the app's result (S138, D121): what of the copy is off (`what`: point,
    /// mark, line, upright, colour), which way, and how far in words, never a number: a little
    /// while `off` lies within `near`, the band in which the check calls it near, clearly beyond.
    #[must_use]
    pub fn reason(what: &'static str, way: &'static str, (off, near): (f64, f64)) -> Self {
        let degree = if off.abs() <= near {
            "little"
        } else {
            "clearly"
        };
        Self {
            key: "reason",
            args: vec![Arg::Word(what), Arg::Word(way), Arg::Word(degree)],
        }
    }

    /// A reason saying what is off and which way only, its degree left unsaid (S139): a value
    /// scale's cell, whose result counts the cells uneven rather than measuring how far one is.
    #[must_use]
    pub fn way(what: &'static str, way: &'static str) -> Self {
        Self {
            key: "reason",
            args: vec![Arg::Word(what), Arg::Word(way), Arg::Word(UNSAID)],
        }
    }

    /// The same reason of the area or cell `number` (from 1).
    #[must_use]
    pub fn numbered(mut self, number: usize) -> Self {
        self.args.push(Arg::Count(number));
        self
    }
}

/// Which way `offset` (the copy's less the answer's, frame units, y down) points on the screen by
/// its larger part: left, right, up or down.
#[must_use]
pub(crate) fn screen_way([x, y]: Point) -> &'static str {
    match (x.abs() >= y.abs(), x < 0.0, y < 0.0) {
        (true, true, _) => "left",
        (true, false, _) => "right",
        (false, _, true) => "up",
        (false, _, false) => "down",
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
    /// The hue and chroma disc with a gamut mask (S153): the areas' original colours as small
    /// rings and the copy's as dots (the chosen one heavier), the convex polygon `corners`
    /// outlined and the disc veiled outside it, the `marked` areas' dots ringed red.
    Gamut {
        originals: Vec<Rgb8>,
        copy: Vec<Option<Rgb8>>,
        corners: Vec<Point>,
        marked: Vec<usize>,
    },
    /// The hue and chroma disc with the colours an exercise gave to mix from (S154: colour under
    /// light's local colour, light, sky and ground): each given colour a small square, each of the
    /// copy's regions a dot in its colour (none where not painted) with a thin line from the given
    /// colour it was mixed from where it names one, the `marked` regions' dots ringed red.
    Mixed {
        given: Vec<Rgb8>,
        copy: Vec<(Option<Rgb8>, Option<usize>)>,
        marked: Vec<usize>,
    },
    /// The lightness bar: the original's lightness over it, the copy's under it (S107).
    Bar { original: Rgb8, copy: Rgb8 },
    /// A point compared on the hue and chroma disc (S154): the original's colour there a small
    /// ring, the copy's a dot in its colour, an arrow between, and the two tendencies the copy
    /// leans toward, each a disc of its colour where it lies (Oklab `a`, `b`: a cold-warm copy's
    /// lights' and shadows').
    Leaning {
        original: Rgb8,
        copy: Rgb8,
        tendencies: [(Point, Rgb8); 2],
    },
    /// A division's levels, their lines counted and measured (S95).
    Levels([LevelRow; 3]),
    /// A row of chips, each a word's key, those `Choices::chips` holds on; a tap turns one, until
    /// the first Compare (S112).
    Toggles(&'static [&'static str]),
    /// A row of small pictures of bars, each its word's key and its bars' heights (0 to 1), the
    /// one `Choices::shape` holds on; a tap chooses it, until the first Compare (S112).
    Shapes(&'static [(&'static str, [f32; 5])]),
    /// Compare's views of the copy, each a word's key, the one `Seen::view` shows on; a tap shows
    /// another (S112).
    Views(&'static [&'static str]),
}

/// What the user chose on a card before the first Compare (S112): chips of its row toggled,
/// several on (a bit each, by their place), and one of its row of pictures. Kept with the first
/// Compare and fixed from then on (D99: the eye before the answer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choices {
    pub chips: u8,
    pub shape: Option<u8>,
}

impl Choices {
    /// With chip `chip` (from 0, under 8) turned.
    #[must_use]
    pub const fn toggled(self, chip: u8) -> Self {
        Self {
            chips: self.chips ^ (1 << (chip % 8)),
            shape: self.shape,
        }
    }

    /// Whether chip `chip` is on.
    #[must_use]
    pub const fn on(self, chip: u8) -> bool {
        self.chips & (1 << (chip % 8)) != 0
    }

    /// As a first Compare keeps them: the chips as a bit set, the picture chosen where one is.
    #[must_use]
    pub fn metrics(self) -> Metrics {
        let mut metrics = vec![(Metric::ChosenChips, f64::from(self.chips))];
        metrics.extend((self.shape).map(|shape| (Metric::ChosenShape, f64::from(shape))));
        metrics
    }

    /// The choices a first Compare kept in `metrics`; none where it kept none.
    #[must_use]
    pub fn kept(metrics: &[(Metric, f64)]) -> Option<Self> {
        let chips = value(metrics, Metric::ChosenChips)?;
        let shape = value(metrics, Metric::ChosenShape);
        let code = |v: f64| v.clamp(0.0, f64::from(u8::MAX)).round() as u8;
        Some(Self {
            chips: code(chips),
            shape: shape.map(code),
        })
    }
}
