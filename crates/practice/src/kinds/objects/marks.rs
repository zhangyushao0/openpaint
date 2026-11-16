//! An object's marks (S94, S98, S145): its picture filling the original's frame (lit on paper, no
//! lines, no horizon), copied by eye; in the copy's frame the horizon and the box's front corner
//! with its three edges, printed grey. Compared, the object's visible edges and outlines in
//! see-through red over the copy; Construction lays over the picture the whole box in pencil (its
//! hidden edges dashed) and V大's construction in dashed pencil, the corners it finds ringed and a
//! round part's middles dotted, inside the frame. The learner's eye judges (D123): the copy's
//! strokes are never read, and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::objects::object::ObjectTask;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{box_edges, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};

/// The picture in the original's frame, the horizon and the given edges in the copy's.
pub(crate) fn givens(task: &ObjectTask) -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    let mut marks = vec![
        SheetMark::free(In::Original, picture),
        SheetMark::cut(In::Copy, Figure::Horizon(task.seen.horizon)),
    ];
    let given = task.seen.given().into_iter();
    marks.extend(given.map(|(a, b, _)| segment(In::Copy, [a, b], LinePen::Printed)));
    marks
}

/// An object compared: its exact visible edges laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectCompared {
    pub task: ObjectTask,
}

impl Compared for ObjectCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let task = &self.task;
        let mut marks = Vec::new();
        if seen.construction {
            let mut over = box_edges(
                In::Original,
                &task.seen,
                (LinePen::Pencil, Some(LinePen::Pencil)),
            );
            let lines = task.construction().into_iter();
            over.extend(
                lines.map(|line| SheetMark::dashed(In::Original, line.to_vec(), LinePen::Pencil)),
            );
            let (found, middles) = task.found();
            over.extend(
                found
                    .into_iter()
                    .map(|p| SheetMark::free(In::Original, Figure::Found(p))),
            );
            let dot = |at| Figure::Dot { at, small: true };
            over.extend(
                middles
                    .into_iter()
                    .map(|p| SheetMark::free(In::Original, dot(p))),
            );
            marks.extend(over.into_iter().map(|mark| mark.clipped(Clip::Inside)));
        }
        let sides = task.answer.sides.iter();
        marks.extend(sides.map(|side| segment(In::Copy, *side, LinePen::Answer)));
        let seen_outlines = (task.answer.outlines.iter().flatten()).filter(|s| !s.hidden);
        let outlines =
            seen_outlines.map(|s| SheetMark::line(In::Copy, s.points.clone(), LinePen::Answer));
        marks.extend(outlines);
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the edges laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
