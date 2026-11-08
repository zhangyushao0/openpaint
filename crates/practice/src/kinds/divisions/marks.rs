//! A division compared (S95): its picture in place under the copy as strongly as the card's slider
//! sets, the lines as drawn, joined, in their levels' colours as thick as the pen and inside the
//! copy's frame (V大's red, green and blue: no answer, the picture under them is the check). The
//! card: the slider and a table of the three levels, their lines counted and measured.

use crate::kinds::divisions::division::DivisionCheck;
use crate::marks::asked::{Asked, Compare};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// `asked`'s division as drawn in its copy's frame (S95): every line of the levels' pens (those
/// drawn after a Compare too, part of it), joined, counted and measured against its picture's
/// longer side; none without its picture.
pub(crate) fn division(asked: &Asked<'_>) -> Option<DivisionCheck> {
    let picture = asked.picture?;
    let side = picture.fit.size[0].max(picture.fit.size[1]);
    Some(DivisionCheck::of(&asked.copy.division(), side))
}

/// A division's copy compared, once anything lies in its frame: its lines counted, the picture
/// under them (S95).
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let check = division(asked)?;
    let corners = asked.corners()?;
    Some(Compare::Made(Comparison::of(DivisionCompared {
        corners,
        check,
    })))
}

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
