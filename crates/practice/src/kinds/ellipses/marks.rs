//! An ellipse's marks (S84, S97, S145): its frame printed grey in the one frame. Compared, the
//! exact ellipse in see-through red over the copy; Construction lays the frame's diagonals and
//! middle lines faint, the middles dotted over the red. A funnel's (S156l): its sides printed grey,
//! its axis dashed, each ellipse's short axis whole on it, its ends marked across it; compared,
//! the three exact ellipses red;
//! Construction lays each one's long axis faint, square to the funnel's, and dots where it touches
//! the sides. The learner's eye judges (D123): the copy's strokes are never read or fitted, and the
//! card asks Accurate, Almost or Again after Compare.

use crate::geometry::ellipse::EllipseGuide;
use crate::kinds::ellipses::funnel::{Funnel, MARK};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::segment;
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// An ellipse drawn in pieces.
pub(crate) const PIECES: usize = 96;

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

/// A funnel's sides printed grey in the one frame, its axis dashed, each ellipse's short axis
/// whole along it, its ends marked across it.
pub(crate) fn funnel_givens(funnel: &Funnel) -> Vec<SheetMark> {
    let mut marks: Vec<SheetMark> = (funnel.sides.iter())
        .map(|side| segment(In::Copy, *side, LinePen::Printed))
        .collect();
    marks.push(SheetMark::dashed(
        In::Copy,
        funnel.axis.to_vec(),
        LinePen::Printed,
    ));
    let across = funnel.across;
    let mark = |at: &Point| {
        let by = |s: f64| [at[0] + s * MARK * across[0], at[1] + s * MARK * across[1]];
        segment(In::Copy, [by(-1.0), by(1.0)], LinePen::Printed)
    };
    for ring in &funnel.rings {
        marks.push(segment(In::Copy, ring.ends, LinePen::Printed));
        marks.extend(ring.ends.iter().map(mark));
    }
    marks
}

/// A funnel compared: its three exact ellipses laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct FunnelCompared {
    pub funnel: Funnel,
}

impl Compared for FunnelCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let rings = self.funnel.rings.iter();
        let mut marks = Vec::new();
        if seen.construction {
            let across = self.funnel.across;
            let long = rings.clone().map(|ring| {
                let (c, a) = (ring.exact.centre, ring.exact.axes[0]);
                let by = |s: f64| [c[0] + s * a * across[0], c[1] + s * a * across[1]];
                segment(In::Copy, [by(-1.0), by(1.0)], LinePen::Faint)
            });
            marks.extend(long);
        }
        let outlines = rings.clone().map(|ring| ring.exact.outline(PIECES));
        marks.extend(outlines.map(|outline| SheetMark::line(In::Copy, outline, LinePen::Answer)));
        let touches = (rings.flat_map(|ring| &ring.touches)).filter(|_| seen.construction);
        let dot = |at: &Point| Figure::Dot {
            at: *at,
            small: false,
        };
        marks.extend(touches.map(|at| SheetMark::free(In::Copy, dot(at))));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the ellipses laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
