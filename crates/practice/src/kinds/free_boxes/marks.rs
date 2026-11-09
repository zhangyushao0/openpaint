//! Free boxes' marks (S84, S97, S98): the horizon in their one frame. Compared, where each box's
//! directions meet ringed red, what of its edges is off, a front corner under 90 degrees an arc;
//! Construction runs each receding edge on to where its direction meets the horizon and the
//! horizon across the sheet, faint. The card: how many edges miss, or that none was found.

use std::sync::Arc;

use crate::geometry::heading::Checked;
use crate::geometry::perspective::{Direction, PerspectiveBox};
use crate::kinds::DrawnCopy;
use crate::kinds::free_boxes::perspective_free::FreeCheck;
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen, Verdict};
use crate::marks::drawing::{counted, edges_of, off_marks, run_on, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// The horizon in the one frame.
pub(crate) fn givens(page: &PerspectiveBox) -> Vec<SheetMark> {
    vec![SheetMark::cut(In::Copy, Figure::Horizon(page.horizon))]
}

/// A page of free boxes compared.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeCompared {
    pub horizon: f64,
    pub check: FreeCheck,
}

impl FreeCompared {
    pub fn of(page: &PerspectiveBox, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let check = FreeCheck::of(page.horizon, &edges_of(copy));
        Arc::new(Self {
            horizon: page.horizon,
            check,
        })
    }
}

impl Compared for FreeCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let mut marks = Vec::new();
        if seen.construction {
            marks.push(run_on(self.horizon));
            for free in &self.check.boxes {
                let receding = (free.edges.iter()).filter(|e| e.direction != Direction::Vertical);
                for edge in receding {
                    let meet = free.meets.iter().find(|(d, _)| *d == edge.direction);
                    if let Some((_, meet)) = meet {
                        let [a, b] = edge.edge.ends;
                        let near = if apart(a, *meet) < apart(b, *meet) {
                            a
                        } else {
                            b
                        };
                        marks.push(segment(In::Copy, [near, *meet], LinePen::Faint));
                    }
                }
            }
        }
        for free in &self.check.boxes {
            let rings = free.meets.iter().map(|(_, meet)| Figure::Ring(*meet));
            marks.extend(rings.map(|ring| SheetMark::cut(In::Copy, ring)));
            let off: Vec<_> = (free.edges.iter()).filter_map(|edge| edge.off).collect();
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
        let counts = counted(&edges, self.check.narrow());
        vec![
            CardItem::Says(Say::counting("off", &counts)),
            CardItem::Construction,
        ]
    }

    /// Free boxes have no answer but the rules: the eye judges (S122 takes their red verdict).
    fn verdict(&self) -> Verdict {
        Verdict::Eye
    }
}

fn apart(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
