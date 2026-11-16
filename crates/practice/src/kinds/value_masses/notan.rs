//! Notan, value masses in two values (S156, Proko's two-value thumbnails, 黑白构图): each mass
//! decided light or dark, no middle. The answer is the pack's three masses merged either way, the
//! middle into the dark or into the light (the two thresholds between its values); the copy, painted
//! with Dark and Light, is read against both and compared with the one it matches better: that
//! merge's greys laid under the slider (the picture its reading brings), its mass edges red, where
//! the copy differs over more than the pen's width hatched as value masses are, the share matched
//! kept (`Metric::MassesMatched`). Either merge is one good answer among several: the learner's eye
//! judges, as value masses' (S145, D123).

use std::sync::Arc;

use crate::compositions::value_masses::{Value, ValueMasses};
use crate::kinds::value_masses::value_masses_read::MassesCheck;
use crate::marks::asked::{Asked, Compare, Pixels, Read, Reading, Reads};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::drawing::picture_laid;
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// The picture its reading brings: the merge the copy matches better, in its greys.
const MERGED: u8 = 0;

/// `answer`'s masses with its middle ones merged into `into` (the dark or the light): two values.
#[must_use]
pub(crate) fn merged(answer: &ValueMasses, into: Value) -> ValueMasses {
    let middle = Value::Middle.index();
    let values = (answer.values.iter())
        .map(|v| if *v == middle { into.index() } else { *v })
        .collect();
    ValueMasses {
        size: answer.size,
        values,
    }
}

/// Notan's copy compared, once anything lies in its frame: read on the worker against both merges
/// of the answer its picture holds, or with none (a picture removed), the picture laid over it.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let picture = asked.picture?;
    let Some(masses) = picture.masses.as_ref() else {
        return picture_laid(asked, Metrics::new());
    };
    Some(Compare::Read(Reading::of(NotanReading {
        answer: Arc::clone(&masses.answer),
        place: [[0.0, 0.0], picture.fit.size],
        corners: picture.fit.corners(),
    })))
}

/// Notan's copy, read from its frame's pixels on the compare worker: the pack's three-value answer,
/// where the picture lies in the frame (its top left and its size) and its corners (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct NotanReading {
    pub answer: Arc<ValueMasses>,
    pub place: [Point; 2],
    pub corners: [Point; 4],
}

impl Reads for NotanReading {
    fn unread(&self) -> Comparison {
        Comparison::of(NotanCompared {
            corners: self.corners,
            merged: None,
            edges: Vec::new(),
            check: None,
        })
    }

    /// Read against the middle merged into the dark and into the light, the one matched better
    /// kept (the light on a tie).
    fn read(&self, size: [u32; 2], pixels: &[u8], _: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let read = |into: Value| {
            let merge = merged(&self.answer, into);
            let check = merge.read((size, pixels), self.place);
            (into, merge, check)
        };
        let (dark, light) = (read(Value::Dark), read(Value::Light));
        let (into, merge, check) = if dark.2.matched > light.2.matched {
            dark
        } else {
            light
        };
        let [[x, y], [w, h]] = self.place;
        let at = |[u, v]: Point| [x + u * w, y + v * h];
        let edges = (merge.edges().into_iter())
            .map(|edge| edge.into_iter().map(at).collect())
            .collect();
        let metrics = check.metrics.clone();
        let pictures = vec![(merge.size, merge.picture())];
        let compared = NotanCompared {
            corners: self.corners,
            merged: Some(into),
            edges,
            check: Some(check),
        };
        Read {
            comparison: Comparison::of(compared),
            metrics,
            pictures,
        }
    }
}

/// Notan compared: where the picture lies in the copy's frame (its corners), which value the
/// middle was merged into for the answer laid, that merge's mass edges (frame units) and the copy
/// as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct NotanCompared {
    pub corners: [Point; 4],
    pub merged: Option<Value>,
    pub edges: Vec<Vec<Point>>,
    pub check: Option<MassesCheck>,
}

impl Compared for NotanCompared {
    /// Once read: the merge's greys under the slider, the hatching, the mass edges red.
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(check) = &self.check else {
            return Vec::new();
        };
        let picture = Figure::Picture {
            source: Source::Read(MERGED),
            place: Place::Corners(self.corners),
            shown: seen.shown,
        };
        let mut marks = vec![SheetMark::free(In::Copy, picture).clipped(Clip::Laid)];
        marks.extend((check.hatching.iter()).map(|(value, ends)| {
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

    /// Once read, as value masses' (`MassesCheck::verdict`: the eye's, or incomplete while left
    /// bare); unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        (self.check.as_ref()).map_or(Verdict::Incomplete, MassesCheck::verdict)
    }
}
