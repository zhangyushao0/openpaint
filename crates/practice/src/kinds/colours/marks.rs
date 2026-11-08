//! A colour's marks (S78): its swatch filling the original's frame; compared, each colour as a
//! patch in the middle of the other's frame, and on the card both on the wheel and in the colour
//! table (D108).

use super::colour_match::{ColourCopy, score_colour};
use crate::marks::asked::{Asked, Compare};
use crate::marks::card::{CardItem, WheelSize};
use crate::marks::compared::{Compared, Comparison, Seen};
use crate::marks::mark::{Figure, In, SheetMark};
use crate::maths::colour::Rgb8;
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
        vec![CardItem::Wheel {
            original: self.original,
            copy: self.last,
            size: WheelSize::Large,
        }]
    }

    fn metrics(&self) -> Metrics {
        self.metrics.clone()
    }
}
