//! A colour's marks (S78): its swatch filling the original's frame; compared, each colour as a
//! patch in the middle of the other's frame, and on the card both on the wheel and in the colour
//! table (D108).

use crate::marks::card::{CardItem, WheelSize};
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Figure, In, SheetMark};
use crate::maths::colour::Rgb8;
use crate::records::metric::Metrics;

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
