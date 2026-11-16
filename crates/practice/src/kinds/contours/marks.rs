//! A contour exercise's marks (S156k): the lit sausage filling the original's frame; in the copy's
//! frame its outline printed grey, at 入门 and 熟练 its spine printed faint and dashed, a short tick
//! across the outline where each asked contour goes, at 入门 the near end's contour printed as the
//! example. Compared, each asked section's exact ellipse in see-through red over the copy, its
//! hidden half dashed; Construction (看画法) lays the spine in pencil and each section's square,
//! dashed, its middles dotted.
//!
//! The learner's eye judges (D123): the copy's strokes are never read, nothing is ringed or kept,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::kinds::contours::sections::Contour;
use crate::kinds::contours::task::ContourTask;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;

/// The picture in the original's frame; the outline, the spine, the ticks and the example in
/// the copy's.
pub(crate) fn givens(task: &ContourTask) -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    let mut marks = vec![SheetMark::free(In::Original, picture)];
    let outline = task.outline.iter().cloned();
    marks.extend(outline.map(|line| SheetMark::line(In::Copy, line, LinePen::Printed)));
    if task.bend.through() {
        marks.push(SheetMark::dashed(
            In::Copy,
            task.spine.clone(),
            LinePen::PrintedHalf,
        ));
    }
    let ticks = task.contours.iter().flat_map(|c| &c.ticks);
    marks.extend(ticks.map(|tick| SheetMark::line(In::Copy, tick.to_vec(), LinePen::Printed)));
    if let Some(example) = &task.example {
        marks.extend(drawn(example, (LinePen::Printed, LinePen::PrintedHalf)));
    }
    marks
}

/// `contour`'s stretches, the seen in `pens.0`, the hidden dashed in `pens.1`.
fn drawn(contour: &Contour, (seen, hidden): (LinePen, LinePen)) -> Vec<SheetMark> {
    (contour.stretches.iter())
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

/// A contour exercise compared: the asked sections' exact ellipses over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourCompared {
    pub task: ContourTask,
}

impl Compared for ContourCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(construction(&self.task));
        }
        let asked = self.task.contours.iter();
        marks.extend(asked.flat_map(|c| drawn(c, (LinePen::Answer, LinePen::Answer))));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the sections laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

/// The spine in pencil, each asked section's square dashed, its middles dotted.
fn construction(task: &ContourTask) -> Vec<SheetMark> {
    let mut over = vec![SheetMark::line(
        In::Copy,
        task.spine.clone(),
        LinePen::Pencil,
    )];
    for contour in &task.contours {
        let corners = contour.guide.corners;
        let square: Vec<Point> = (0..=4).map(|k| corners[k % 4]).collect();
        over.push(SheetMark::dashed(In::Copy, square, LinePen::Pencil));
        let dot = |at: Point| Figure::Dot { at, small: true };
        over.extend((contour.guide.touches.iter()).map(|p| SheetMark::free(In::Copy, dot(*p))));
    }
    over.into_iter()
        .map(|mark| mark.clipped(Clip::Inside))
        .collect()
}
