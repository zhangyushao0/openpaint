//! A division compared (S95): its picture in place under the copy as strongly as the card's slider
//! sets, the lines as drawn, joined, in their levels' colours as thick as the pen and inside the
//! copy's frame (V大's red, green and blue: no answer, the picture under them is the check). The
//! card: the slider and a table of the three levels, their lines counted and measured.

use crate::kinds::divisions::division::DivisionCheck;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// A division compared: its picture's corners (frame units, its frame's shape) and its lines read.
#[derive(Debug, Clone, PartialEq)]
pub struct DivisionCompared {
    pub corners: [Point; 4],
    pub check: DivisionCheck,
}

impl Compared for DivisionCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = Figure::Picture {
            source: Source::Pack,
            place: Place::Corners(self.corners),
            shown: seen.shown,
        };
        let lines = self.check.lines.iter().map(|line| SheetMark {
            on: In::Copy,
            drawn: Figure::Line {
                points: line.points.clone(),
                pen: LinePen::Level(line.level),
                dashed: false,
            },
            clip: Clip::Inside,
        });
        std::iter::once(SheetMark::free(In::Copy, picture).clipped(Clip::Laid))
            .chain(lines)
            .collect()
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Slider, CardItem::Levels(self.check.rows)]
    }

    fn metrics(&self) -> Metrics {
        self.check.metrics.clone()
    }
}
