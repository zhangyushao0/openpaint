//! A key shift's marks (S107): its tendency printed beside the copy's frame as a given. Compared,
//! nothing laid over the copy (a key shift has no answer), only what is off once read: the areas
//! whose lightness left the crop's or that did not move toward the tendency outlined red on the
//! copy, the edge where two areas apart merged red, the area the card shows outlined in ink on both
//! frames; in the Grey view both frames in grey and only the lightness failures outlined. The card
//! (D108, shows rather than tells): a line after a red dot for each check that failed, Grey, the
//! areas' chips (the failing ones outlined red), the hue and chroma disc with Gurney's mask, and the
//! chosen area's lightness bar.

use crate::kinds::key_shifts::key_shift::Tendency;
use crate::kinds::key_shifts::key_shift_check::{KeyCheck, NEAR_AREAS};
use crate::marks::card::{Arg, CardItem, Say};
use crate::marks::compared::{Compared, Outcome, Seen, Verdict};
use crate::marks::mark::{Figure, In, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// The crop in the original's frame, the tendency beside the copy's.
pub(crate) fn givens(tendency: Tendency) -> Vec<SheetMark> {
    let mut marks = crate::marks::drawing::pictured();
    let disc = Figure::Tendency(tendency.colour());
    marks.push(SheetMark::free(In::Copy, disc));
    marks
}

/// A key shift's copy as read: each area's colour in it (none where not painted), what is off, the
/// outlines of the areas marked and the merged pairs' shared edges (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyRead {
    pub colours: Vec<Option<Rgb8>>,
    pub check: KeyCheck,
    pub outlines: Vec<(usize, Vec<Vec<Point>>)>,
    pub edges: Vec<Vec<Point>>,
}

impl KeyRead {
    /// The outline of `area`, if it is marked.
    #[must_use]
    pub fn outline(&self, area: usize) -> Option<&[Vec<Point>]> {
        (self.outlines.iter()).find_map(|(marked, lines)| (*marked == area).then_some(&lines[..]))
    }
}

/// A key shift compared: the crop's areas' colours, where its picture lies in the original's frame
/// (frame units), the tendency, and the copy as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyCompared {
    pub colours: Vec<Rgb8>,
    pub corners: [Point; 4],
    pub tendency: Tendency,
    pub read: Option<KeyRead>,
}

impl Compared for KeyCompared {
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
            let original = picture(Source::GreyOriginal, Place::Corners(self.corners));
            let mut marks = vec![
                SheetMark::free(In::Original, original),
                SheetMark::free(In::Copy, picture(Source::GreyCopy, Place::Frame)),
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
        let mut items = failures(check);
        items.push(CardItem::Grey);
        let count = self.colours.len();
        items.push(CardItem::Chips {
            originals: self.colours.clone(),
            copy: Some(read.colours.clone()),
            marked: (0..count).filter(|area| check.marked(*area)).collect(),
        });
        items.push(CardItem::Disc {
            originals: self.colours.clone(),
            copy: read.colours.clone(),
            tendency: (self.tendency.ab(), self.tendency.colour()),
            mask: self.tendency.mask(),
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

    /// Once read: incomplete while an area is not painted; clean when no area is marked by any of
    /// the three checks, near when `NEAR_AREAS` are, else redo. Unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        let Some(read) = &self.read else {
            return Verdict::Incomplete;
        };
        if read.colours.iter().any(Option::is_none) {
            return Verdict::Incomplete;
        }
        let marked = (0..self.colours.len())
            .filter(|area| read.check.marked(*area))
            .count();
        Verdict::App(Outcome::counting(marked, NEAR_AREAS))
    }
}

/// One line for each check that failed.
fn failures(check: &KeyCheck) -> Vec<CardItem> {
    let (lighter, darker) = check.lighter_darker();
    let (areas, blocks) = check.merged_blocks();
    let off = |count: usize, way: &'static str| Say {
        key: "lightness-off",
        args: vec![Arg::Count(count), Arg::Word(way)],
    };
    let unmoved = check.unmoved.len();
    [
        (lighter > 0).then(|| off(lighter, "lighter")),
        (darker > 0).then(|| off(darker, "darker")),
        (unmoved > 0).then(|| Say::counting("not-toward", &[unmoved])),
        (blocks > 0).then(|| Say::counting("merged", &[areas, blocks])),
    ]
    .into_iter()
    .flatten()
    .map(CardItem::Failed)
    .collect()
}
