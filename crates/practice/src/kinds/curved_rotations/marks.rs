//! A curved structure rotation's marks (S154f): the picture of view A filling the original's frame,
//! lit on paper; in the copy's frame the horizon and view B's box given as the level says (its
//! front corner with three edges, or at 稳 its near upright alone and the small angle box beside
//! it), its axis printed dashed through the box's end faces' middles, at 入门 each joint and end
//! ticked on it. Compared, B's exact answer over the copy: what the eye sees in see-through red,
//! what the object's own parts hide dashed; Construction (看画法) lays V大's construction in the
//! copy's frame: the whole box in pencil (its hidden edges dashed), each section's square found in
//! dashed pencil with its ellipse in it, the corners it finds ringed, the middles dotted, and the
//! box what stands on the side stands in.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::demos::demo::tick;
use crate::geometry::perspective::Direction;
use crate::kinds::curved_rotations::curved::{CurvedTask, Handed};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{box_edges, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};

/// A tick's half across the axis (frame units); an ellipse drawn in this many pieces.
const TICK: f64 = 0.018;
const PIECES: usize = 96;

/// The picture in the original's frame; the horizon, B's given box, its axis and ticks in the
/// copy's.
pub(crate) fn givens(task: &CurvedTask) -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    let asked = &task.asked;
    let seen = &asked.seen;
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
    marks.push(SheetMark::dashed(
        In::Copy,
        asked.axis.to_vec(),
        LinePen::Printed,
    ));
    if task.curved.handed == Handed::Ticked {
        let [foot, top] = asked.axis;
        let ticks = (asked.joints.iter()).map(|at| {
            let towards = if *at == top { foot } else { top };
            SheetMark::line(In::Copy, tick(*at, towards, TICK), LinePen::Printed)
        });
        marks.extend(ticks);
    }
    marks
}

/// A curved structure rotation compared: view B's answer laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvedCompared {
    pub task: CurvedTask,
}

impl Compared for CurvedCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let asked = &self.task.asked;
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(construction(&self.task));
        }
        let answer = &asked.answer;
        marks.extend((answer.sides.iter()).map(|side| segment(In::Copy, *side, LinePen::Answer)));
        let hidden = answer.hidden.iter();
        marks
            .extend(hidden.map(|side| SheetMark::dashed(In::Copy, side.to_vec(), LinePen::Answer)));
        for stretch in answer.curves.iter().flat_map(|curve| &curve.stretches) {
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

    /// The eye judges the copy against the answer laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

/// V大's construction at B, in the copy's frame: the box, each section found and its ellipse, the
/// box what stands on the side stands in, a spout's sections.
fn construction(task: &CurvedTask) -> Vec<SheetMark> {
    let asked = &task.asked;
    let pencil = (LinePen::Pencil, Some(LinePen::Pencil));
    let mut over = box_edges(In::Copy, &asked.seen, pencil);
    let lines = (asked.stages.iter()).flat_map(|stage| stage.lines.iter());
    over.extend(lines.map(|line| SheetMark::dashed(In::Copy, line.to_vec(), LinePen::Pencil)));
    let (edges, guides) = &asked.attached;
    over.extend((edges.iter()).map(|edge| segment(In::Copy, *edge, LinePen::Pencil)));
    let ellipses = (asked.sections.iter().map(|s| &s.guide)).chain(guides);
    over.extend(
        ellipses
            .map(|guide| SheetMark::line(In::Copy, guide.exact.outline(PIECES), LinePen::Pencil)),
    );
    let found = (asked.stages.iter()).flat_map(|stage| stage.found.iter());
    over.extend(found.map(|p| SheetMark::free(In::Copy, Figure::Found(*p))));
    let middles = (asked.stages.iter()).flat_map(|stage| stage.middles.iter());
    let dot = |at| Figure::Dot { at, small: true };
    over.extend(middles.map(|p| SheetMark::free(In::Copy, dot(*p))));
    over.into_iter()
        .map(|mark| mark.clipped(Clip::Inside))
        .collect()
}
