//! A line's or a group's marks (S88): its lines in ink in the original's frame; compared, laid over
//! the copy in place or aligned (`Seen::fit`) in the marking red as thick as the pen and
//! see-through; the card offers In place and Aligned.

use crate::marks::asked::{Asked, Compare, ToFit};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen};
use crate::marks::mark::{Clip, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// A line's or a group's copy compared, once anything lies in its frame: its original's lines laid
/// over it, fitted onto it for Aligned.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let lines = asked.original.lines()?.to_vec();
    let fit = ToFit {
        original: lines.clone(),
        shape: false,
    };
    let shown = Comparison::of(LinesCompared { lines });
    Some(Compare::Fitted(shown, Some(fit)))
}

/// The lines in ink in the original's frame.
pub(crate) fn givens(lines: &[Vec<Point>]) -> Vec<SheetMark> {
    (lines.iter())
        .map(|line| SheetMark::free_line(In::Original, line.clone(), LinePen::Ink))
        .collect()
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
}
