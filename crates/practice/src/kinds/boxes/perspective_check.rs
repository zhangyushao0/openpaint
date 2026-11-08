//! Compare for boxes in perspective (S84, S97): geometry made visible, never a score. Every
//! exercise here has an exact answer or an exact rule; the exact box lies over the copy, and only
//! what is off is marked (S97, the author: every stroke drawn on dashed was clutter): an edge whose
//! line misses its vanishing point is drawn on past where it comes nearest the point
//! (`geometry/heading.rs`), so by how much it misses shows beside the point.
//!
//! A box to complete (here): each edge found is taken for the direction whose vanishing point its
//! line passes nearest (in angle); one that misses it is extended from its end nearer the point to
//! `PAST` beyond where its line passes nearest it (a three-point box's verticals: the third
//! point). A two-point vertical has no point: a leaning one gets the true vertical through it.
//! Free boxes, with no answer but the rules: `free_boxes/perspective_free.rs`.

use crate::geometry::edges::Edge;
use crate::geometry::heading::{Checked, checked, line_angle, reach, sub};
use crate::geometry::perspective::{Direction, PerspectiveBox};
use crate::maths::path::Point;

/// An edge is taken for a box's direction only within this angle of it (degrees).
const TAKEN: f64 = 20.0;

/// A completed box as checked against the exact one.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxCheck {
    pub edges: Vec<Checked>,
}

impl BoxCheck {
    /// The edges `found` in the copy checked against `exact`'s vanishing points; an edge near no
    /// direction of the box (`TAKEN`) is left out.
    #[must_use]
    pub fn of(exact: &PerspectiveBox, found: &[Edge]) -> Self {
        let points: Vec<Point> = exact.vanishing.iter().flatten().copied().collect();
        let reach = reach(&points);
        let edges = (found.iter())
            .filter_map(|edge| against(exact, edge, reach))
            .collect();
        Self { edges }
    }

    /// How many edges miss their vanishing point (or lean, a two-point vertical).
    #[must_use]
    pub fn missed(&self) -> usize {
        self.edges.iter().filter(|edge| !edge.on).count()
    }
}

/// `edge` taken for the direction of `exact` its line lies nearest, a miss extended within `reach`
/// of the frame's middle.
fn against(exact: &PerspectiveBox, edge: &Edge, reach: f64) -> Option<Checked> {
    let middle = edge.middle();
    let off = |d: Direction| match exact.vanishing[d.index()] {
        Some(p) => line_angle(edge.direction(), sub(p, middle)),
        None => line_angle(edge.direction(), [0.0, 1.0]),
    };
    let (direction, angle) = (Direction::ALL.iter())
        .map(|d| (*d, off(*d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    if angle > TAKEN {
        return None;
    }
    Some(checked(
        edge,
        direction,
        exact.vanishing[direction.index()],
        reach,
    ))
}
