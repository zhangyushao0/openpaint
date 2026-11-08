//! The copies Compare checks as geometry, at once on the frame loop (each in well under a
//! millisecond): the copy's lines handed to its kind's check (`Exercise::check`, S109) as drawn,
//! and once compared before, as drawn before its first Compare and corrected after it; the sketch
//! pen's construction is never read. And (S95) a division's lines, read by their pens' levels,
//! joined, counted and measured: no answer to check them against.

use std::ops::Range;

use openpaint_core::Artwork;
use openpaint_practice::{Comparison, DivisionCheck, DrawnCopy, Frame, Original};

use super::PracticeSheet;
use crate::practice_copy::{ALL_STROKES, division_pieces, line_pieces};

impl PracticeSheet {
    /// Exercise `index`'s copy in `frame` of `artwork` checked as geometry; none for an exercise
    /// of a kind not checked so.
    pub(super) fn checked(
        &self,
        index: usize,
        original: &Original,
        (artwork, frame): (&Artwork, &Frame),
    ) -> Option<Comparison> {
        let pieces = |ids: Range<u64>| line_pieces(artwork, frame, ids);
        let copy = DrawnCopy {
            all: pieces(ALL_STROKES),
            corrected: (self.corrected_from(index))
                .map(|since| (pieces(0..since), pieces(since..u64::MAX))),
        };
        self.record.exercises.get(index)?.check(original, &copy)
    }

    /// The first stroke id of exercise `index`'s corrections, once it was compared.
    fn corrected_from(&self, index: usize) -> Option<u64> {
        (self.record.result(index)).and_then(|result| result.corrected_from)
    }

    /// Exercise `index`'s division as drawn in its copy's frame of `artwork` (S95): every line of
    /// the levels' pens (those drawn after a Compare too, part of it), joined, counted and
    /// measured against its picture's longer side; none without its picture.
    pub(super) fn division(&self, index: usize, artwork: &Artwork) -> Option<DivisionCheck> {
        let picture = self.sheet.pictures.get(index)?.as_ref()?;
        let frame = self.sheet.pairs.get(index)?.copy;
        let side = picture.fit.size[0].max(picture.fit.size[1]);
        Some(DivisionCheck::of(&division_pieces(artwork, &frame), side))
    }
}
