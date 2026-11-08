//! Value masses compared (S96): the answer's three greys laid over the picture's place in the copy's
//! frame as strongly as the card's slider sets, where the copy's grey differs from the answer's over
//! more than a pen's width hatched in the grey it should be, and the mass edges in the marking red
//! as thick as the pen and see-through (S88). The card: the slider; until the copy is read, saying
//! so.

use crate::kinds::value_masses::value_masses_read::MassesCheck;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;

/// Value masses compared: where the picture lies in the copy's frame (its corners, frame units),
/// its mass edges (frame units) and the copy as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct MassesCompared {
    pub corners: [Point; 4],
    pub edges: Vec<Vec<Point>>,
    pub check: Option<MassesCheck>,
}

impl Compared for MassesCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = Figure::Picture {
            source: Source::Painted,
            place: Place::Corners(self.corners),
            shown: seen.shown,
        };
        let mut marks = vec![SheetMark::free(In::Copy, picture).clipped(Clip::Laid)];
        let hatching = self.check.iter().flat_map(|check| &check.hatching);
        marks.extend(hatching.map(|(value, ends)| {
            SheetMark::free_line(In::Copy, ends.to_vec(), LinePen::Hatch(*value))
        }));
        let edges = self.edges.iter();
        marks.extend(edges.map(|edge| SheetMark::free_line(In::Copy, edge.clone(), LinePen::Laid)));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let reading = self.check.is_none().then_some(CardItem::Reading);
        std::iter::once(CardItem::Slider).chain(reading).collect()
    }
}
