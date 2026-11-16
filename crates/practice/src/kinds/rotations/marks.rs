//! A structure rotation's marks (S113): the picture of view A filling the original's frame, lit on
//! paper as an object's; in the copy's frame the horizon and view B's box given, printed grey: its
//! front corner with three edges, or at 稳 its near upright alone and the small angle box beside
//! it. Compared, B's exact answer over the copy: its visible edges and seen curves in see-through
//! red, what its own parts hide dashed; Construction (看画法) lays V大's construction at B in the
//! copy's frame: the whole box in pencil (its hidden edges dashed), the construction in dashed
//! pencil, the corners it finds ringed, a round part's middles dotted.
//!
//! The learner's eye judges (the author, 2026-11-16): the copy's strokes are never read, nothing is
//! ringed or kept, and the card asks Accurate, Almost or Again after Compare.

use crate::geometry::perspective::Direction;
use crate::kinds::rotations::rotation::RotationTask;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{box_edges, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};

/// The picture in the original's frame, the horizon and B's given box in the copy's.
pub(crate) fn givens(task: &RotationTask) -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    let seen = &task.asked.seen;
    let mut marks = vec![
        SheetMark::free(In::Original, picture),
        SheetMark::cut(In::Copy, Figure::Horizon(seen.horizon)),
    ];
    let bare = task.angle.is_some();
    let given = (seen.given().into_iter())
        .filter(|(_, _, way)| !bare || *way == Direction::Vertical)
        .map(|(a, b, _)| segment(In::Copy, [a, b], LinePen::Printed));
    marks.extend(given);
    if let Some(angle) = &task.angle {
        marks.extend(box_edges(In::Copy, angle, (LinePen::PrintedHalf, None)));
    }
    marks
}

/// A structure rotation compared: view B's answer laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct RotationCompared {
    pub task: RotationTask,
}

impl Compared for RotationCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let asked = &self.task.asked;
        let mut marks = Vec::new();
        if seen.construction {
            let pencil = (LinePen::Pencil, Some(LinePen::Pencil));
            let mut over = box_edges(In::Copy, &asked.seen, pencil);
            let lines = asked.construction().into_iter();
            over.extend(
                lines.map(|line| SheetMark::dashed(In::Copy, line.to_vec(), LinePen::Pencil)),
            );
            let (found, middles) = asked.found();
            over.extend(
                found
                    .into_iter()
                    .map(|p| SheetMark::free(In::Copy, Figure::Found(p))),
            );
            let dot = |at| Figure::Dot { at, small: true };
            over.extend(
                middles
                    .into_iter()
                    .map(|p| SheetMark::free(In::Copy, dot(p))),
            );
            marks.extend(over.into_iter().map(|mark| mark.clipped(Clip::Inside)));
        }
        let sides = asked.answer.sides.iter();
        marks.extend(sides.map(|side| segment(In::Copy, *side, LinePen::Answer)));
        let hidden = asked.hidden.iter();
        marks
            .extend(hidden.map(|side| SheetMark::dashed(In::Copy, side.to_vec(), LinePen::Answer)));
        for stretch in asked.answer.outlines.iter().flatten() {
            let points = stretch.points.clone();
            marks.push(if stretch.hidden {
                SheetMark::dashed(In::Copy, points, LinePen::Answer)
            } else {
                SheetMark::line(In::Copy, points, LinePen::Answer)
            });
        }
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the answer laid over it (the author, 2026-11-16).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
