//! Demonstrations (S92): each kind's How shows V大's way played out on an example of the kind, step
//! by step at a hand's pace, then Compare's marks as the exercise shows them. A demonstration is a
//! short timeline of marks in frame units: what the example gives (`Marker::Given`, shown from the
//! start), the pencil of looking and construction, passes in the air, the ink of the answer, and
//! Compare's see-through red answer with only what is off marked (S97, S98: the exercise's own
//! check on the example's ink, `off_marks`). The screen draws it on its frame clock; the words of
//! each step are the UI's, one per step.
//!
//! The example is never the exercise in hand: each kind's is the clearest on a small card of the
//! first few its generator makes from a fixed seed's sequence (`clearest`: a line long and plainly
//! bent near the middle, a box widest beside its vanishing points with both within `WIDE_REACH`,
//! a face shaded wide), so it is the same every time; a single shape's is a generated vase, so no
//! pack is needed. The marks reuse what the exercises compute: the generators' exact answers, the
//! tricks' constructions, the ellipse's middles, the solids' exact outlines (S93), an object's
//! picture, construction and visible edges (S94), a landscape's picture, its division in V大's
//! three levels and its line drawing (S95), its value masses and a copy of them read (S96).

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::compositions::levels::Level;
use crate::compositions::value_masses::{Value, ValueMasses};
use crate::maths::colour::Rgb8;
use crate::maths::path::{Point, length};
use crate::sessions::plan::Family;

// Each kind's How lives in its kind's folder (`kinds/`); the hand they share is here.
#[path = "demo_hand.rs"]
mod hand;

pub(crate) use hand::{
    DRAWN_OFF, TICK, band, bounds, clearest, line, off_marks, off_middle, rect, seen, tick, wobbled,
};

/// A picture an example shows that its kind draws (an object's, S94), made when first shown.
pub trait Pictured: Any + Debug + Send + Sync {
    /// Its pixels at `side` pixels square: 8-bit RGBA, straight alpha.
    fn picture(&self, side: u32) -> Vec<u8>;
    /// Whether `other` is the same picture.
    fn same(&self, other: &dyn Any) -> bool;
}

/// The picture a `Drawn::Shaded` mark lays, held; equal when it is the same picture.
#[derive(Debug, Clone)]
pub struct Shaded(pub Arc<dyn Pictured>);

impl PartialEq for Shaded {
    fn eq(&self, other: &Self) -> bool {
        let other: &dyn Any = &*other.0;
        self.0.same(other)
    }
}

impl Shaded {
    /// It as its kind's own picture `T`, if it is one (the tests read it so).
    #[must_use]
    pub fn downcast<T: Pictured>(&self) -> Option<&T> {
        let any: &dyn Any = &*self.0;
        any.downcast_ref::<T>()
    }
}

/// What a demonstration shows under its frames (a key shift's, S107): a chip of the colour being
/// painted (its colour and the colour it goes to) from when its painting begins, and a disc of
/// the colour aimed at under the copy, ringed in pencil from `looked` on.
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    pub disc: Rgb8,
    pub looked: f64,
    pub chips: Vec<(f64, [Rgb8; 2])>,
}

/// Seconds before the first mark (the card grows meanwhile), between marks and between steps (a
/// moment to read the step's words before it draws).
const LEAD: f64 = 0.7;
const GAP: f64 = 0.1;
const BETWEEN: f64 = 0.6;
/// How long Compare's marks take to show, and a still moment after them before the end.
const SHOWING: f64 = 0.6;
const HOLD: f64 = 0.5;
/// How far beyond its frame's sides a wide example's vanishing points may lie (frame units), so the
/// box still reads large once the card fits it with them.
pub const WIDE_REACH: f64 = 0.85;

/// How the example lies in the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// The original's frame and the copy's side by side.
    Pair,
    /// The copy's frame alone, what the exercise gives lying in it: the example shown as large as
    /// the card lets it.
    Single,
    /// As `Single`, its vanishing points on the horizon beside it shown too (`WIDE_REACH`).
    Wide,
    /// The original's frame, the copy's and a colour wheel.
    Wheel,
    /// The copy's frame cropped to its row (`Demo::view`) and a colour wheel: a value scale's, one
    /// frame (S97).
    Row,
}

/// Where a mark lies: in a frame (frame units, a single frame's is `Copy`) or on the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    Original,
    Copy,
    Wheel,
}

