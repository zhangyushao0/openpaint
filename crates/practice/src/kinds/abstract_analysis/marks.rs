//! Abstract analysis's card and Compare (S112). Before Compare the card asks V大's four steps: what
//! draws the eye most (ringed with 强), by what contrast (`CONTRASTS`, several chips), the second
//! and the weak (次 ringed, 弱 hatched), and how strength is spread (`DISTRIBUTIONS`, four small bar
//! pictures, one chosen); what was chosen is kept with the first Compare (`Choices`). Compare is
//! read on the worker from the picture's own pixels: its five views (`VIEWS`, as
//! `ContrastView::ALL`), each laid over the picture as its reading brought it (a veil of paper
//! where that contrast is weak, or the picture squinted), the marks redrawn over it in their pens'
//! colours as a division's are, along the curve the canvas drew them on (`curved`, S158k); the
//! card the views, one shown, and what was chosen in the chips'
//! words. Nothing is scored: the eye judges once a 强 mark lies in the frame.

use super::contrast::{ContrastView, Contrasts};
use crate::compositions::levels::Level;
use crate::marks::asked::{Asked, Compare, Pixels, Read, Reading, Reads};
use crate::marks::card::{Arg, CardItem, Choices, Say};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::{Point, distance};
use crate::records::metric::Metrics;

/// The contrasts the strongest place may live by, as the card's chips show them: 明度, 虚实, 纯度,
/// 色相, 大小, 隔离 (V大's second step).
pub const CONTRASTS: [&str; 6] = ["lightness", "edges", "chroma", "hue", "size", "isolation"];
/// How strength is spread (V大's four distributions), each with its bars' heights as the card
/// draws them: even, falling evenly, one far above the rest, one above a weaker rhythm.
pub const DISTRIBUTIONS: [(&str, [f32; 5]); 4] = [
    ("even", [0.55, 0.55, 0.55, 0.55, 0.55]),
    ("linear", [0.9, 0.72, 0.55, 0.36, 0.18]),
    ("curved", [1.0, 0.36, 0.23, 0.18, 0.14]),
    ("noisy", [1.0, 0.41, 0.64, 0.27, 0.5]),
];
/// Compare's views, as `ContrastView::ALL`: 明度, 纯度, 色相, 虚实, 眯眼.
pub const VIEWS: [&str; 5] = ["lightness", "chroma", "hue", "edges", "squint"];

/// The card's four steps while the copy is made.
pub(crate) fn task() -> Vec<CardItem> {
    let say = |key| CardItem::Says(Say::plain(key));
    vec![
        say("analysis-strongest"),
        say("analysis-strongest-how"),
        say("analysis-by"),
        CardItem::Toggles(&CONTRASTS),
        say("analysis-second"),
        say("analysis-second-how"),
        say("analysis-spread"),
        CardItem::Shapes(&DISTRIBUTIONS),
    ]
}

/// `asked`'s marks compared, once anything lies in its frame: read on the worker, its picture's
/// views made from the picture's pixels.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let corners = asked.corners()?;
    Some(Compare::Read(Reading::of(AnalysisReading {
        lines: asked.copy.division(),
        corners,
        choices: asked.choices,
    })))
}

/// An abstract analysis compared: its marks by their pens' levels (frame units), where its picture
/// lies in the frame, what the card's choices held, and the reading: none until it is read, then
/// whether the picture was there to make its views of.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisReading {
    pub lines: Vec<(Level, Vec<Point>)>,
    pub corners: [Point; 4],
    pub choices: Choices,
}

impl AnalysisReading {
    fn compared(&self, read: Option<bool>) -> AnalysisCompared {
        AnalysisCompared {
            lines: self.lines.clone(),
            corners: self.corners,
            choices: self.choices,
            read,
        }
    }
}

impl Reads for AnalysisReading {
    fn unread(&self) -> Comparison {
        Comparison::of(self.compared(None))
    }

    /// The picture's views, in `VIEWS`' order, from its pixels as read for the screen; none
    /// where it was not read. The copy's own pixels are the marks over it: nothing to read.
    fn read(&self, _: [u32; 2], _: &[u8], _: Rgb8, original: Option<Pixels<'_>>) -> Read {
        let pictures: Vec<_> = original.map_or_else(Vec::new, |(size, rgba)| {
            let contrasts = Contrasts::of(size, rgba);
            (ContrastView::ALL.iter())
                .map(|view| (contrasts.size, contrasts.picture(*view)))
                .collect()
        });
        Read {
            comparison: Comparison::of(self.compared(Some(!pictures.is_empty()))),
            metrics: self.choices.metrics(),
            pictures,
        }
    }
}

/// An abstract analysis compared (see `AnalysisReading`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisCompared {
    pub lines: Vec<(Level, Vec<Point>)>,
    pub corners: [Point; 4],
    pub choices: Choices,
    pub read: Option<bool>,
}

