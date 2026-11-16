//! A solid's marks (S93, S98, S145): in its one frame the box whole printed grey (its hidden edges
//! dashed and thinner) and its horizon. Compared, the exact solid in see-through red over the copy,
//! its hidden part dashed; Construction lays V大's construction faint (the diagonals, the lines
//! through the centres, a sphere's box halved and its section's ellipse) with the middles and a
//! cone's point dotted. The learner's eye judges (D123): the copy's strokes are never read, and the
//! card asks Accurate, Almost or Again after Compare.

use crate::kinds::solids::solid::{Solid, SolidTask};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{box_edges, segment};
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// An ellipse drawn in pieces.
const PIECES: usize = 96;

/// The horizon and the box whole, printed grey in the one frame.
pub(crate) fn givens(task: &SolidTask) -> Vec<SheetMark> {
    let horizon = SheetMark::cut(In::Copy, Figure::Horizon(task.seen.horizon));
    let pens = (LinePen::Printed, Some(LinePen::PrintedHalf));
    std::iter::once(horizon)
        .chain(box_edges(In::Copy, &task.seen, pens))
        .collect()
}

/// A solid compared: the exact solid laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidCompared {
    pub task: SolidTask,
}

impl Compared for SolidCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let task = &self.task;
        let mut marks = Vec::new();
        if seen.construction {
            let lines = task.construction().into_iter();
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
            if task.solid == Solid::Sphere
                && let Some(section) = task.squares.first()
            {
                let outline = section.exact.outline(PIECES);
                marks.push(SheetMark::line(In::Copy, outline, LinePen::Faint));
            }
            let dots = task.middles().into_iter().chain(task.point);
            let dot = |at: Point| Figure::Dot { at, small: false };
            marks.extend(dots.map(|at| SheetMark::free(In::Copy, dot(at))));
        }
        for stretch in task.answer.outlines.iter().flatten() {
            let points = stretch.points.clone();
            marks.push(if stretch.hidden {
                SheetMark::dashed(In::Copy, points, LinePen::Answer)
            } else {
                SheetMark::line(In::Copy, points, LinePen::Answer)
            });
        }
        let sides = task.answer.sides.iter();
        marks.extend(sides.map(|side| segment(In::Copy, *side, LinePen::Answer)));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the solid laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
