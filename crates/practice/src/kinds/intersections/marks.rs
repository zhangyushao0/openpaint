//! A form intersection's marks (S156k): in its one frame the horizon and both forms printed grey
//! (drawn through at 入门 and 熟练, what is hidden or inside the other dashed and thinner; only what
//! the eye sees of them at 稳). Compared, the exact line where they meet in see-through red over
//! the copy, its hidden stretch dashed; Construction (看画法) lays the way to find it in pencil:
//! the lines carried along faces and sides, dashed; the sections' squares (their sides and
//! diagonals, dashed) and circles; the pierce points ringed and what the sections find dotted.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::intersections::meeting::IntersectionTask;
use crate::kinds::solids::solid_answer::Stretch;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// An ellipse drawn in pieces.
const PIECES: usize = 96;

/// The horizon and both forms printed in the one frame.
pub(crate) fn givens(task: &IntersectionTask) -> Vec<SheetMark> {
    let horizon = SheetMark::cut(In::Copy, Figure::Horizon(task.seen.horizon));
    std::iter::once(horizon)
        .chain(drawn(
            &task.printed,
            (LinePen::Printed, LinePen::PrintedHalf),
        ))
        .collect()
}

/// `stretches` in the copy's frame, the seen in `pens.0`, the hidden dashed in `pens.1`.
fn drawn(stretches: &[Stretch], (seen, hidden): (LinePen, LinePen)) -> Vec<SheetMark> {
    (stretches.iter())
        .map(|stretch| {
            let points = stretch.points.clone();
            if stretch.hidden {
                SheetMark::dashed(In::Copy, points, hidden)
            } else {
                SheetMark::line(In::Copy, points, seen)
            }
        })
        .collect()
}

/// A form intersection compared: the line where the forms meet laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionCompared {
    pub task: IntersectionTask,
}

impl Compared for IntersectionCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(construction(&self.task));
        }
        marks.extend(drawn(&self.task.answer, (LinePen::Answer, LinePen::Answer)));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the line laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

/// The way to find the line, in pencil.
fn construction(task: &IntersectionTask) -> Vec<SheetMark> {
    let found = &task.found;
    let dashed = |line: Vec<Point>| SheetMark::dashed(In::Copy, line, LinePen::Pencil);
    let mut over: Vec<SheetMark> = found
        .lines
        .iter()
        .map(|line| dashed(line.to_vec()))
        .collect();
    for square in &found.squares {
        let c = square.corners;
        over.push(dashed((0..=4).map(|k| c[k % 4]).collect()));
        over.extend([[c[0], c[2]], [c[1], c[3]]].map(|line| dashed(line.to_vec())));
        let dot = |at: Point| Figure::Dot { at, small: true };
        over.extend((square.touches.iter()).map(|p| SheetMark::free(In::Copy, dot(*p))));
    }
    over.extend(
        (found.rings.iter())
            .map(|ring| SheetMark::line(In::Copy, ring.exact.outline(PIECES), LinePen::Pencil)),
    );
    over.extend((task.pierced.iter()).map(|p| SheetMark::free(In::Copy, Figure::Found(*p))));
    let dot = |at: Point| Figure::Dot { at, small: true };
    over.extend((found.points.iter()).map(|p| SheetMark::free(In::Copy, dot(*p))));
    over.into_iter()
        .map(|mark| mark.clipped(Clip::Inside))
        .collect()
}
