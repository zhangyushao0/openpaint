//! A placement's marks (S153): in the original's frame the plan printed grey (`printed.rs`); in the
//! copy's the horizon, labelled, the ground's four edges and its near corner, printed grey, and the
//! unit: the upright at the near corner, or at 稳 its length along the near edge between two dots.
//! Compared, the exact answer over the copy: every edge and outline the eye sees in see-through
//! red, what the pieces hide dashed. Construction (看画法) lays V大's construction in the copy's
//! frame: the ground's diagonals and the lines of its halves and quarters, the upright, the height
//! lines dashed, the footprints and a cylinder's top square in pencil, the corners and tops they
//! find ringed.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed off or
//! kept, and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::placements::placement::PlacementTask;
use crate::kinds::placements::plan::Footprint;
use crate::kinds::placements::printed::{Printed, printed};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::segment;
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// The plan in the original's frame; the horizon, the ground and the unit in the copy's.
pub(crate) fn givens(task: &PlacementTask) -> Vec<SheetMark> {
    let mut marks: Vec<SheetMark> = (printed(&task.plan, task.along.is_some()).into_iter())
        .map(|printed| match printed {
            Printed::Line(points) => SheetMark::free_line(In::Original, points, LinePen::Printed),
            Printed::Area(points) => SheetMark::free(In::Original, Figure::Shade(points)),
            Printed::Dot(at) => given(In::Original, at),
        })
        .collect();
    marks.push(SheetMark::cut(In::Copy, Figure::Horizon(task.horizon)));
    let g = task.ground;
    for k in 0..4 {
        marks.push(segment(In::Copy, [g[k], g[(k + 1) % 4]], LinePen::Printed));
    }
    marks.push(given(In::Copy, g[0]));
    match task.along {
        Some(along) => marks.push(given(In::Copy, along)),
        None => marks.push(segment(In::Copy, task.upright, LinePen::Printed)),
    }
    marks
}

/// A given dot at `at`, printed grey.
fn given(on: In, at: Point) -> SheetMark {
    SheetMark::free(
        on,
        Figure::Given {
            at,
            pen: LinePen::Printed,
        },
    )
}

/// A placement compared: the exact answer laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementCompared {
    pub task: PlacementTask,
}

impl Compared for PlacementCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let task = &self.task;
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(
                constructed(task)
                    .into_iter()
                    .map(|mark| mark.clipped(Clip::Inside)),
            );
        }
        let sides = task.answer.sides.iter();
        marks.extend(sides.map(|side| segment(In::Copy, *side, LinePen::Answer)));
        let hidden = task.hidden.iter();
        marks
            .extend(hidden.map(|side| SheetMark::dashed(In::Copy, side.to_vec(), LinePen::Answer)));
        for stretch in task.answer.outlines.iter().flatten() {
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

/// V大's construction in the copy's frame: the grid and the height lines dashed, the upright, the
/// footprints and a cylinder's top square in pencil, the corners and tops found ringed.
pub(crate) fn constructed(task: &PlacementTask) -> Vec<SheetMark> {
    let built = &task.construction;
    let dashed = |line: &[Point; 2]| SheetMark::dashed(In::Copy, line.to_vec(), LinePen::Pencil);
    let mut marks: Vec<SheetMark> = built.grid.iter().map(dashed).collect();
    marks.push(segment(In::Copy, built.upright, LinePen::Pencil));
    let pieces = task.plan.pieces.iter();
    for (((piece, feet), tops), carried) in (pieces.zip(&built.footprints))
        .zip(&built.tops)
        .zip(&built.carried)
    {
        marks.push(SheetMark::line(In::Copy, closed(feet), LinePen::Pencil));
        if let Footprint::Round { .. } = piece.footprint {
            marks.push(SheetMark::line(In::Copy, closed(tops), LinePen::Pencil));
        }
        marks.extend(carried.lines.iter().map(dashed));
        let found = feet.iter().copied().chain([carried.top]);
        marks.extend(found.map(|p| SheetMark::free(In::Copy, Figure::Found(p))));
    }
    marks.extend(built.runs.iter().map(dashed));
    marks
}

/// `corners` round and back to the first.
fn closed(corners: &[Point; 4]) -> Vec<Point> {
    corners.iter().chain(corners.first()).copied().collect()
}
