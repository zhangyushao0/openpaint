//! The copy Compare reads, as the shown exercise's kind asks (`openpaint_practice::Copied`, S109b):
//! read off the artwork in the copy's frame, each part only when asked: whether anything lies
//! there, the colours laid, the drawn lines (the sketch pen's construction never read) and a
//! division's lines by their pens' levels (S95).

use openpaint_core::Artwork;
use openpaint_practice::{Copied, DrawnCopy, Frame, Level, Point, Rgb8};

use crate::copy::{ALL_STROKES, colour_copy, division_pieces, holds_copy, line_pieces};

/// The copy in `frame` of `artwork`.
pub(super) struct CopyOn<'a> {
    pub(super) artwork: &'a Artwork,
    pub(super) frame: &'a Frame,
}

impl Copied for CopyOn<'_> {
    fn holds(&self) -> bool {
        holds_copy(self.artwork, self.frame)
    }

    fn picks(&self) -> Vec<Rgb8> {
        colour_copy(self.artwork, self.frame).picks
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy {
            all: line_pieces(self.artwork, self.frame, ALL_STROKES),
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        division_pieces(self.artwork, self.frame)
    }
}
