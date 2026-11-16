//! What a practice sheet draws over an exercise's pair of frames, whatever its kind (S109, D111): a
//! small vocabulary of marks in a frame's units that the screen draws without knowing the kind.
//! A kind's givens (`Described::givens`) and Compare's marks (`Compared::marks`) are lists of these,
//! drawn in order.

use crate::compositions::levels::Level;
use crate::compositions::value_masses::Value;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::sheets::layout::Frame;

/// Which frame of the pair a mark lies in, in that frame's units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum In {
    Original,
    Copy,
}

/// How a line is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinePen {
    /// An original's line in ink, an original's width.
    Ink,
    /// A given part printed grey (S97), an original's width; `PrintedHalf` half as wide.
    Printed,
    PrintedHalf,
    /// The answer in see-through red, as wide as the Draw pen.
    Answer,
    /// An original laid over the copy in the marking red at half strength, as wide as the pen.
    Laid,
    /// What is off, the marking red, a mark's width.
    Off,
    /// The way to see an answer, faint (Construction).
    Faint,
    /// Pencil, faint ink: an object's box and construction (S94).
    Pencil,
    /// A division's level in V大's colour, as wide as the pen, round (S95).
    Level(Level),
    /// Hatching where a copy's value differs from the answer's, in the answer's grey (S96).
    Hatch(Value),
    /// Hatching where the answer holds the darkest grey (S153: a core shadow, a contact).
    HatchDarkest,
}

/// Where a line may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clip {
    /// Figure as it lies.
    Free,
    /// Figure as it lies, its canvas points held as an original laid over the copy holds them
    /// (single precision).
    Laid,
    /// Cut at the sheet's edge (S98), a ring off it left out.
    Sheet,
    /// Cut at the sheet's edge and kept off the original's frame (S88: an object's marks).
    KeptOff,
    /// Kept inside its own frame (an object's construction over its picture, a division's lines).
    Inside,
}

/// Which picture a mark lays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The exercise's pack picture.
    Pack,
    /// The picture the practice crate drew for it (a generated combination's, an object's, value
    /// masses' answer).
    Painted,
    /// The picture drawn for it, else its pack picture (a colour combination's).
    PaintedOrPack,
    /// A picture the compared copy's reading brought, by its place among them (`Read::pictures`:
    /// a key shift's frames in grey for its Grey view, S107).
    Read(u8),
}

/// Where a picture lies in its frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Place {
    /// Filling the frame.
    Frame,
    /// Between these corners (top left, top right, bottom right, bottom left).
    Corners([Point; 4]),
}

/// What a mark draws.
#[derive(Debug, Clone, PartialEq)]
pub enum Figure {
    /// A line through the points in `pen`, dashed or whole.
    Line {
        points: Vec<Point>,
        pen: LinePen,
        dashed: bool,
    },
    /// A point ringed red over the paper's halo: what is off, a vanishing point.
    Ring(Point),
    /// A corner found (an object's construction): a small ink ring.
    Found(Point),
    /// A middle dotted (Construction); `small`: an object's.
    Dot { at: Point, small: bool },
    /// A given point, a dot in `pen`'s colour (ink or printed grey) of a frame's size.
    Given { at: Point, pen: LinePen },
    /// The end a proportion counts from, ringed in `pen` over the paper, `radius` frame units.
    Counted {
        at: Point,
        radius: f64,
        pen: LinePen,
    },
    /// A front corner under 90 degrees: an arc between its legs and a ring, red.
    Narrow { at: Point, legs: [Point; 2] },
    /// A face shaded, printed grey.
    Shade(Vec<Point>),
    /// The horizon at this height, printed grey and labelled.
    Horizon(f64),
    /// A colour filling the frame (a colour's swatch).
    Swatch(Rgb8),
    /// A polygon filled with a colour (a scale's cell).
    Fill { corners: Vec<Point>, colour: Rgb8 },
    /// A colour compared, as a patch in the middle of the frame.
    Patch(Rgb8),
    /// A value scale's cell (its top left and bottom right), outlined; in ink while in hand.
    Cell { cell: usize, rect: [Point; 2] },
    /// A cell's rule, thin.
    Rule([Point; 2]),
    /// A cell outlined red over the paper's halo.
    Uneven([Point; 2]),
    /// Lines over the paper's halo, red (at least the pen's width) or ink.
    Haloed { lines: Vec<Vec<Point>>, red: bool },
    /// A picture laid, `shown` strong.
    Picture {
        source: Source,
        place: Place,
        shown: f32,
    },
    /// The exercise's pack picture filling its frame of its shape (S99), its credit under it; or
    /// that it was removed.
    Pictured,
    /// A key shift's tendency printed beside the frame as a disc of its colour, named under it.
    Tendency(Rgb8),
    /// A gamut mask printed beside the frame (S153): a small hue and chroma disc (Oklab `a`, `b`,
    /// as the card's) of `radius`, centred `at` (frame units from the frame's top right corner,
    /// `beside`), the convex polygon `corners` outlined on it and the disc veiled outside.
    Gamut {
        at: Point,
        radius: f64,
        corners: Vec<Point>,
    },
    /// A colour given beside the frame (S153): a swatch `BESIDE_SIDE` square centred `at` (frame
    /// units from the frame's top right corner, `beside`), its name's key under it where it has
    /// one (one of `BESIDE_NAMES`); the eyedropper picks its colour whole. A gamut mask's corners
    /// (the colours a painter premixes), and any colours an exercise hands the learner to mix from.
    Beside {
        at: Point,
        colour: Rgb8,
        name: Option<&'static str>,
    },
}

