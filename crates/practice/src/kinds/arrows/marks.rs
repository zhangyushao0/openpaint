//! An organic arrow's marks (S156l): in its one frame the path dashed grey, the arrowhead printed
//! with the near width ticked where the edges leave it, at 入门 the far width ticked. Compared, the
//! exact ribbon over the copy in see-through red: its edges and its folds, what the ribbon itself
//! hides dashed; Construction (看画法) lays the width across at even steps along it, faint, dashed
//! where hidden.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::arrows::answer::ArrowAnswer;
use crate::kinds::arrows::task::ArrowTask;
use crate::kinds::solids::solid_answer::Stretch;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::segment;
use crate::marks::mark::{In, LinePen, SheetMark};

/// The path dashed, the arrowhead and the ticks, printed grey in the one frame.
pub(crate) fn givens(task: &ArrowTask) -> Vec<SheetMark> {
    let [tip, a, b] = task.head;
    let mut marks = vec![
        SheetMark::dashed(In::Copy, task.path.clone(), LinePen::Printed),
        SheetMark::line(In::Copy, vec![tip, a, b, tip], LinePen::Printed),
    ];
    marks.extend((task.ticks.iter()).map(|tick| segment(In::Copy, *tick, LinePen::Printed)));
    marks
}

/// An organic arrow compared: the exact ribbon laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowCompared {
    pub answer: ArrowAnswer,
}

impl Compared for ArrowCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let answer = &self.answer;
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(answer.widths.iter().map(|s| stretch(s, LinePen::Faint)));
        }
        let folds = answer.folds.iter().flatten();
        let red = answer.stretches.iter().flatten().chain(folds);
        marks.extend(red.map(|s| stretch(s, LinePen::Answer)));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the ribbon laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

/// `stretch` in `pen`, dashed where hidden.
fn stretch(stretch: &Stretch, pen: LinePen) -> SheetMark {
    let points = stretch.points.clone();
    if stretch.hidden {
        SheetMark::dashed(In::Copy, points, pen)
    } else {
        SheetMark::line(In::Copy, points, pen)
    }
}
