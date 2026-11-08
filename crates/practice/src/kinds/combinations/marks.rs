//! A colour combination compared (S82, S101): its picture over the copy in place as strongly as the
//! user sets, the area the card compares outlined in ink on both frames, the edges where the copy
//! made a weak contrast strong in the marking red. The card: the slider, a chip for each area to
//! choose one, the chosen area's colours on the wheel and in the colour table, or the point the
//! eyedropper compared; a line where the area is not painted. A generated one's (a record of 1.18.1
//! and before) shows its picture drawn again in its original's frame.

use crate::colour_areas::areas_read::AreasCopy;
use crate::marks::card::{CardItem, Say, WheelSize};
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Figure, In, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// A generated combination's picture, drawn again, in its original's frame.
pub(crate) fn generated_givens() -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    vec![SheetMark::free(In::Original, picture)]
}

/// A colour combination's copy as read at the last Compare, and the edges where it made a weak
/// contrast strong (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasRead {
    pub copy: AreasCopy,
    pub strong: Vec<Vec<Point>>,
}

/// A colour combination compared: its areas' colours and the copy as read (none until its pixels
/// are).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasCompared {
    pub colours: Vec<Rgb8>,
    pub read: Option<AreasRead>,
}

impl Compared for AreasCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = Figure::Picture {
            source: Source::PaintedOrPack,
            place: Place::Frame,
            shown: seen.shown,
        };
        let chosen = |on| {
            let lines = seen.outline.to_vec();
            SheetMark::free(on, Figure::Haloed { lines, red: false })
        };
        let mut marks = vec![
            SheetMark::free(In::Copy, picture),
            chosen(In::Original),
            chosen(In::Copy),
        ];
        if let Some(read) = &self.read {
            let lines = read.strong.clone();
            marks.push(SheetMark::free(
                In::Copy,
                Figure::Haloed { lines, red: true },
            ));
        }
        marks
    }

    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem> {
        let copy = self.read.as_ref().map(|read| read.copy.colours.clone());
        let chips = CardItem::Chips {
            originals: self.colours.clone(),
            copy,
            marked: Vec::new(),
        };
        let wheel = |(original, copy)| CardItem::Wheel {
            original,
            copy,
            size: WheelSize::Small,
        };
        let last = match (seen.probe, &self.read) {
            (Some(probe), _) => wheel(probe),
            (None, None) => CardItem::Reading,
            (None, Some(read)) => match read.copy.colours.get(seen.chosen).copied().flatten() {
                Some(mine) => wheel((self.colours[seen.chosen], mine)),
                None => CardItem::Says(Say::plain("not-painted")),
            },
        };
        vec![CardItem::Slider, chips, last]
    }
}
