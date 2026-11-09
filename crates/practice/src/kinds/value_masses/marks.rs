//! Value masses compared (S96): the answer's three greys laid over the picture's place in the copy's
//! frame as strongly as the card's slider sets, where the copy's grey differs from the answer's over
//! more than a pen's width hatched in the grey it should be, and the mass edges in the marking red
//! as thick as the pen and see-through (S88). The card: the slider; until the copy is read, saying
//! so.

use std::sync::Arc;

use crate::compositions::value_masses::ValueMasses;
use crate::kinds::value_masses::value_masses_read::MassesCheck;
use crate::marks::asked::{Asked, Compare, Read, Reading, Reads};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::drawing::picture_laid;
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// Value masses' copy, read from its frame's pixels on the compare worker against the answer: the
/// answer, where the picture lies in the frame (its top left and its size), its corners and its
/// mass edges (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct MassesReading {
    pub answer: Arc<ValueMasses>,
    pub place: [Point; 2],
    pub corners: [Point; 4],
    pub edges: Arc<Vec<Vec<Point>>>,
}

impl Reads for MassesReading {
    fn unread(&self) -> Comparison {
        Comparison::of(MassesCompared {
            corners: self.corners,
            edges: self.edges.to_vec(),
            check: None,
        })
    }

    fn read(&self, size: [u32; 2], pixels: &[u8], _paper: Rgb8) -> Read {
        let check = self.answer.read((size, pixels), self.place);
        let metrics = check.metrics.clone();
        let compared = MassesCompared {
            corners: self.corners,
            edges: self.edges.to_vec(),
            check: Some(check),
        };
        Read {
            comparison: Comparison::of(compared),
            metrics,
        }
    }
}

/// Value masses' copy compared, once anything lies in its frame: read against the answer its
/// picture holds, or with none (a picture removed), the picture laid over it.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let picture = asked.picture?;
    let Some(masses) = picture.masses.as_ref() else {
        return picture_laid(asked, Metrics::new());
    };
    Some(Compare::Read(Reading::of(MassesReading {
        answer: Arc::clone(&masses.answer),
        place: [[0.0, 0.0], picture.fit.size],
        corners: picture.fit.corners(),
        edges: Arc::new(masses.edges.clone()),
    })))
}

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

    /// Once read, its check's (`MassesCheck::verdict`); unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        (self.check.as_ref()).map_or(Verdict::Incomplete, MassesCheck::verdict)
    }
}
