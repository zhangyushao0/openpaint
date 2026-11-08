//! A solid's marks (S93, S98): in its one frame the box whole printed grey (its hidden edges dashed
//! and thinner) and its horizon. Compared, the exact solid in see-through red, its hidden part
//! dashed, and only what is off; Construction lays V大's construction faint (the diagonals, the
//! lines through the centres, a sphere's box halved and its section's ellipse) with the middles and
//! a cone's point dotted.

use std::sync::Arc;

use crate::kinds::solids::solid::{Solid, SolidTask};
use crate::kinds::solids::solid_check::SolidCheck;
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, DrawnCopy, Seen};
use crate::marks::drawing::{box_edges, off_marks, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

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

/// A solid compared.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidCompared {
    pub task: SolidTask,
    pub check: SolidCheck,
}

impl SolidCompared {
    pub fn of(task: &SolidTask, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let (first, corrections) = copy.split();
        Arc::new(Self {
            task: task.clone(),
            check: SolidCheck::of(task, (&first, &corrections)),
        })
    }
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
        marks.extend(off_marks(&self.check.off, Clip::Sheet));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let none = (self.check.metrics.is_empty()).then(|| CardItem::Says(Say::plain("no-solid")));
        none.into_iter().chain([CardItem::Construction]).collect()
    }

    fn metrics(&self) -> Metrics {
        self.check.metrics.clone()
    }
}
