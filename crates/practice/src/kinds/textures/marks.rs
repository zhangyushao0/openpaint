//! Texture's marks (S154j): the strip's picture filling the original's frame, the copy's frame of
//! its shape left blank. Compared, the answer's shadows laid over the copy as strongly as the slider
//! sets, each shadow the copy missed ringed, each mark of the copy over light outlined red. The
//! card: the slider; until the copy is read, that it is being read. The learner's eye judges, the
//! marks its help (D123).

use std::sync::Arc;

use crate::kinds::textures::read::TextureCheck;
use crate::kinds::textures::shadows::TextureTask;
use crate::marks::asked::{Pixels, Read, Reads};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;

/// The picture its reading brings: the answer's shadows.
const ANSWER: u8 = 0;

/// The strip's picture in the original's frame.
pub(crate) fn givens() -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    vec![SheetMark::free(In::Original, picture)]
}

/// Texture's copy, read from its frame's pixels on the compare worker against the answer.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureReading {
    pub task: Arc<TextureTask>,
}

impl Reads for TextureReading {
    fn unread(&self) -> Comparison {
        Comparison::of(TextureCompared { check: None })
    }

    fn read(&self, size: [u32; 2], pixels: &[u8], _: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let (check, answer) = self.task.read((size, pixels));
        Read {
            comparison: Comparison::of(TextureCompared { check: Some(check) }),
            metrics: Vec::new(),
            pictures: vec![(size, answer)],
        }
    }
}

/// Texture compared: the copy as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureCompared {
    pub check: Option<TextureCheck>,
}

impl Compared for TextureCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(check) = &self.check else {
            return Vec::new();
        };
        let answer = Figure::Picture {
            source: Source::Read(ANSWER),
            place: Place::Frame,
            shown: seen.shown,
        };
        let mut marks = vec![SheetMark::free(In::Copy, answer).clipped(Clip::Laid)];
        marks.extend((check.missed.iter()).map(|at| SheetMark::free(In::Copy, Figure::Ring(*at))));
        if !check.over.is_empty() {
            let lines = check.over.clone();
            marks.push(SheetMark::free(
                In::Copy,
                Figure::Haloed { lines, red: true },
            ));
        }
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let reading = self.check.is_none().then_some(CardItem::Reading);
        std::iter::once(CardItem::Slider).chain(reading).collect()
    }

    /// Once read, the eye's, the marks its help (D123); unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        if self.check.is_some() {
            Verdict::Eye
        } else {
            Verdict::Incomplete
        }
    }
}
