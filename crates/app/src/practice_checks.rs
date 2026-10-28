//! The copies Compare checks as geometry, at once on the frame loop (each in well under a
//! millisecond): a box's edges found and extended towards its vanishing points or a free page's
//! horizon (a correction in place of the edge it redraws), an ellipse fitted to the lines (to the
//! corrections once there are some), and (S90) an estimate's or a point in space's last mark or
//! lines read against the exact answer. The sketch pen's construction is never read.

use std::ops::Range;
use std::sync::Arc;

use openpaint_core::Artwork;
use openpaint_practice::{
    BoxCheck, Edge, EllipseCheck, EllipseGuide, FlatCheck, Frame, FreeCheck, Original, Point,
    SpaceCheck, corrected, edges,
};

use super::PracticeSheet;
use crate::practice::Overlay;
use crate::practice_copy::{ALL_STROKES, line_pieces};

impl PracticeSheet {
    /// Exercise `index`'s copy in `frame` of `artwork` checked as geometry; none for an exercise
    /// of another kind.
    pub(super) fn checked(
        &self,
        index: usize,
        original: &Original,
        (artwork, frame): (&Artwork, &Frame),
    ) -> Option<Overlay> {
        let drawn = || line_pieces(artwork, frame, ALL_STROKES);
        Some(match original {
            Original::Box(exact) => {
                let found = self.edges_in(index, artwork, frame);
                Overlay::Box(Arc::new(BoxCheck::of(exact, &found)))
            }
            Original::FreeBoxes(page) => {
                let found = self.edges_in(index, artwork, frame);
                Overlay::FreeBoxes(Arc::new(FreeCheck::of(page.horizon, &found)))
            }
            Original::Ellipse(guide) => {
                Overlay::Ellipse(Arc::new(self.ellipse_in(index, guide, (artwork, frame))))
            }
            Original::Flat(flat) => {
                let pieces = drawn();
                Overlay::Flat(Arc::new(FlatCheck::of(flat, &read(&pieces))))
            }
            Original::Space(task) => {
                let pieces = drawn();
                Overlay::Space(Arc::new(SpaceCheck::of(task, &read(&pieces))))
            }
            Original::Lines(_)
            | Original::Colour(_)
            | Original::Picture(_)
            | Original::Areas(_)
            | Original::Scale(_) => return None,
        })
    }

    /// The straight edges of exercise `index`'s copy drawn in `frame` (the sketch pen's
    /// construction left out), a correction in place of the edge it redraws.
    fn edges_in(&self, index: usize, artwork: &Artwork, frame: &Frame) -> Vec<Edge> {
        let found = |ids: Range<u64>| {
            let pieces = line_pieces(artwork, frame, ids);
            let lines: Vec<&[Point]> = pieces.iter().map(Vec::as_slice).collect();
            edges(&lines)
        };
        match self.corrected_from(index) {
            Some(since) => corrected(found(0..since), found(since..u64::MAX)),
            None => found(ALL_STROKES),
        }
    }

    /// The ellipse drawn in `frame` against `guide`: the corrections once there are some (the
    /// latest try, not one averaged with the first), else every line drawn.
    fn ellipse_in(
        &self,
        index: usize,
        guide: &EllipseGuide,
        (artwork, frame): (&Artwork, &Frame),
    ) -> EllipseCheck {
        let pieces = |ids: Range<u64>| line_pieces(artwork, frame, ids);
        let corrections = self
            .corrected_from(index)
            .map(|since| pieces(since..u64::MAX));
        let drawn = (corrections.filter(|pieces| !pieces.is_empty()))
            .unwrap_or_else(|| pieces(ALL_STROKES));
        let lines: Vec<&[Point]> = drawn.iter().map(Vec::as_slice).collect();
        EllipseCheck::of(guide, &lines)
    }

    /// The first stroke id of exercise `index`'s corrections, once it was compared.
    fn corrected_from(&self, index: usize) -> Option<u64> {
        (self.record.result(index)).and_then(|result| result.corrected_from)
    }
}

/// `pieces` as lines to read.
fn read(pieces: &[Vec<Point>]) -> Vec<&[Point]> {
    pieces.iter().map(Vec::as_slice).collect()
}
