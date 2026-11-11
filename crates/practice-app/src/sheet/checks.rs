//! The copy Compare reads, as the shown exercise's kind asks (`openpaint_practice::Copied`, S109b):
//! read off the artwork in the copy's frame, each part only when asked: whether anything lies
//! there, the colours laid, the drawn lines (the sketch pen's construction never read; once
//! compared, those before its first Compare and the corrections after it) and a division's lines
//! by their pens' levels (S95).

use openpaint_core::Artwork;
use openpaint_practice::{Copied, DrawnCopy, Frame, Level, Point, Rgb8};

use crate::copy::{ALL_STROKES, colour_copy, division_pieces, holds_copy, line_pieces};

/// The copy in `frame` of `artwork`; `corrected_from` the first stroke id of its corrections,
/// once it was compared.
pub(super) struct CopyOn<'a> {
    pub(super) artwork: &'a Artwork,
    pub(super) frame: &'a Frame,
    pub(super) corrected_from: Option<u64>,
}

impl Copied for CopyOn<'_> {
    fn holds(&self) -> bool {
        holds_copy(self.artwork, self.frame)
    }

    fn picks(&self) -> Vec<Rgb8> {
        colour_copy(self.artwork, self.frame).picks
    }

    fn drawn(&self) -> DrawnCopy {
        let pieces = |ids| line_pieces(self.artwork, self.frame, ids);
        DrawnCopy {
            all: pieces(ALL_STROKES),
            corrected: (self.corrected_from)
                .map(|since| (pieces(0..since), pieces(since..u64::MAX))),
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        division_pieces(self.artwork, self.frame)
    }
}
