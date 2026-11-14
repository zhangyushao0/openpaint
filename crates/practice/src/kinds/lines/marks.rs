//! A line's or a group's original and marks (S88): its lines in ink in the original's frame;
//! compared, laid over the copy in place or aligned (`Seen::fit`) in the marking red as thick as
//! the pen and see-through; the card offers In place and Aligned.

use crate::kinds::Original;
use crate::marks::asked::{Asked, Compare, ToFit};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// A line's or a group's original: its lines, each in frame units (combined lines are lines
/// copied as one shape).
#[derive(Debug)]
pub(crate) struct GivenLines(pub(crate) Vec<Vec<Point>>);

impl Original for GivenLines {
    /// The lines in ink in the original's frame.
    fn givens(&self) -> Vec<SheetMark> {
        (self.0.iter())
            .map(|line| SheetMark::free_line(In::Original, line.clone(), LinePen::Ink))
            .collect()
    }

    /// Once anything lies in its frame: its lines laid over the copy, fitted onto it for
    /// Aligned.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let lines = self.0.clone();
        let fit = ToFit {
            original: lines.clone(),
            shape: false,
        };
        let shown = Comparison::of(LinesCompared { lines });
        Some(Compare::Fitted(shown, Some(fit)))
    }

    fn lines(&self) -> Option<&[Vec<Point>]> {
        Some(&self.0)
    }
}

/// A line or a group of lines compared: the original's lines (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct LinesCompared {
    pub lines: Vec<Vec<Point>>,
}

impl Compared for LinesCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(fit) = seen.fit else {
            return Vec::new();
        };
        let laid =
            |line: &Vec<Point>| -> Vec<Point> { line.iter().map(|p| fit.apply(*p)).collect() };
        (self.lines.iter())
            .map(|line| {
                SheetMark::free_line(In::Copy, laid(line), LinePen::Laid).clipped(Clip::Laid)
            })
            .collect()
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Overlaid]
    }

    /// The overlay is the feedback (D99): the eye judges.
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
