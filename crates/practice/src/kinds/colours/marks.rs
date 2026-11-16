//! A colour's marks (S78): its swatch filling the original's frame; compared, each colour as a
//! patch in the middle of the other's frame, and on the card a reason when not right (S138) over
//! both on the wheel and in the colour table (D108).

use super::bands::{NOTICEABLE, PART_SPAN};
use super::colour_match::{ColourCopy, score_colour};
use crate::marks::asked::{Asked, Compare};
use crate::marks::card::{CardItem, Say, WheelSize};
use crate::marks::compared::{Compared, Comparison, Outcome, Seen, Verdict};
use crate::marks::mark::{Figure, In, SheetMark};
use crate::maths::colour::{Difference, Rgb8, oklch};
use crate::records::metric::Metrics;

/// A colour's copy compared, once a colour was laid in its frame: the swatch beside the first and
/// last colours laid, the numbers of the match.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    let original = asked.original.swatch()?;
    let copy = ColourCopy {
        picks: asked.copy.picks(),
    };
    let (&first, &last) = (copy.picks.first()?, copy.picks.last()?);
    let metrics = (score_colour(original, &copy))
        .map(|score| score.metrics())
        .unwrap_or_default();
    Some(Compare::Made(Comparison::of(ColourCompared {
        original,
        first,
        last,
        metrics,
    })))
}

/// The swatch filling the original's frame.
pub(crate) fn givens(colour: Rgb8) -> Vec<SheetMark> {
    vec![SheetMark::free(In::Original, Figure::Swatch(colour))]
}

/// A colour compared: the swatch, the first and last colours laid, and its numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourCompared {
    pub original: Rgb8,
    pub first: Rgb8,
    pub last: Rgb8,
    pub metrics: Metrics,
}

impl Compared for ColourCompared {
    fn marks(&self, _seen: &Seen<'_>) -> Vec<SheetMark> {
        vec![
            SheetMark::free(In::Copy, Figure::Patch(self.original)),
            SheetMark::free(In::Original, Figure::Patch(self.last)),
        ]
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let reason = colour_reason(self.original, self.last).map(CardItem::Says);
        (reason.into_iter())
            .chain([CardItem::Wheel {
                original: self.original,
                copy: self.last,
                size: WheelSize::Large,
            }])
            .collect()
    }

    fn metrics(&self) -> Metrics {
        self.metrics.clone()
    }

    /// The colour laid at this Compare against the swatch, as one Oklab difference: clean within
    /// a just-noticeable step (`NOTICEABLE`), near within two and a half (`PART_SPAN`), else redo.
    /// A first Compare's is the eye's before the answer (D99).
    fn verdict(&self) -> Verdict {
        let off = Difference::of(self.original, self.last).delta;
        Verdict::App(Outcome::within(off, [NOTICEABLE, PART_SPAN]))
    }
}

/// The hues a colour's reason names, at their Oklab hues (degrees).
const HUES: [(f64, &str); 6] = [
    (29.0, "red"),
    (110.0, "yellow"),
    (142.0, "green"),
    (195.0, "cyan"),
    (264.0, "blue"),
    (328.0, "purple"),
];
/// A named hue this near the original's (degrees) is its own: the reason names the next one on.
const OWN_HUE: f64 = 15.0;

/// The reason `copy` gives against `original` (S138), none while they lie within a
/// just-noticeable step (`NOTICEABLE`, the right answer): the first of its hue, lightness and
/// purity off by more than that step, in the order the How corrects them, else the largest of
/// them; a little within `PART_SPAN` (the near band), clearly beyond. Its hue named as the one it
/// turned toward from the original's.
pub(crate) fn colour_reason(original: Rgb8, copy: Rgb8) -> Option<Say> {
    let d = Difference::of(original, copy);
    if d.delta <= NOTICEABLE {
        return None;
    }
    let either = |off: f64, more, less| (off, if off > 0.0 { more } else { less });
    let parts = [
        (d.hue, toward(oklch(original)[2], d.hue > 0.0)),
        either(d.lightness, "light", "dark"),
        either(d.chroma, "pure", "grey"),
    ];
    let largest = (parts.iter()).max_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
    let (off, way) = (parts.iter())
        .find(|(off, _)| off.abs() > NOTICEABLE)
        .or(largest)?;
    Some(Say::reason("colour", way, (*off, PART_SPAN)))
}

/// The named hue nearest on from `from` (degrees) the way red goes to yellow (`on`) or back.
pub(crate) fn toward(from: f64, on: bool) -> &'static str {
    let away = |h: f64| {
        let turn = if on { h - from } else { from - h };
        turn.rem_euclid(360.0)
    };
    (HUES.iter())
        .filter(|(h, _)| away(*h) > OWN_HUE)
        .min_by(|a, b| away(a.0).total_cmp(&away(b.0)))
        .map_or("red", |(_, name)| name)
}