impl Compared for AnalysisCompared {
    /// The view shown laid over the picture once read, then the marks over it.
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if self.read == Some(true) {
            let view = seen.view.min(VIEWS.len() - 1);
            let picture = Figure::Picture {
                source: Source::Read(u8::try_from(view).unwrap_or(0)),
                place: Place::Corners(self.corners),
                shown: 1.0,
            };
            marks.push(SheetMark::free(In::Copy, picture).clipped(Clip::Laid));
        }
        marks.extend(self.lines.iter().map(|(level, points)| SheetMark {
            on: In::Copy,
            drawn: Figure::Line {
                points: curved(points),
                pen: LinePen::Level(*level),
                dashed: false,
            },
            clip: Clip::Inside,
        }));
        marks
    }

    /// The views once read, then what the card's choices held.
    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let mut items = match self.read {
            None => vec![CardItem::Reading],
            Some(true) => vec![
                CardItem::Says(Say::plain("analysis-look")),
                CardItem::Views(&VIEWS),
            ],
            Some(false) => Vec::new(),
        };
        let chips = (0..CONTRASTS.len()).filter(|chip| {
            let chip = u8::try_from(*chip).unwrap_or(u8::MAX);
            self.choices.on(chip)
        });
        let chosen = Say {
            key: "analysis-chosen",
            args: chips.map(|chip| Arg::Word(CONTRASTS[chip])).collect(),
        };
        let shape = (self.choices.shape).and_then(|shape| DISTRIBUTIONS.get(usize::from(shape)));
        let spread = Say {
            key: "analysis-spread-chosen",
            args: shape.map(|(word, _)| Arg::Word(word)).into_iter().collect(),
        };
        items.extend([CardItem::Says(chosen), CardItem::Says(spread)]);
        items
    }

    fn metrics(&self) -> Metrics {
        self.choices.metrics()
    }

    /// The eye judges once the strongest place is ringed; before, no verdict is honest.
    fn verdict(&self) -> Verdict {
        if self.lines.iter().any(|(level, _)| *level == Level::Main) {
            Verdict::Eye
        } else {
            Verdict::Incomplete
        }
    }
}

/// A redrawn mark's pieces at most this long, frame units.
const CURVE_STEP: f64 = 0.004;

/// The curve a stroke through the pen's samples `points` follows on the canvas, in straight pieces
/// at most `CURVE_STEP` long (S158k): the renderer's centripetal Catmull-Rom spline through every
/// sample, a mirrored one standing in past either end, so a mark redrawn over a view is as smooth
/// as it was drawn, however few samples a quick ring left.
fn curved(points: &[Point]) -> Vec<Point> {
    let Some(&first) = points.first() else {
        return Vec::new();
    };
    let mirrored =
        |about: Point, other: Point| [2.0 * about[0] - other[0], 2.0 * about[1] - other[1]];
    let mut out = vec![first];
    for (j, pair) in points.windows(2).enumerate() {
        let [b, c] = [pair[0], pair[1]];
        let a = j
            .checked_sub(1)
            .map_or_else(|| mirrored(b, c), |k| points[k]);
        let d = points.get(j + 2).copied().unwrap_or_else(|| mirrored(c, b));
        let span = centripetal([a, b, c, d]);
        let pieces = (distance(b, c) / CURVE_STEP).ceil().clamp(1.0, 64.0);
        out.extend((1..=pieces as usize).map(|k| bezier(&span, k as f64 / pieces)));
    }
    out
}

/// The centripetal Catmull-Rom span from `p[1]` to `p[2]` as a cubic Bézier (Barry and Goldman's
/// tangents, knot steps the square roots of the chords), as `render/src/curve` builds it.
fn centripetal([p0, p1, p2, p3]: [Point; 4]) -> [Point; 4] {
    let [a, b, c] = [(p0, p1), (p1, p2), (p2, p3)].map(|(p, q)| distance(p, q).sqrt());
    if b < 1e-6 {
        return [p1, p1, p2, p2];
    }
    let tangent = |[p, q, r]: [Point; 3], [before, after]: [f64; 2]| -> Point {
        if before < 1e-6 {
            return [0.0; 2];
        }
        [0, 1].map(|k| {
            let slope =
                (q[k] - p[k]) / before - (r[k] - p[k]) / (before + after) + (r[k] - q[k]) / after;
            slope * b
        })
    };
    let start = tangent([p0, p1, p2], [a, b]);
    let end = if c < 1e-6 {
        [0.0; 2]
    } else {
        tangent([p1, p2, p3], [b, c])
    };
    let along = |p: Point, t: Point, by: f64| [p[0] + t[0] * by, p[1] + t[1] * by];
    [
        p1,
        along(p1, start, 1.0 / 3.0),
        along(p2, end, -1.0 / 3.0),
        p2,
    ]
}

/// The point of the cubic Bézier `span` at `t`.
fn bezier(span: &[Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    let weights = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
    [0, 1].map(|k| weights.iter().zip(span).map(|(w, p)| w * p[k]).sum())
}