/// A swatch's side beside a frame (`Figure::Beside`), frame units.
pub const BESIDE_SIDE: f64 = 0.07;
/// The keys a swatch beside a frame may be named by, the UI's words under it (`Figure::Beside`);
/// stored by their place here, so a key is appended, never moved. Colour under light's (S154): the
/// local colour, the sun's or a lamp's, the sky's and the ground's.
pub const BESIDE_NAMES: [&str; 5] = [
    "local-colour",
    "sunlight",
    "lamplight",
    "skylight",
    "ground",
];

/// The canvas point of the point `at` beside `frame`: frame units from its top right corner.
#[must_use]
pub fn beside(frame: &Frame, [x, y]: Point) -> Point {
    let unit = frame.unit();
    [
        frame.origin[0] + frame.size[0] + x * unit,
        frame.origin[1] + y * unit,
    ]
}

/// A mark over a pair of frames.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetMark {
    pub on: In,
    pub drawn: Figure,
    pub clip: Clip,
}

impl SheetMark {
    /// `drawn` in `on`, as it lies.
    #[must_use]
    pub const fn free(on: In, drawn: Figure) -> Self {
        Self {
            on,
            drawn,
            clip: Clip::Free,
        }
    }

    /// `drawn` in `on`, cut at the sheet's edge.
    #[must_use]
    pub const fn cut(on: In, drawn: Figure) -> Self {
        Self {
            on,
            drawn,
            clip: Clip::Sheet,
        }
    }

    /// The line through `points` in `pen`, cut at the sheet's edge.
    #[must_use]
    pub fn line(on: In, points: Vec<Point>, pen: LinePen) -> Self {
        let dashed = false;
        Self::cut(
            on,
            Figure::Line {
                points,
                pen,
                dashed,
            },
        )
    }

    /// The line through `points` in `pen`, dashed, cut at the sheet's edge.
    #[must_use]
    pub fn dashed(on: In, points: Vec<Point>, pen: LinePen) -> Self {
        let dashed = true;
        Self::cut(
            on,
            Figure::Line {
                points,
                pen,
                dashed,
            },
        )
    }

    /// The line through `points` in `pen`, as it lies.
    #[must_use]
    pub fn free_line(on: In, points: Vec<Point>, pen: LinePen) -> Self {
        let dashed = false;
        Self::free(
            on,
            Figure::Line {
                points,
                pen,
                dashed,
            },
        )
    }

    /// The same mark clipped `clip`.
    #[must_use]
    pub fn clipped(self, clip: Clip) -> Self {
        Self { clip, ..self }
    }
}
