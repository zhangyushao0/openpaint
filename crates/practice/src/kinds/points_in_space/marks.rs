//! A point in space's marks (S90, S98): built from given parts, it has the one frame: its box as
//! the eye sees it, its horizon, the face shaded and a mirror's given dot, printed grey. Compared,
//! the exact answer in see-through red (a cross for a point, the uprights or the next face) and only
//! what is off; Construction lays the trick's construction faint.

use std::sync::Arc;

use crate::kinds::points_in_space::space::{SpaceAnswer, SpaceTask, Trick};
use crate::kinds::points_in_space::space_check::SpaceCheck;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, DrawnCopy, Seen, Verdict};
use crate::marks::drawing::{box_edges, cross, nothing_read, off_marks, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// The box, its horizon, the shaded face and a mirror's given dot, printed grey in the one frame.
pub(crate) fn givens(task: &SpaceTask) -> Vec<SheetMark> {
    let mut marks = vec![
        SheetMark::free(In::Copy, Figure::Shade(task.corners.to_vec())),
        SheetMark::cut(In::Copy, Figure::Horizon(task.seen.horizon)),
    ];
    marks.extend(box_edges(In::Copy, &task.seen, (LinePen::Printed, None)));
    let given = |at: Point| Figure::Given {
        at,
        pen: LinePen::Printed,
    };
    marks.extend(
        task.given_at()
            .map(|at| SheetMark::free(In::Copy, given(at))),
    );
    marks
}

/// A point in space compared.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceCompared {
    pub task: SpaceTask,
    pub check: SpaceCheck,
}

impl SpaceCompared {
    pub fn of(task: &SpaceTask, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let lines: Vec<&[Point]> = copy.all.iter().map(Vec::as_slice).collect();
        Arc::new(Self {
            task: task.clone(),
            check: SpaceCheck::of(task, &lines),
        })
    }
}

impl Compared for SpaceCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            let lines = self.task.construction().into_iter();
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
        }
        match self.task.answer() {
            SpaceAnswer::Point(p) => marks.extend(cross(p, LinePen::Answer)),
            SpaceAnswer::Lines(lines) => {
                let red = lines
                    .into_iter()
                    .map(|l| segment(In::Copy, l, LinePen::Answer));
                marks.extend(red);
            }
        }
        marks.extend(off_marks(&self.check.off, Clip::Sheet));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let marked = matches!(self.task.trick, Trick::Centre | Trick::Mirror);
        let reason = self.check.reason(&self.task).map(CardItem::Says);
        (reason.into_iter())
            .chain(nothing_read(self.check.metrics.is_empty(), marked))
            .collect()
    }

    fn metrics(&self) -> Metrics {
        self.check.metrics.clone()
    }

    fn verdict(&self) -> Verdict {
        self.check.verdict(&self.task)
    }
}
