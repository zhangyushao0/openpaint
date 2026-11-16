//! Ghosted lines' marks (S156). In its one frame, printed grey: a superimposed line and the dot it
//! starts from; each pair's two points, a short stub at each toward the other (so five pairs read
//! apart); a plane's four corners. Compared, the exact lines between
//! the given points over the copy in see-through red, so the eye sees where each stroke started,
//! how its middle bowed and where it ended; at 稳 Construction (看画法) runs each pair of opposite
//! edges and the line through the centre between them on faint toward where they meet.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::ghosted_lines::ghosted::{Ghosted, GhostedTask};
use crate::kinds::ghosted_lines::plane::Plane;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::segment;
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::path::{Point, distance};

/// How far Construction runs a plane's lines on toward a vanishing point at most (frame units from
/// the centre); the sheet's edge cuts them.
const RUN_ON: f64 = 1.5;

/// A stub's length at a pair's point (frame units): enough to say which way its partner lies.
const STUB: f64 = 0.03;

/// What it prints in its one frame: a superimposed line and its start, or the points to join, each
/// of a pair with a short stub toward its partner, so the pairs read apart.
pub(crate) fn givens(task: &GhostedTask) -> Vec<SheetMark> {
    let dot = |at| {
        let pen = LinePen::Printed;
        SheetMark::cut(In::Copy, Figure::Given { at, pen })
    };
    let printed = |line: [Point; 2]| segment(In::Copy, line, LinePen::Printed);
    let mut marks = Vec::new();
    match task.ghosted {
        Ghosted::Superimposed => marks.extend(task.lines.iter().map(|line| printed(*line))),
        Ghosted::Pairs => {
            marks.extend(task.lines.iter().flat_map(|line| stubs(*line).map(printed)))
        }
        Ghosted::Plane => {}
    }
    marks.extend(task.points().into_iter().map(dot));
    marks
}

/// A pair's two stubs, `STUB` from each end toward the other.
pub(crate) fn stubs([a, b]: [Point; 2]) -> [[Point; 2]; 2] {
    let share = STUB / distance(a, b).max(STUB);
    let toward = |from: Point, to: Point| {
        [
            from,
            [
                from[0] + (to[0] - from[0]) * share,
                from[1] + (to[1] - from[1]) * share,
            ],
        ]
    };
    [toward(a, b), toward(b, a)]
}

/// Ghosted lines compared: the exact lines laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostedCompared {
    pub task: GhostedTask,
}

impl Compared for GhostedCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if let Some(plane) = self.task.plane.filter(|_| seen.construction) {
            let lines = run_on(&plane).into_iter();
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
        }
        let lines = self.task.lines.iter();
        marks.extend(lines.map(|line| segment(In::Copy, *line, LinePen::Answer)));
        marks
    }

    /// A plane's Construction; nothing else beside the eye's choice.
    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        self.task
            .plane
            .map(|_| CardItem::Construction)
            .into_iter()
            .collect()
    }

    /// The eye judges the copy against the lines laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

/// A plane's construction: each pair of opposite edges and the line through the centre between
/// them run on toward where they meet, as far as `RUN_ON` from the centre.
pub(crate) fn run_on(plane: &Plane) -> Vec<[Point; 2]> {
    let [a, b, c, d] = plane.corners;
    let centre = plane.centre();
    let [first, second] = plane.middles();
    let pairs = [([a, b], [d, c], first), ([a, d], [b, c], second)];
    let mut lines = Vec::new();
    for (vanishing, (one, other, middle)) in plane.vanishing().into_iter().zip(pairs) {
        let Some(vanishing) = vanishing else {
            continue;
        };
        // From the end nearer the vanishing point on toward it, to it or `RUN_ON` from the centre.
        let share = (RUN_ON / distance(vanishing, centre)).min(1.0);
        let toward = |[p, q]: [Point; 2]| -> [Point; 2] {
            let near = if distance(p, vanishing) < distance(q, vanishing) {
                p
            } else {
                q
            };
            let end = [
                near[0] + (vanishing[0] - near[0]) * share,
                near[1] + (vanishing[1] - near[1]) * share,
            ];
            [near, end]
        };
        lines.extend([toward(one), toward(other), toward(middle)]);
    }
    lines
}