/// How a mark is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// What the exercise gives (an original's lines): ink, from the start.
    Given,
    /// What a kind built from given parts gives in its one frame (a box's given edges, an
    /// ellipse's frame, a trick's shaded face): printed grey, from the start (S97).
    Printed,
    /// What it gives that the eye cannot see (a solid's box's hidden edges): printed grey, dashed
    /// (S98).
    PrintedHidden,
    /// Sketch's pencil: looking and construction.
    Pencil,
    /// Sketch's pencil finding an object's points (S94): dark while its step draws, faint once
    /// the next begins, so the step in hand reads apart from those before it.
    Construction,
    /// A pass in the air: a faint trace that goes again.
    Air,
    /// Draw's ink: the answer.
    Ink,
    /// Compare: the original or the exact answer, in see-through red.
    Answer,
    /// Compare: the exact answer where the solid hides it, dashed in see-through red (S93).
    Hidden,
    /// Compare: an edge that misses where it heads drawn on past it, dashed red (S97, S98).
    Missed,
    /// Dashed pencil: a horizon, the way to see it.
    Guide,
    /// Pencil shading: a face, a negative shape looked at.
    Shade,
    /// A thin rule: a cell's edge.
    Rule,
    /// Compare: the original laid faint over the copy.
    Faint,
    /// A division's pen of its level, in V大's colour (S95).
    Level(Level),
    /// Compare: hatching where a copy's value differs from the answer's, in the answer's grey
    /// (S96).
    Hatch(Value),
}

/// What a mark draws.
#[derive(Debug, Clone, PartialEq)]
pub enum Drawn {
    /// A line, drawn along its length.
    Line(Vec<Point>),
    Ring(Point),
    /// A small cross, an arm after the other.
    Cross(Point),
    Dot(Point),
    /// A small number centred there: a cell's place in the order to paint (S97).
    Numeral(Point, u8),
    /// The area between rows of two points, each row joined to the next: a frame's colour, a
    /// cell, a face, a silhouette; in `colour`, or the pen's.
    Band {
        rows: Vec<[Point; 2]>,
        colour: Option<Rgb8>,
    },
    /// The example's colour combination filling its frame, its areas in these colours, its edges
    /// soft where the picture's are or (`soft` false) all hard, as the hard tip lays them (S100).
    Areas {
        colours: Vec<Rgb8>,
        soft: bool,
    },
    /// The example object's picture filling its frame (S94).
    Shaded,
    /// The example landscape's picture between its top left and bottom right (frame units, S95).
    Masses([Point; 2]),
    /// The example's value masses there (S96), the hand's copy's or the answer's, in their greys
    /// where `shown` (dark, middle, light), the paper elsewhere.
    Values {
        rect: [Point; 2],
        copy: bool,
        shown: [bool; 3],
    },
    /// The wheel's pick moved to this colour (hue in degrees, saturation, value).
    Pick([f64; 3]),
    /// A colour ringed on the wheel (hue in degrees, saturation, value), in the pen's colour.
    Ringed([f64; 3]),
}

/// A mark of a demonstration: drawn between `from` and `to` seconds after it starts (a given
/// mark at 0).
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub on: On,
    pub pen: Marker,
    pub drawn: Drawn,
    pub from: f64,
    pub to: f64,
}

/// A kind's demonstration.
#[derive(Debug, Clone, PartialEq)]
pub struct Demo {
    pub family: Family,
    pub layout: Layout,
    /// The colour combination its `Drawn::Areas` marks paint (a combination's only).
    pub areas: Option<crate::colour_areas::areas_generated::GeneratedAreas>,
    /// The picture its `Drawn::Shaded` mark shows (an object's, S94).
    pub shaded: Option<Shaded>,
    /// The landscape its `Drawn::Masses` marks show (a division's and a whole copy's, S95).
    pub masses: Option<Box<crate::compositions::masses::Masses>>,
    /// A whole copy's division (S95): its marks dim from the first time on (Draw chosen) and are
    /// gone from the second (Compare hides them).
    pub dimmed: Option<[f64; 2]>,
    /// The value masses its `Drawn::Values` marks show (S96): the answer's, then the hand's copy's.
    pub values: Option<Box<[ValueMasses; 2]>>,
    /// What shows under its frames (a key shift's chips and tendency, S107).
    pub strip: Option<Box<Strip>>,
    /// Its marks in the order drawn, given ones first.
    pub marks: Vec<Mark>,
    /// When each step begins, seconds; a step's words are the UI's.
    pub steps: Vec<f64>,
    /// When it is over: from then on it stands still on its last frame.
    pub length: f64,
    /// The part of the copy's frame a lone example shows, as large as the card lets it: its top
    /// left and bottom right (frame units); what lies beyond is cut at the picture's edge. None
    /// for frames side by side, shown whole, but an object's (S94): both cropped to its box; a
    /// division's, a whole copy's and value masses' to their picture, the frames its shape (S99);
    /// a value scale's frame cropped to its rows (S97).
    pub view: Option<[Point; 2]>,
}

