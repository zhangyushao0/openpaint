//! Free boxes' marks (S84, S114, S142, S145): the horizon in their one frame, and on a guided
//! three-point page two faint guides leading to its third point; on a path page (S156l) the path's
//! two edges printed grey. Compared, the page's horizon run on across the sheet, faint, so the
//! learner sees where their own edges, run on in pencil, meet. A free page has no answer and
//! nothing is read from its strokes (D123): the eye judges it.

use crate::kinds::free_boxes::FreePage;
use crate::kinds::free_boxes::path::PathPage;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{run_on, segment};
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// The guides' place across the page, and how far from the horizon they start and from the
/// frame's edge they end.
const GUIDES: [f64; 2] = [0.1, 0.9];
const GAP: f64 = 0.06;
const INSIDE: f64 = 0.06;

/// The horizon in the one frame, and a guided page's guides, faint.
pub(crate) fn givens(page: &FreePage) -> Vec<SheetMark> {
    let horizon = page.0.horizon;
    let mut marks = vec![SheetMark::cut(In::Copy, Figure::Horizon(horizon))];
    if let Some(third) = page.given_third() {
        let guided = guides(horizon, third).map(|guide| segment(In::Copy, guide, LinePen::Faint));
        marks.extend(guided);
    }
    marks
}

/// A path page's horizon in the one frame, and its path's two edges printed grey.
pub(crate) fn path_givens(page: &PathPage) -> Vec<SheetMark> {
    let mut marks = vec![SheetMark::cut(In::Copy, Figure::Horizon(page.horizon))];
    let edges = page.edges.iter();
    marks.extend(edges.map(|edge| SheetMark::line(In::Copy, edge.clone(), LinePen::Printed)));
    marks
}

/// A guided page's two faint guides toward `third` (S142): near either side of the page, from
/// beside the horizon at `horizon` to the frame's far edge.
#[must_use]
pub fn guides(horizon: f64, third: Point) -> [[Point; 2]; 2] {
    let (from, to) = if third[1] > horizon {
        (horizon + GAP, 1.0 - INSIDE)
    } else {
        (horizon - GAP, INSIDE)
    };
    GUIDES.map(|x| {
        let t = (to - from) / (third[1] - from);
        [[x, from], [x + (third[0] - x) * t, to]]
    })
}

/// A page of free boxes compared: its horizon (frame units), run on across the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeCompared {
    pub horizon: f64,
}

impl Compared for FreeCompared {
    fn marks(&self, _seen: &Seen<'_>) -> Vec<SheetMark> {
        vec![run_on(self.horizon)]
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        Vec::new()
    }

    /// Free boxes have no answer: the eye judges (S114, D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
