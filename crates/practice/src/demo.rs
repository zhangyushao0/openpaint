//! Demonstrations (S92): each kind's How shows V大's way played out on an example of the kind, step
//! by step at a hand's pace, then Compare's marks as the exercise shows them. A demonstration is a
//! short timeline of marks in frame units: what the example gives (`Marker::Given`, shown from the
//! start), the pencil of looking and construction, passes in the air, the ink of the answer, and
//! Compare's see-through red answer and blue reading. The screen draws it on its frame clock; the
//! words of each step are the UI's, one per step.
//!
//! The example is never the exercise in hand: each kind's is the clearest on a small card of the
//! first few its generator makes from a fixed seed's sequence (`clearest`: a line long and plainly
//! bent near the middle, a box widest beside its vanishing points with both within `WIDE_REACH`,
//! a face shaded wide), so it is the same every time; a single shape's is a generated vase, so no
//! pack is needed. The marks reuse what the exercises compute: the generators' exact answers, the
//! tricks' constructions, the ellipse's middles.

use crate::colour::Rgb8;
use crate::path::{Point, distance, length};
use crate::plan::Family;

#[path = "demo_colour.rs"]
mod colour;
#[path = "demo_drawn.rs"]
mod drawn;
#[path = "demo_points.rs"]
mod points;
#[path = "demo_space.rs"]
mod space;

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
    /// What the exercise gives (an original's lines, a box's given edges): ink, from the start.
    Given,
    /// Sketch's pencil: looking and construction.
    Pencil,
    /// A pass in the air: a faint trace that goes again.
    Air,
    /// Draw's ink: the answer.
    Ink,
    /// Compare: the original or the exact answer, in see-through red.
    Answer,
    /// Compare: what was read of the copy, in blue.
    Found,
    /// Dashed pencil: a horizon, the way to see it.
    Guide,
    /// Pencil shading: a face, a negative shape looked at.
    Shade,
    /// A thin rule: a cell's edge.
    Rule,
    /// Compare: the original laid faint over the copy.
    Faint,
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
    /// The area between rows of two points, each row joined to the next: a frame's colour, a
    /// cell, a face, a silhouette; in `colour`, or the pen's.
    Band {
        rows: Vec<[Point; 2]>,
        colour: Option<Rgb8>,
    },
    /// The example's colour combination filling its frame, its areas in these colours.
    Areas(Vec<Rgb8>),
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
    pub areas: Option<crate::areas::Areas>,
    /// Its marks in the order drawn, given ones first.
    pub marks: Vec<Mark>,
    /// When each step begins, seconds; a step's words are the UI's.
    pub steps: Vec<f64>,
    /// When it is over: from then on it stands still on its last frame.
    pub length: f64,
}

impl Demo {
    /// `family`'s demonstration; none for the colour copy.
    #[must_use]
    pub fn of(family: Family) -> Option<Self> {
        Some(match family {
            Family::Line => drawn::line(),
            Family::Combined => drawn::combined(),
            Family::Shape => drawn::shape(),
            Family::Colour => colour::colour(),
            Family::Areas => colour::areas(),
            Family::Scale => colour::scale(),
            Family::Box => space::perspective_box(),
            Family::FreeBoxes => space::free_boxes(),
            Family::Ellipse => space::ellipse(),
            Family::Point | Family::Proportion | Family::Angle => points::estimate(family)?,
            Family::Centre | Family::Halves | Family::Thirds | Family::Step | Family::Mirror => {
                points::trick(family)?
            }
            Family::Study => return None,
        })
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
    at: f64,
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
        Demo {
            family,
            layout,
            areas: None,
            marks: self.marks,
            steps: self.steps,
            length: from + SHOWING + HOLD,
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
                Marker::Ink => (1.3, 0.25, 0.9),
                Marker::Pencil | Marker::Guide => (1.2, 0.4, 0.6),
                _ => (2.0, 0.3, 0.5),
            };
            (long / speed).clamp(least, most)
        }
        Drawn::Ring(_) | Drawn::Dot(_) => 0.4,
        Drawn::Cross(_) => 0.7,
        Drawn::Band { .. } | Drawn::Ringed(_) => 0.55,
        Drawn::Areas(_) => 0.65,
        Drawn::Pick(_) => 1.1,
    }
}

/// A short tick across the point `at` of a line heading `towards`, `half` long either side.
pub(crate) fn tick(at: Point, towards: Point, half: f64) -> Vec<Point> {
    let d = [towards[0] - at[0], towards[1] - at[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    let n = [-d[1] / long * half, d[0] / long * half];
    vec![[at[0] - n[0], at[1] - n[1]], [at[0] + n[0], at[1] + n[1]]]
}

/// `points` as a hand copies them: moved off sideways by up to `amount` (frame units) in one
/// smooth swell along the line, the ends near their places.
pub(crate) fn wobbled(points: &[Point], amount: f64, phase: f64) -> Vec<Point> {
    let total = length(points).max(f64::MIN_POSITIVE);
    let mut walked = 0.0;
    let mut out = Vec::with_capacity(points.len());
    for (index, p) in points.iter().enumerate() {
        if index > 0 {
            walked += distance(points[index - 1], *p);
        }
        let (before, after) = (
            points[index.saturating_sub(1)],
            points[(index + 1).min(points.len() - 1)],
        );
        let d = [after[0] - before[0], after[1] - before[1]];
        let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
        let s = walked / total;
        let off = amount * (std::f64::consts::PI * s).sin() * (phase + 2.0 * s).cos();
        out.push([p[0] - d[1] / long * off, p[1] + d[0] / long * off]);
    }
    out
}

/// The example a kind shows: of the first `tries` seeds of `seed`'s sequence, the one whose
/// example `score` finds clearest on a small card (the first of the best), so it is fixed.
pub(crate) fn clearest<T>(
    (seed, tries): (u64, u64),
    made: impl Fn(u64) -> T,
    score: impl Fn(&T) -> f64,
) -> T {
    let mut best = made(crate::random::derived(seed, 0));
    let mut best_score = score(&best);
    for k in 1..tries {
        let example = made(crate::random::derived(seed, k));
        let scored = score(&example);
        if scored > best_score {
            (best, best_score) = (example, scored);
        }
    }
    best
}

/// How far the middle of `points`' bounds lies from the frame's middle.
pub(crate) fn off_middle(points: &[Point]) -> f64 {
    let [mut low, mut high] = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    for p in points {
        for axis in 0..2 {
            low[axis] = low[axis].min(p[axis]);
            high[axis] = high[axis].max(p[axis]);
        }
    }
    distance(
        [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0],
        [0.5, 0.5],
    )
}

/// The line through `a` and `b`, two points.
pub(crate) fn line(a: Point, b: Point) -> Drawn {
    Drawn::Line(vec![a, b])
}
