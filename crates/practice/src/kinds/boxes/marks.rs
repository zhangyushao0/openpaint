//! A box's marks (S84, S97, S98, S145): in its one frame the horizon, labelled, and its front
//! corner's three edges, printed grey. Compared, over the copy, the exact box in see-through red
//! (its hidden edges dashed) and its vanishing points ringed: the answer's own, never a reading of
//! the copy. Construction lays every corner's line to its vanishing points and the horizon run on
//! across the sheet, faint. The learner's eye judges (D123): the copy's strokes are never read,
//! and the card asks Accurate, Almost or Again after Compare.

use crate::geometry::perspective::PerspectiveBox;
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{run_on, segment};
use crate::marks::mark::{Figure, In, LinePen, SheetMark};

/// The horizon and the front corner's edges, printed grey in the one frame.
pub(crate) fn givens(exact: &PerspectiveBox) -> Vec<SheetMark> {
    let horizon = SheetMark::cut(In::Copy, Figure::Horizon(exact.horizon));
    let given =
        (exact.given().into_iter()).map(|(a, b, _)| segment(In::Copy, [a, b], LinePen::Printed));
    std::iter::once(horizon).chain(given).collect()
}

/// A box compared: the exact box laid over the copy for the eye.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxCompared {
    pub exact: PerspectiveBox,
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
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        vec![CardItem::Construction]
    }

    /// The eye judges the copy against the box laid over it (D123).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}
