//! A single shape compared (S88, S99): its picture faint over the copy (`SHAPE_SHOWN`) with its
//! outline in the marking red, in place or aligned (`Seen::fit`); a picture without an outline lies
//! in place only, and the card says why Aligned is greyed.

use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;

/// How much of a single shape's picture shows under its outline.
pub const SHAPE_SHOWN: f32 = 0.35;

/// A single shape compared: its picture's corners (frame units, its frame's shape) and its
/// outline's lines (none: the picture has no outline).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeCompared {
    pub corners: [Point; 4],
    pub outline: Option<Vec<Vec<Point>>>,
}

impl Compared for ShapeCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(fit) = seen.fit else {
            return Vec::new();
        };
        let picture = Figure::Picture {
            source: Source::Pack,
            place: Place::Corners(self.corners.map(|p| fit.apply(p))),
            shown: SHAPE_SHOWN,
        };
        let outline = self.outline.iter().flatten().map(|line| {
            let laid = line.iter().map(|p| fit.apply(*p)).collect();
            SheetMark::free_line(In::Copy, laid, LinePen::Laid).clipped(Clip::Laid)
        });
        std::iter::once(SheetMark::free(In::Copy, picture).clipped(Clip::Laid))
            .chain(outline)
            .collect()
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let none = self
            .outline
            .is_none()
            .then(|| CardItem::Says(Say::plain("no-outline")));
        std::iter::once(CardItem::Overlaid).chain(none).collect()
    }
}
