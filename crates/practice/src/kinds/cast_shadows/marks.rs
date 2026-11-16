//! Cast shadows' marks (S153). In its one frame, printed grey: the horizon, labelled; the boxes,
//! their hidden edges dashed; the light's givens: the pin (its top a dot) and its shadow, at 熟练
//! the light's point and the shadows' point ringed too, at 稳 the lamp and its foot, a dashed
//! upright between. Compared, the shadows' exact outline over the copy in see-through red, what the
//! boxes hide dashed; Construction (看画法) lays each shadow corner's ray and ground line faint.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::cast_shadows::scene::ShadowTask;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{box_edges, segment};
use crate::marks::mark::{Figure, In, LinePen, SheetMark};

/// A light's point printed as a ring this wide (frame units).
const RING: f64 = 0.012;

/// The horizon, the boxes and the light's givens, printed grey in the one frame.
pub(crate) fn givens(task: &ShadowTask) -> Vec<SheetMark> {
    let mut marks = vec![SheetMark::cut(In::Copy, Figure::Horizon(task.horizon()))];
    for seen in &task.boxes {
        marks.extend(box_edges(
            In::Copy,
            seen,
            (LinePen::Printed, Some(LinePen::PrintedHalf)),
        ));
    }
    let dot = |at| {
        SheetMark::cut(
            In::Copy,
            Figure::Given {
                at,
                pen: LinePen::Printed,
            },
        )
    };
    if let Some([foot, top, tip]) = task.pinned {
        marks.push(segment(In::Copy, [foot, top], LinePen::Printed));
        marks.push(segment(In::Copy, [foot, tip], LinePen::Printed));
        marks.push(dot(top));
    }
    match (task.points, task.pinned) {
        (Some(points), Some(_)) => {
            let ring = |at| Figure::Counted {
                at,
                radius: RING,
                pen: LinePen::Printed,
            };
            marks.extend(points.map(|at| SheetMark::cut(In::Copy, ring(at))));
        }
        (Some([lamp, foot]), None) => {
            marks.push(SheetMark::dashed(
                In::Copy,
                vec![foot, lamp],
                LinePen::PrintedHalf,
            ));
            marks.extend([dot(lamp), dot(foot)]);
        }
        _ => {}
    }
    marks
}

/// Cast shadows compared: the shadows' outline laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowCompared {
    pub task: ShadowTask,
}

impl Compared for ShadowCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            let lines = self.task.shadows.construction(self.task.points).into_iter();
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
        }
        for (ends, hidden) in &self.task.shadows.outline {
            marks.push(if *hidden {
                SheetMark::dashed(In::Copy, ends.to_vec(), LinePen::Answer)
            } else {
                segment(In::Copy, *ends, LinePen::Answer)
            });
        }
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the outline laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