impl Demo {
    /// `family`'s demonstration; none for the colour copy.
    #[must_use]
    pub fn of(family: Family) -> Option<Self> {
        family.described().how(family)
    }

    /// `family`'s demonstration for an exercise of `cells` cells in hand (S97: a value scale's
    /// steps name that row's cells; other kinds' are the same whatever it holds).
    #[must_use]
    pub fn of_cells(family: Family, cells: usize) -> Option<Self> {
        family.described().how_of(family, cells)
    }

    /// The step being drawn `seconds` after the start (the first before it begins, the last once
    /// it is over).
    #[must_use]
    pub fn step_at(&self, seconds: f64) -> usize {
        let begun = self.steps.iter().filter(|start| **start <= seconds).count();
        begun.saturating_sub(1)
    }
}

/// A demonstration as it is written: marks one after another at a hand's pace, step by step.
pub(crate) struct Script {
    marks: Vec<Mark>,
    steps: Vec<f64>,
    pub(crate) at: f64,
}

impl Script {
    pub(crate) const fn new() -> Self {
        Self {
            marks: Vec::new(),
            steps: Vec::new(),
            at: LEAD,
        }
    }

    /// A mark shown from the start: what the exercise gives.
    pub(crate) fn given(&mut self, on: On, pen: Marker, drawn: Drawn) {
        self.marks.push(Mark {
            on,
            pen,
            drawn,
            from: 0.0,
            to: 0.0,
        });
    }

    /// The next step begins.
    pub(crate) fn step(&mut self) {
        if !self.steps.is_empty() {
            self.at += BETWEEN - GAP;
        }
        self.steps.push(self.at);
    }

    /// A mark drawn after the last, at its pen's pace.
    pub(crate) fn draw(&mut self, on: On, pen: Marker, drawn: Drawn) {
        self.together(vec![(on, pen, drawn)]);
    }

    /// Marks drawn at once (both hands of a construction, a fill and its outline), the next
    /// after the slowest.
    pub(crate) fn together(&mut self, marks: Vec<(On, Marker, Drawn)>) {
        let from = self.at;
        let mut end = from;
        for (on, pen, drawn) in marks {
            let to = from + seconds(pen, &drawn);
            end = end.max(to);
            self.marks.push(Mark {
                on,
                pen,
                drawn,
                from,
                to,
            });
        }
        self.at = end + GAP;
    }

    /// The last step: Compare, its marks shown at once; then the end.
    pub(crate) fn compare(
        mut self,
        family: Family,
        layout: Layout,
        marks: Vec<(On, Marker, Drawn)>,
    ) -> Demo {
        self.step();
        let from = self.at;
        for (on, pen, drawn) in marks {
            self.marks.push(Mark {
                on,
                pen,
                drawn,
                from,
                to: from + SHOWING,
            });
        }
        let view = matches!(layout, Layout::Single | Layout::Wide).then(|| seen(&self.marks));
        Demo {
            family,
            layout,
            areas: None,
            shaded: None,
            masses: None,
            dimmed: None,
            values: None,
            strip: None,
            marks: self.marks,
            steps: self.steps,
            length: from + SHOWING + HOLD,
            view: view.flatten(),
        }
    }
}

/// How long `drawn` takes in `pen` at a hand's pace: a line by its length (a pass in the air and
/// a sure stroke quicker than looking with the pencil), a long construction line no longer than
/// a moment.
fn seconds(pen: Marker, drawn: &Drawn) -> f64 {
    match drawn {
        Drawn::Line(points) => {
            let long = length(points);
            let (speed, least, most) = match pen {
                Marker::Air => (1.3, 0.5, 0.9),
                Marker::Ink | Marker::Level(_) => (1.3, 0.25, 0.9),
                Marker::Pencil | Marker::Construction | Marker::Guide => (1.2, 0.4, 0.6),
                _ => (2.0, 0.3, 0.5),
            };
            (long / speed).clamp(least, most)
        }
        Drawn::Ring(_) | Drawn::Dot(_) | Drawn::Numeral(..) => 0.4,
        Drawn::Cross(_) => 0.7,
        Drawn::Band { .. } | Drawn::Ringed(_) => 0.55,
        Drawn::Areas { .. } | Drawn::Shaded | Drawn::Masses(_) => 0.65,
        Drawn::Values { .. } => 1.2,
        Drawn::Pick(_) => 1.1,
    }
}
