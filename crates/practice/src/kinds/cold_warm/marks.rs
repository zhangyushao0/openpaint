//! A cold-warm copy compared (S154), as a long colour copy is with a key shift's Grey view: the
//! picture laid over the copy as strongly as the slider sets; once its frames are read, both in
//! grey on asking (lightness is what a shift keeps), the picture in grey laid over the copy in
//! grey the same way; the point the eyedropper compared on the hue and chroma disc beside the two
//! tendencies, with its lightness bar. No result is asked or kept: the eye looks (D123).

use super::leans::Leans;
use crate::marks::asked::{Pixels, Read, Reads};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, Place, SheetMark, Source};
use crate::maths::colour::{Rgb8, in_grey};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// The pictures its reading brings, by their place: the copy in grey, then the picture in grey
/// where it was read.
const GREY_COPY: u8 = 0;
const GREY_PICTURE: u8 = 1;

/// A cold-warm copy to be read: where its picture lies in the frame (frame units) and its pair.
#[derive(Debug, Clone, PartialEq)]
pub struct ShiftReading {
    pub corners: [Point; 4],
    pub leans: Leans,
}

impl ShiftReading {
    fn compared(&self, read: bool) -> ShiftCompared {
        ShiftCompared {
            corners: self.corners,
            leans: self.leans,
            read,
        }
    }
}

impl Reads for ShiftReading {
    fn unread(&self) -> Comparison {
        Comparison::of(self.compared(false))
    }

    /// Both frames in grey, the copy's and the picture's where it was read; no number kept.
    fn read(&self, size: [u32; 2], pixels: &[u8], _: Rgb8, original: Option<Pixels<'_>>) -> Read {
        let grey = (size, in_grey(pixels));
        let pictures = std::iter::once(grey)
            .chain(original.map(|(size, rgba)| (size, in_grey(rgba))))
            .collect();
        Read {
            comparison: Comparison::of(self.compared(true)),
            metrics: Metrics::new(),
            pictures,
        }
    }

    fn greys(&self) -> bool {
        true
    }
}

/// A cold-warm copy compared: where its picture lies, its pair, and whether its frames were read
/// (the Grey view's pictures).
#[derive(Debug, Clone, PartialEq)]
pub struct ShiftCompared {
    pub corners: [Point; 4],
    pub leans: Leans,
    pub read: bool,
}

impl Compared for ShiftCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let at = Place::Corners(self.corners);
        let picture = |source, place, shown| Figure::Picture {
            source,
            place,
            shown,
        };
        if seen.grey && self.read {
            let laid = picture(Source::Read(GREY_PICTURE), at, seen.shown);
            return vec![
                SheetMark::free(In::Original, picture(Source::Read(GREY_PICTURE), at, 1.0)),
                SheetMark::free(
                    In::Copy,
                    picture(Source::Read(GREY_COPY), Place::Frame, 1.0),
                ),
                SheetMark::free(In::Copy, laid).clipped(Clip::Laid),
            ];
        }
        let laid = picture(Source::Pack, at, seen.shown);
        vec![SheetMark::free(In::Copy, laid).clipped(Clip::Laid)]
    }

    /// Grey once read, the slider, then the point compared: on the disc with the two tendencies,
    /// and its lightness bar.
    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem> {
        let mut items: Vec<CardItem> = self.read.then_some(CardItem::Grey).into_iter().collect();
        items.push(CardItem::Slider);
        if let Some((original, copy)) = seen.probe {
            items.push(CardItem::Leaning {
                original,
                copy,
                tendencies: self.leans.on_disc(),
            });
            items.push(CardItem::Bar { original, copy });
        }
        items
    }

    /// The eye looks; nothing is asked or kept.
    fn verdict(&self) -> Verdict {
        Verdict::Nobody
    }
}
