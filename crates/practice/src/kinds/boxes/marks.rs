//! A box's marks (S84, S97, S98): in its one frame the horizon, labelled, and its front corner's
//! three edges, printed grey. Compared, over the copy, the exact box in see-through red (its hidden
//! edges dashed), its vanishing points ringed, and only what is off: an edge that misses its point
//! drawn on dashed red past it, a leaning vertical's true vertical. Construction lays every
//! corner's line to its vanishing points and the horizon run on across the sheet, faint. The card:
//! how many edges miss, or that none was found.

use std::sync::Arc;

use crate::geometry::perspective::PerspectiveBox;
use crate::kinds::DrawnCopy;
use crate::kinds::boxes::perspective_check::{BoxCheck, EDGES_TO_ADD, THROUGH_TO_ADD};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{counted, edges_of, off_marks, run_on, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};

/// The horizon and the front corner's edges, printed grey in the one frame.
pub(crate) fn givens(exact: &PerspectiveBox) -> Vec<SheetMark> {
    let horizon = SheetMark::cut(In::Copy, Figure::Horizon(exact.horizon));
    let given =
        (exact.given().into_iter()).map(|(a, b, _)| segment(In::Copy, [a, b], LinePen::Printed));
    std::iter::once(horizon).chain(given).collect()
}

/// A completed box compared: the edges it is complete with beside the three given (nine for a
/// box drawn through).
#[derive(Debug, Clone, PartialEq)]
pub struct BoxCompared {
    pub exact: PerspectiveBox,
    pub check: BoxCheck,
    pub to_add: usize,
}

impl BoxCompared {
    pub fn of(exact: &PerspectiveBox, copy: &DrawnCopy) -> Arc<dyn Compared> {
        Self::adding(exact, EDGES_TO_ADD, copy)
    }

    /// A box drawn through, its hidden edges too.
    pub fn through(exact: &PerspectiveBox, copy: &DrawnCopy) -> Arc<dyn Compared> {
        Self::adding(exact, THROUGH_TO_ADD, copy)
    }

    fn adding(exact: &PerspectiveBox, to_add: usize, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let check = BoxCheck::of(exact, &edges_of(copy));
        Arc::new(Self {
            exact: *exact,
            check,
            to_add,
        })
    }
}

impl Compared for BoxCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let exact = &self.exact;
        let mut marks = Vec::new();
        if seen.construction {
            marks.push(run_on(exact.horizon));
            for point in exact.vanishing.iter().flatten() {
                let lines =
                    (exact.corners.iter()).map(|c| segment(In::Copy, [*c, *point], LinePen::Faint));
                marks.extend(lines);
            }
        }
        for ((a, b, _), hidden) in exact.edges().into_iter().zip(exact.hidden()) {
            marks.push(if hidden {
                SheetMark::dashed(In::Copy, vec![a, b], LinePen::Answer)
            } else {
                segment(In::Copy, [a, b], LinePen::Answer)
            });
        }
        let rings = exact.vanishing.iter().flatten();
        marks.extend(rings.map(|p| SheetMark::cut(In::Copy, Figure::Ring(*p))));
        let off: Vec<_> = (self.check.edges.iter())
            .filter_map(|edge| edge.off)
            .collect();
        marks.extend(off_marks(&off, Clip::Sheet));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        if self.check.edges.is_empty() {
            return vec![CardItem::Says(Say::plain("no-edges"))];
        }
        let counts = counted(&self.check.edges, 0);
        vec![
            CardItem::Says(Say::counting("off", &counts)),
            CardItem::Construction,
        ]
    }

    fn verdict(&self) -> Verdict {
        self.check.verdict(&self.exact, self.to_add)
    }
}
