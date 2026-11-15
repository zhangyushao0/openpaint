//! Free boxes' marks (S84, S97, S98, S114, S142): the horizon in their one frame, and on a guided
//! three-point page two faint guides leading to its third point. Compared, where each box's
//! directions meet ringed and a three-point page's third point, a vertical that leans its true
//! vertical (an upright that misses the third point its true upright), a front corner under 90
//! degrees an arc; Construction runs each edge on to where its direction meets, faint, and the
//! horizon across the sheet. A direction's edges missing one point are no fault drawn or counted:
//! the eye judges a free page (S114, the author 2026-11-07). The card: the verticals that lean and
//! the narrow corners, or that no edge was found.

use std::sync::Arc;

use crate::geometry::heading::Checked;
use crate::geometry::perspective::Direction;
use crate::kinds::DrawnCopy;
use crate::kinds::free_boxes::FreePage;
use crate::kinds::free_boxes::perspective_free::FreeCheck;
use crate::kinds::free_boxes::third::{Uprights, guides};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{counted, edges_of, off_marks, run_on, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// The horizon in the one frame, and a guided page's guides, faint.
pub(crate) fn givens(page: &FreePage) -> Vec<SheetMark> {
    let horizon = page.0.horizon;
    let mut marks = vec![SheetMark::cut(In::Copy, Figure::Horizon(horizon))];
    if let Uprights::Given(third) = page.uprights() {
        let guided = guides(horizon, third).map(|guide| segment(In::Copy, guide, LinePen::Faint));
        marks.extend(guided);
    }
    marks
}

/// A page of free boxes compared.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeCompared {
    pub horizon: f64,
    pub check: FreeCheck,
}

impl FreeCompared {
    pub fn of(page: &FreePage, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let horizon = page.0.horizon;
        let check = FreeCheck::on_page(horizon, page.uprights(), &edges_of(copy));
        Arc::new(Self { horizon, check })
    }
}

impl Compared for FreeCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            marks.push(run_on(self.horizon));
            for free in &self.check.boxes {
                for edge in &free.edges {
                    let meet = match edge.direction {
                        Direction::Vertical => self.check.third,
                        direction => (free.meets.iter())
                            .find(|(d, _)| *d == direction)
                            .map(|(_, meet)| *meet),
                    };
                    if let Some(meet) = meet {
                        let [a, b] = edge.edge.ends;
                        let near = if apart(a, meet) < apart(b, meet) {
                            a
                        } else {
                            b
                        };
                        marks.push(segment(In::Copy, [near, meet], LinePen::Faint));
                    }
                }
            }
        }
        let third = self.check.third.map(Figure::Ring);
        marks.extend(third.map(|ring| SheetMark::cut(In::Copy, ring)));
        for free in &self.check.boxes {
            let rings = free.meets.iter().map(|(_, meet)| Figure::Ring(*meet));
            marks.extend(rings.map(|ring| SheetMark::cut(In::Copy, ring)));
            // Only a leaning upright: a receding edge's miss is the eye's to see (S114).
            let leaning = free.edges.iter().filter(|edge| edge.leans());
            let off: Vec<_> = leaning.filter_map(|edge| edge.off).collect();
            marks.extend(off_marks(&off, Clip::Sheet));
            let narrow = free.narrow.iter().map(|corner| Figure::Narrow {
                at: corner.at,
                legs: corner.legs,
            });
            marks.extend(narrow.map(|arc| SheetMark::cut(In::Copy, arc)));
        }
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        if self.check.boxes.is_empty() {
            return vec![CardItem::Says(Say::plain("no-edges"))];
        }
        let edges: Vec<Checked> = (self.check.boxes.iter())
            .flat_map(|free| free.edges.iter().copied())
            .collect();
        let [_, lean, narrow] = counted(&edges, self.check.narrow());
        let counts = [0, lean, narrow];
        vec![
            CardItem::Says(Say::counting("off", &counts)),
            CardItem::Construction,
        ]
    }

    /// Free boxes have no answer but the rules: the eye judges (S114).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

fn apart(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
