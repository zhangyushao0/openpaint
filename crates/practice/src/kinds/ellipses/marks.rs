//! An ellipse's marks (S84, S97, S145): its frame printed grey in the one frame. Compared, the
//! exact ellipse in see-through red over the copy; Construction lays the frame's diagonals and
//! middle lines faint, the middles dotted over the red. The learner's eye judges (D123): the copy's
//! strokes are never read or fitted, and the card asks Accurate, Almost or Again after Compare.

use crate::geometry::ellipse::EllipseGuide;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::segment;
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// An ellipse drawn in pieces.
const PIECES: usize = 96;

/// The guide's frame printed grey in the one frame.
pub(crate) fn givens(guide: &EllipseGuide) -> Vec<SheetMark> {
    let [a, b, c, d] = guide.corners;
    vec![SheetMark::line(
        In::Copy,
        vec![a, b, c, d, a],
        LinePen::Printed,
    )]
}

/// An ellipse compared: the exact one laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseCompared {
    pub guide: EllipseGuide,
}

impl Compared for EllipseCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let guide = &self.guide;
        let mut marks = Vec::new();
        if seen.construction {
            let (c, t) = (guide.corners, guide.touches);
            let lines = [[c[0], c[2]], [c[1], c[3]], [t[0], t[2]], [t[1], t[3]]];
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
        }
        let outline = guide.exact.outline(PIECES);
        marks.push(SheetMark::line(In::Copy, outline, LinePen::Answer));
        // The middles dotted over the red, which runs through them.
        let dots = (guide.touches.iter()).filter(|_| seen.construction);
        let dot = |at: &Point| Figure::Dot {
            at: *at,
            small: false,
        };
        marks.extend(dots.map(|at| SheetMark::free(In::Copy, dot(at))));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the ellipse laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
