//! A gamut mask's marks (S153): beside the copy's frame the mask on a small hue and chroma disc,
//! and under it its corners' colours as swatches (Gurney's premixed primaries, picked whole by the
//! eyedropper: they are the exercise's, not the original's). Compared, nothing laid over the copy:
//! the areas off outlined red on it (outside the mask or their lightness left), the edge where two
//! areas apart merged red, the area the card shows outlined in ink on both frames; in the Grey
//! view both frames in grey and only the lightness failures outlined. The card (D108): the reason
//! (D121), Grey, the areas' chips (those off outlined red), the disc with the mask (each crop
//! colour a ring, the copy's a dot, an area outside ringed red) and the chosen area's lightness
//! bar. The app judges (D123): a colour inside a shape is a direct measure.

use super::check::MaskCheck;
use super::mask::Mask;
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::mark::{Figure, In, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// The pictures its reading brings (`reading.rs`), by their place: the copy in grey, then the
/// crop in grey where it was read.
pub(crate) const GREY_COPY: u8 = 0;
pub(crate) const GREY_ORIGINAL: u8 = 1;
/// Beside the copy's frame (frame units from its top right corner, a lesson's frame 400 canvas
/// pixels long): the mask's disc at the top, its centre and radius, then its corners' swatches
/// down under it, the first's centre and the step to the next.
const DISC_AT: Point = [0.115, 0.075];
const DISC_RADIUS: f64 = 0.075;
const FIRST_SWATCH: f64 = 0.215;
const SWATCH_STEP: f64 = 0.09;

/// The crop in the original's frame; the mask and its corners' swatches beside the copy's.
pub(crate) fn givens(mask: Mask) -> Vec<SheetMark> {
    let mut marks = crate::marks::drawing::pictured();
    let disc = Figure::Gamut {
        at: DISC_AT,
        radius: DISC_RADIUS,
        corners: mask.corners(),
    };
    marks.push(SheetMark::free(In::Copy, disc));
    for (k, colour) in mask.swatches().into_iter().enumerate() {
        let swatch = Figure::Beside {
            at: [DISC_AT[0], FIRST_SWATCH + SWATCH_STEP * k as f64],
            colour,
            name: None,
        };
        marks.push(SheetMark::free(In::Copy, swatch));
    }
    marks
}

/// A gamut mask's copy as read: each area's colour in it (none where not painted), what is off,
/// the outlines of the areas off and the merged pairs' shared edges (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskRead {
    pub colours: Vec<Option<Rgb8>>,
    pub check: MaskCheck,
    pub outlines: Vec<(usize, Vec<Vec<Point>>)>,
    pub edges: Vec<Vec<Point>>,
}

impl MaskRead {
    /// The outline of `area`, if it is off.
    #[must_use]
    pub fn outline(&self, area: usize) -> Option<&[Vec<Point>]> {
        (self.outlines.iter()).find_map(|(marked, lines)| (*marked == area).then_some(&lines[..]))
    }
}

/// A gamut mask compared: the crop's areas' colours, where its picture lies in the original's
/// frame (frame units), the mask, and the copy as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskCompared {
    pub colours: Vec<Rgb8>,
    pub corners: [Point; 4],
    pub mask: Mask,
    pub read: Option<MaskRead>,
}

impl Compared for MaskCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(read) = &self.read else {
            return Vec::new();
        };
        let red = |lines: &[Vec<Point>]| {
            let lines = lines.to_vec();
            SheetMark::free(In::Copy, Figure::Haloed { lines, red: true })
        };
        if seen.grey {
            let picture = |source, place| Figure::Picture {
                source,
                place,
                shown: 1.0,
            };
            let original = picture(Source::Read(GREY_ORIGINAL), Place::Corners(self.corners));
            let mut marks = vec![
                SheetMark::free(In::Original, original),
                SheetMark::free(In::Copy, picture(Source::Read(GREY_COPY), Place::Frame)),
            ];
            let off = read.check.off_lightness.iter();
            marks.extend(off.map(|area| red(read.outline(*area).unwrap_or_default())));
            return marks;
        }
        let chosen = |on| {
            let lines = seen.outline.to_vec();
            SheetMark::free(on, Figure::Haloed { lines, red: false })
        };
        let mut marks = vec![chosen(In::Original), chosen(In::Copy)];
        marks.extend(read.outlines.iter().map(|(_, lines)| red(lines)));
        marks.push(red(&read.edges));
        marks
    }

    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem> {
        let Some(read) = &self.read else {
            return vec![CardItem::Reading];
        };
        let check = &read.check;
        let reason = self.reason().map(|(_, say)| CardItem::Says(say));
        let mut items: Vec<CardItem> = reason.into_iter().collect();
        items.push(CardItem::Grey);
        let count = self.colours.len();
        items.push(CardItem::Chips {
            originals: self.colours.clone(),
            copy: Some(read.colours.clone()),
            marked: (0..count).filter(|area| check.marked(*area)).collect(),
        });
        items.push(CardItem::Gamut {
            originals: self.colours.clone(),
            copy: read.colours.clone(),
            corners: self.mask.corners(),
            marked: check.outside.iter().map(|(area, _)| *area).collect(),
        });
        let original = self.colours.get(seen.chosen).copied();
        items.push(
            match (original, read.colours.get(seen.chosen).copied().flatten()) {
                (Some(original), Some(copy)) => CardItem::Bar { original, copy },
                _ => CardItem::Says(Say::plain("not-painted")),
            },
        );
        items
    }

    /// Once read: incomplete while an area is not painted, else the app's by the findings
    /// (`MaskCheck::outcome`). Unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        let Some(read) = &self.read else {
            return Verdict::Incomplete;
        };
        if read.colours.iter().any(Option::is_none) {
            return Verdict::Incomplete;
        }
        Verdict::App(read.check.outcome())
    }

    /// The area the reason names: the card opens on its chip (S139).
    fn named_area(&self) -> Option<usize> {
        self.reason().map(|(area, _)| area)
    }
}

impl MaskCompared {
    /// The reason beside a result not clean (D121) and the area it names; none while an area is
    /// not painted or nothing is off.
    #[must_use]
    pub fn reason(&self) -> Option<(usize, Say)> {
        let read = self.read.as_ref()?;
        if !matches!(self.verdict(), Verdict::App(_)) {
            return None;
        }
        read.check.reason(&read.colours, self.mask)
    }
}
